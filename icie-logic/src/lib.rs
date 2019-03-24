extern crate backtrace;
extern crate ci;
extern crate dirs;
extern crate failure;
extern crate open;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate unijudge;

mod config;
#[macro_use]
mod error;
mod handle;
mod impulse_ui;
mod manifest;
mod paste_lib;
mod progress;
mod status;
pub mod testview;
mod util;
pub mod vscode;

pub use self::handle::Handle;
use self::{error::R, status::Status, vscode::*};
use crate::{
	config::Config, paste_lib::{Library, Piece}
};
use ci::testing::Execution;
pub use ci::testing::Outcome;
use failure::ResultExt;
use rand::prelude::SliceRandom;
use std::{
	env, ffi::OsStr, fs, io, path::{Path, PathBuf}, process::Stdio, str::from_utf8, sync::{
		atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, Sender}, Arc, Mutex
	}, thread, time::Duration
};

#[derive(Debug)]
pub enum Impulse {
	TriggerBuild,
	TriggerTest,
	TriggerInit,
	TriggerSubmit,
	TriggerTemplateInstantiate,
	TriggerManualSubmit,
	TriggerTestview,
	TriggerMultitestView,
	TriggerRR {
		in_path: PathBuf,
	},
	TriggerPastePick,
	NewTest {
		input: String,
		desired: String,
	},
	WorkspaceInfo {
		root_path: Option<String>,
	},
	QuickPick {
		response: Option<String>,
	},
	InputBox {
		response: Option<String>,
	},
	SavedAll,
	MessageResponse {
		id: String,
		response: Option<String>,
	},
	CiTestList {
		paths: Vec<PathBuf>,
	},
	CiTestSingle {
		outcome: ci::testing::Outcome,
		timing: Option<Duration>,
		in_path: PathBuf,
		output: Option<String>,
	},
	CiTestFinish {
		success: bool,
	},
	CiAuthRequest {
		domain: String,
		channel: Sender<Option<(String, String)>>,
	},
	CiInitFinish,
	CiSubmitSuccess {
		id: String,
	},
	CiTrack {
		verdict: unijudge::Verdict,
		finish: bool,
	},
	WorkerError {
		error: failure::Error,
	},
	DiscoveryStart,
	DiscoveryPause,
	DiscoveryReset,
	DiscoverySave {
		input: String,
	},
	CiMultitestRow {
		number: i64,
		input: String,
		brut_measure: Execution,
		measures: Vec<Execution>,
		fitness: i64,
	},
	CiMultitestFinish,
	DocumentText {
		contents: String,
	},
	AcknowledgeEdit,
}
#[derive(Debug)]
pub enum Reaction {
	Status {
		message: Option<String>,
	},
	Message {
		message: String,
		kind: vscode::MessageKind,
		items: Option<vscode::MessageItems>,
		modal: Option<bool>,
	},
	QuickPick {
		items: Vec<QuickPickItem>,
	},
	InputBox {
		options: InputBoxOptions,
	},
	ConsoleLog {
		message: String,
	},
	SaveAll,
	OpenFolder {
		path: PathBuf,
		in_new_window: bool,
	},
	ConsoleError {
		message: String,
	},
	OpenEditor {
		path: PathBuf,
		row: i64,
		column: i64,
	},
	ProgressStart {
		id: String,
		title: Option<String>,
	},
	ProgressUpdate {
		id: String,
		increment: Option<f64>,
		message: Option<String>,
	},
	ProgressEnd {
		id: String,
	},
	TestviewFocus,
	TestviewUpdate {
		tree: testview::Tree,
	},
	MultitestViewFocus,
	DiscoveryRow {
		number: i64,
		outcome: Outcome,
		fitness: i64,
		input: Option<String>,
	},
	DiscoveryState {
		running: bool,
		reset: bool,
	},
	QueryDocumentText {
		path: PathBuf,
	},
	EditPaste {
		position: vscode::Position,
		text: String,
		path: PathBuf,
	},
}

struct ICIE {
	input: Receiver<Impulse>,
	output: Sender<Reaction>,
	input_sender: Sender<Impulse>,

	config: Config,
	directory: Directory,
	id_factory: Mutex<i64>,
	status_stack: Mutex<status::StatusStack>,
}
impl ICIE {
	fn main_loop(&mut self) {
		loop {
			match self.process() {
				Ok(()) => (),
				Err(err) => {
					let details_info = match error::save_details(&err) {
						Ok(path) => format!("error details have been saved in {}", path.display()),
						Err(err2) => format!("failed to save error details ({})", err2),
					};
					self.error(format!("{}, {}", err, details_info));
				},
			}
		}
	}

	fn process(&mut self) -> R<()> {
		match self.recv() {
			Impulse::WorkspaceInfo { root_path } => self.setup_workspace(root_path)?,
			Impulse::TriggerBuild => self.build()?,
			Impulse::TriggerTest => self.test()?,
			Impulse::TriggerInit => self.init()?,
			Impulse::TriggerSubmit => self.submit()?,
			Impulse::TriggerTemplateInstantiate => self.template_instantiate()?,
			Impulse::TriggerManualSubmit => self.manual_submit()?,
			Impulse::TriggerTestview => self.trigger_test_view()?,
			Impulse::TriggerMultitestView => self.trigger_multitest_view()?,
			Impulse::TriggerRR { in_path } => self.rr(in_path)?,
			Impulse::NewTest { input, desired } => self.new_test(input, desired)?,
			Impulse::DiscoveryStart => self.discovery()?,
			Impulse::TriggerPastePick => self.paste_pick()?,
			impulse => Err(error::unexpected(impulse, "trigger").err())?,
		}
		Ok(())
	}

	fn build(&self) -> R<()> {
		let _status = self.status("Compiling");
		self.assure_compiled()?;
		self.info("Compilation successful!");
		Ok(())
	}

	fn test(&self) -> R<()> {
		let _status = self.status("Testing");
		self.assure_passes_tests()?;
		self.info("Tests run successfully");
		Ok(())
	}

	fn init(&self) -> R<()> {
		let _status = self.status("Creating project");
		let name = self.random_codename()?;
		let root = dirs::home_dir()
			.ok_or(error::Category::DegenerateEnvironment { detail: "no home directory" }.err())?
			.join(&name);
		let new_dir = Directory::new(root.clone());
		let url = match self.input_box(InputBoxOptions {
			ignore_focus_out: true,
			password: false,
			placeholder: Some("https://codeforces.com/contest/960/problem/D".to_owned()),
			prompt: Some("Enter task URL".to_owned()),
		})? {
			Some(url) => url,
			None => {
				self.info("ICIE Init cancelled");
				return Ok(());
			},
		};
		let mut ui = self.make_ui();
		let dir = util::TransactionDir::new(&root)?;

		let root2 = root.clone();
		let url2 = url.clone();
		let t1 = self.worker(move || ci::commands::init::run(&url2, &root2, &mut ui));
		loop {
			match self.recv() {
				Impulse::CiAuthRequest { domain, channel } => self.respond_auth(domain, channel)?,
				Impulse::CiInitFinish => break,
				Impulse::WorkerError { error } => return Err(error)?,
				impulse => Err(error::unexpected(impulse, "ci init event").err())?,
			}
		}
		t1.join().map_err(|_| error::Category::ThreadPanicked.err())?;

		fs::copy(&self.config.template_main()?.path, &root.join("main.cpp"))?;
		manifest::Manifest { task_url: url }.save(&new_dir.get_manifest()?)?;
		dir.commit();
		self.send(Reaction::OpenFolder { path: root, in_new_window: false });
		Ok(())
	}

	fn submit(&self) -> R<()> {
		let _status = self.status("Submitting");
		self.assure_passes_tests()?;
		let code = self.directory.get_source()?;
		let mut ui = self.make_ui();
		let manifest = manifest::Manifest::load(&self.directory.get_manifest()?)?;
		let task_url2 = manifest.task_url.clone();
		let t1 = self.worker(move || ci::commands::submit::run(&task_url2, &code, &mut ui));
		let id = loop {
			match self.recv() {
				Impulse::CiAuthRequest { domain, channel } => self.respond_auth(domain, channel)?,
				Impulse::CiSubmitSuccess { id } => break id,
				Impulse::WorkerError { error } => return Err(error)?,
				impulse => Err(error::unexpected(impulse, "ci submit event").err())?,
			}
		};
		t1.join().map_err(|_| error::Category::ThreadPanicked.err())?;
		self.track_submit(id, manifest.task_url)?;
		Ok(())
	}

	fn template_instantiate(&self) -> R<()> {
		let _status = self.status("Creating template");
		let items = self
			.config
			.templates
			.iter()
			.map(|tpl| QuickPickItem {
				id: tpl.id.clone(),
				label: tpl.name.clone(),
				description: None,
				detail: Some(tpl.path.display().to_string()),
			})
			.collect::<Vec<_>>();
		self.send(Reaction::QuickPick { items });
		let template_id = loop {
			match self.recv() {
				Impulse::QuickPick { response: Some(template_id) } => break template_id,
				Impulse::QuickPick { response: None } => return Ok(()),
				impulse => Err(error::unexpected(impulse, "template quick pick").err())?,
			}
		};
		let template = self
			.config
			.templates
			.iter()
			.find(|tpl| tpl.id == template_id)
			.ok_or_else(|| error::Category::TemplateDoesNotExist { id: template_id }.err())?;
		self.send(Reaction::InputBox {
			options: InputBoxOptions {
				ignore_focus_out: true,
				password: false,
				placeholder: Some(template.default_filename.clone()),
				prompt: Some("New file name".to_string()),
			},
		});
		let filename = loop {
			match self.recv() {
				Impulse::InputBox { response: Some(filename) } => break filename,
				Impulse::InputBox { response: None } => return Ok(()),
				impulse => Err(error::unexpected(impulse, "template name input box").err())?,
			}
		};
		let path = self.directory.need_root()?.join(filename);
		if path.exists() && !from_utf8(&fs::read(&path)?)?.trim().is_empty() {
			return Err(error::Category::FileAlreadyExists { path }.err())?;
		}
		fs::copy(&template.path, &path)?;
		self.send(Reaction::OpenEditor {
			path: path.clone(),
			row: template.cursor.row,
			column: template.cursor.column,
		});
		Ok(())
	}

	fn manual_submit(&self) -> R<()> {
		let _status = self.status("Submitting manually");
		self.assure_passes_tests()?;
		let manifest = manifest::Manifest::load(&self.directory.get_manifest()?)?;
		let tu = unijudge::TaskUrl::deconstruct(&manifest.task_url)?;
		let mut ui = self.make_ui();
		let session = ci::connect(&manifest.task_url, &mut ui)?;
		let contest = session.contest(&tu.contest);
		let url = contest.manual_submit_url(&tu.task)?;
		open::that(url)?;
		Ok(())
	}

	fn trigger_test_view(&self) -> R<()> {
		self.collect_tests()?;
		self.send(Reaction::TestviewFocus);
		Ok(())
	}

	fn trigger_multitest_view(&self) -> R<()> {
		self.send(Reaction::MultitestViewFocus);
		Ok(())
	}

	fn rr(&self, in_path: PathBuf) -> R<()> {
		self.log("wtf?");
		let _status = self.status("Recording");
		self.assure_compiled()?;
		let rec_out = util::try_commands(
			&[("rr", &["record", util::path_to_str(&self.directory.get_executable()?)?])],
			"sudo apt install rr",
			|cmd| {
				cmd.stdin(std::fs::File::open(util::path_to_str(&in_path)?)?);
				cmd.stdout(Stdio::piped());
				cmd.stderr(Stdio::piped());
				Ok(())
			},
		)?
		.wait_with_output()?;

		if from_utf8(&rec_out.stderr)?.contains("/proc/sys/kernel/perf_event_paranoid") {
			return Err(error::Category::PerfEventParanoid.err())?;
		}
		util::try_commands(
			&[
				("x-terminal-emulator", &["-e", "bash -c \"rr replay -- -q ; bash\""]),
				("i3-sensible-terminal", &["-e", "bash -c \"rr replay -- -q ; bash\""]),
				("xfce4-terminal", &["-e", "bash -c \"rr replay -- -q ; bash\""]),
			],
			"sudo apt install xfce4-terminal",
			|_| Ok(()),
		)?;
		Ok(())
	}

	fn new_test(&self, input: String, desired: String) -> R<()> {
		let dir = self.directory.get_custom_tests()?;
		ci::util::demand_dir(&dir)?;
		let used = fs::read_dir(&dir)?
			.into_iter()
			.map(|der| {
				der.ok()
					.and_then(|de| de.path().file_stem().map(OsStr::to_owned))
					.and_then(|stem| stem.to_str().map(str::to_owned))
					.and_then(|name| name.parse::<i64>().ok())
			})
			.filter_map(|o| o)
			.collect::<Vec<_>>();
		let id = util::mex(1, used);
		fs::write(dir.join(format!("{}.in", id)), input)?;
		fs::write(dir.join(format!("{}.out", id)), desired)?;
		self.update_test_view(&self.collect_tests()?)?;
		self.send(Reaction::TestviewFocus);
		Ok(())
	}

	fn discovery(&self) -> R<()> {
		let _status = self.status("Discovering");
		self.send(Reaction::DiscoveryState { running: true, reset: true });
		let gen = self.get_generator()?;
		let brut = self.get_brut()?;
		let brut2 = brut.clone();
		let solution = self.get_solution()?;
		let executables = [brut, solution];
		let checker = self.get_checker()?;
		let count = Some(std::i64::MAX);
		let fitness = ci::fitness::Bytelen;
		let time_limit = None;
		let ignore_generator_fail = false;
		let end_variable = Arc::new(AtomicBool::new(false));
		let end_variable2 = end_variable.clone();
		let impulse_ui::PausableUi { mut ui, pause } = self.make_pausable_ui();
		let t1 = self.worker(move || ci::commands::multitest::run(&gen, &executables, &*checker, count, &fitness, time_limit, ignore_generator_fail, end_variable, &mut ui));
		let mut best_fitness = None;
		let mut paused = false;
		let mut found_test = None;
		loop {
			match self.recv() {
				Impulse::DiscoveryStart if paused => {
					paused = false;
					pause.send(())?;
					self.send(Reaction::DiscoveryState { running: true, reset: false });
				},
				Impulse::DiscoveryPause if !paused => {
					paused = true;
					pause.send(())?;
					self.send(Reaction::DiscoveryState { running: false, reset: false });
				},
				Impulse::DiscoveryReset => {
					self.send(Reaction::DiscoveryState { running: false, reset: true });
					end_variable2.store(true, Ordering::SeqCst);
					if paused {
						pause.send(()).unwrap();
						paused = false;
					}
				},
				Impulse::DiscoverySave { input } => {
					self.send(Reaction::DiscoveryState { running: false, reset: false });
					end_variable2.store(true, Ordering::SeqCst);
					if paused {
						pause.send(()).unwrap();
						paused = false;
					}
					found_test = Some(input);
				},
				Impulse::CiMultitestRow {
					number,
					input,
					brut_measure,
					measures,
					fitness,
				} => {
					let is_failed = brut_measure.outcome != Outcome::Accept || measures[0].outcome != Outcome::Accept;
					let new_best = is_failed && best_fitness.map(|bf| bf < fitness).unwrap_or(true);
					if new_best {
						best_fitness = Some(fitness);
					}
					self.send(Reaction::DiscoveryRow {
						number,
						outcome: measures[0].outcome.clone(),
						fitness,
						input: if new_best { Some(input) } else { None },
					});
				},
				Impulse::CiMultitestFinish => break,
				impulse => Err(error::unexpected(impulse, "ci discovery").err())?,
			}
		}
		t1.join().unwrap();
		self.send(Reaction::DiscoveryState { running: false, reset: false });
		if let Some(input) = found_test {
			let exec = ci::testing::run(&brut2, ci::strres::StrRes::InMemory(input.clone()), None)?;
			let desired = exec.out.get_string()?;
			self.new_test(input, desired)?;
		}
		Ok(())
	}

	fn paste_pick(&self) -> R<()> {
		let _status = self.status("Copy-pasting code");
		let library = paste_lib::Library::load(&self.config.library_path()?)?;
		self.send(Reaction::QuickPick {
			items: library
				.pieces
				.iter()
				.map(|(id, piece)| vscode::QuickPickItem {
					label: piece.name.clone(),
					description: piece.description.as_ref().map(|s| s.clone()),
					detail: piece.detail.as_ref().map(|s| s.clone()),
					id: id.clone(),
				})
				.collect(),
		});
		let piece_id = loop {
			match self.recv() {
				Impulse::QuickPick { response: Some(piece_id) } => break piece_id,
				Impulse::QuickPick { response: None } => return Ok(()),
				impulse => return Err(error::unexpected(impulse, "quick pick response").err())?,
			}
		};
		let piece = library.pieces.iter().find(|piece| *piece.0 == piece_id).unwrap().1;
		let mut text = self.query_document_text(self.directory.get_source()?)?;
		self.paste_piece(piece, true, &mut text, &library)?;
		Ok(())
	}

	fn paste_piece(&self, piece: &Piece, top_level: bool, text: &mut String, library: &Library) -> R<()> {
		if text.contains(&piece.guarantee) {
			return Ok(());
		}
		for dep in &piece.dependencies {
			self.paste_piece(&library.pieces[dep], false, text, library)?;
		}
		self.log(format!("Wanna paste: {}", piece.code));
		let (position, snippet) = library.place(piece, text)?;
		self.send(Reaction::EditPaste {
			position,
			text: snippet,
			path: self.directory.get_source()?,
		});
		match self.recv() {
			Impulse::AcknowledgeEdit => (),
			impulse => Err(error::unexpected(impulse, "acknowledgment of edit").err())?,
		}
		if !top_level {
			*text = self.query_document_text(self.directory.get_source()?)?;
		}
		Ok(())
	}

	fn query_document_text(&self, path: PathBuf) -> R<String> {
		self.send(Reaction::QueryDocumentText { path });
		match self.recv() {
			Impulse::DocumentText { contents } => Ok(contents),
			impulse => return Err(error::unexpected(impulse, "document text query response").err())?,
		}
	}

	fn setup_workspace(&mut self, root_path: Option<String>) -> R<()> {
		self.directory.set_root_path(root_path.map(PathBuf::from));
		self.launch()
	}

	fn launch(&self) -> R<()> {
		let _status = self.status("Launching");
		if self.directory.is_open() && self.directory.get_source()?.exists() {
			self.send(Reaction::OpenEditor {
				path: self.directory.get_source()?,
				row: self.config.template_main()?.cursor.row,
				column: self.config.template_main()?.cursor.column,
			});
		}
		Ok(())
	}

	fn track_submit(&self, id: String, url: String) -> R<()> {
		let _status = self.status("Tracking");
		let title = format!("Tracking submit #{}", id);
		let mut ui = self.make_ui();
		let sleep_duration = Duration::from_millis(500);
		let t1 = self.worker(move || ci::commands::tracksubmit::run(&url, id, sleep_duration, &mut ui));
		let progress = self.progress_start(Some(&title))?;
		let mut last_verdict = None;
		let verdict = loop {
			match self.recv() {
				Impulse::CiTrack { verdict, finish } => {
					if Some(&verdict) != last_verdict.as_ref() {
						progress.update(None, Some(&ci::ui::human::fmt_verdict(&verdict)))?;
						last_verdict = Some(verdict.clone());
					}
					if finish {
						break verdict;
					}
				},
				Impulse::CiAuthRequest { domain, channel } => self.respond_auth(domain, channel)?,
				Impulse::WorkerError { error } => return Err(error)?,
				impulse => Err(error::unexpected(impulse, "ci track event").err())?,
			}
		};
		progress.end();
		t1.join().unwrap();
		self.info(ci::ui::human::fmt_verdict(&verdict));
		Ok(())
	}

	fn respond_auth(&self, domain: String, channel: Sender<Option<(String, String)>>) -> R<()> {
		let username = self
			.input_box(InputBoxOptions {
				prompt: Some(format!("Username at {}", domain)),
				placeholder: None,
				ignore_focus_out: true,
				password: false,
			})?
			.ok_or_else(|| error::Category::LackOfInput.err())?;
		let password = self
			.input_box(InputBoxOptions {
				prompt: Some(format!("Password for {} at {}", username, domain)),
				placeholder: None,
				ignore_focus_out: true,
				password: true,
			})?
			.ok_or_else(|| error::Category::LackOfInput.err())?;
		channel.send(Some((username, password))).context("thread suddenly stopped")?;
		Ok(())
	}

	fn random_codename(&self) -> R<String> {
		let mut rng = rand::thread_rng();
		static ADJECTIVES: &[&str] = &[
			"playful",
			"shining",
			"sparkling",
			"rainbow",
			"kawaii",
			"superb",
			"amazing",
			"glowing",
			"blessed",
			"smiling",
			"exquisite",
			"cuddly",
			"caramel",
			"serene",
			"sublime",
			"beaming",
			"graceful",
			"plushy",
			"heavenly",
			"marshmallow",
		];
		static ANIMALS: &[&str] = &[
			"capybara", "squirrel", "spider", "anteater", "hamster", "whale", "eagle", "zebra", "dolphin", "hedgehog", "penguin", "wombat", "ladybug", "platypus", "squid",
			"koala", "panda",
		];
		Ok(format!(
			"{}-{}",
			ADJECTIVES.choose(&mut rng).ok_or_else(|| error::Category::NoCuteAnimals.err())?,
			ANIMALS.choose(&mut rng).ok_or_else(|| error::Category::NoCuteAnimals.err())?
		))
	}

	fn run_tests(&self) -> R<(bool, Option<(ci::testing::Outcome, PathBuf)>)> {
		let tests = self.collect_tests()?;
		let first_failed = tests
			.into_iter()
			.find(|test| test.outcome != ci::testing::Outcome::Accept)
			.map(|test| (test.outcome, test.in_path));
		let good = first_failed.is_none();
		Ok((good, first_failed))
	}

	fn collect_tests(&self) -> R<Vec<Test>> {
		let _status = self.status("Testing");
		self.assure_compiled()?;
		let progress = self.progress_start(Some("Testing"))?;
		let executable = self.directory.get_executable()?;
		let testdir = self.directory.get_tests()?;
		let checker = self.get_checker()?;
		let mut ui = self.make_ui();
		let t1 = self.worker(move || ci::commands::test::run(&executable, &testdir, &*checker, false, true, &mut ui));
		let mut test_count = None;
		let mut tests = Vec::new();
		loop {
			match self.recv() {
				Impulse::CiTestList { paths } => test_count = Some(paths.len()),
				Impulse::CiTestSingle { outcome, timing, in_path, output } => {
					progress.update(test_count.map(|total| 100.0 / total as f64), Some(&format!("Ran {}", in_path.display())))?;
					tests.push(Test { in_path, outcome, timing, output });
				},
				Impulse::CiTestFinish { .. } => break,
				Impulse::WorkerError { error } => return Err(error)?,
				impulse => Err(error::unexpected(impulse, "ci test event").err())?,
			}
		}
		t1.join().map_err(|_| error::Category::ThreadPanicked.err())?;
		progress.end();
		self.update_test_view(&tests)?;
		Ok(tests)
	}

	fn assure_passes_tests(&self) -> R<()> {
		let (_, first_fail) = self.run_tests()?;
		if let Some((verdict, path)) = first_fail {
			self.send(Reaction::TestviewFocus);
			Err(error::Category::TestFailure { verdict, path }.err())?
		} else {
			Ok(())
		}
	}

	fn get_checker(&self) -> R<Box<ci::checkers::Checker+Send>> {
		match self.directory.get_checker_source()? {
			Some(source) => {
				self.assure_compiled_path(&source)?;
				Ok(Box::new(ci::checkers::CheckerApp::new(util::path_to_str(&self.directory.get_checker()?)?.to_owned())?))
			},
			None => Ok(Box::new(ci::checkers::CheckerDiffOut)),
		}
	}

	fn get_brut(&self) -> R<PathBuf> {
		let source = self.directory.get_brut_source()?;
		self.assure_compiled_path(&source)?;
		let exec = self.directory.get_brut()?;
		Ok(exec)
	}

	fn get_generator(&self) -> R<PathBuf> {
		let source = self.directory.get_gen_source()?;
		self.assure_compiled_path(&source)?;
		let exec = self.directory.get_gen()?;
		Ok(exec)
	}

	fn get_solution(&self) -> R<PathBuf> {
		let source = self.directory.get_source()?;
		self.assure_compiled_path(&source)?;
		let exec = self.directory.get_executable()?;
		Ok(exec)
	}

	fn assure_compiled(&self) -> R<()> {
		self.assure_compiled_path(&self.directory.get_source()?)
	}

	fn assure_compiled_path(&self, path: &Path) -> R<()> {
		if self.requires_compilation(path)? {
			self.compile(path)?;
		}
		Ok(())
	}

	fn compile(&self, source: &Path) -> R<()> {
		let _status = self.status("Compiling");
		let codegen = ci::commands::build::Codegen::Debug;
		let cppver = ci::commands::build::CppVer::Cpp17;
		let library = self.directory.get_library_source()?;
		let library = library.as_ref().map(|pb| pb.as_path());
		self.log(format!("source = {:?}, codegen = {:?}, cppver = {:?}, library = {:?}", source, codegen, cppver, library));
		match ci::commands::build::run(source, &codegen, &cppver, library, true) {
			Ok(()) => {},
			Err(ci::commands::build::BuildError::Other(e)) => return Err(e)?,
			Err(ci::commands::build::BuildError::CompilationError { mode, messages }) => {
				let messages = messages.expect("compiler messages were not parsed");
				self.log(format!("{:#?}", messages));
				if messages.len() > 0 {
					self.send(Reaction::OpenEditor {
						path: env::current_dir()?.join(&messages[0].file),
						row: messages[0].line,
						column: messages[0].column,
					});
				}
				return Err(error::Category::CompilationError {
					file: source.to_path_buf(),
					message: messages.first().map(|cem| cem.message.to_owned()),
					mode,
				}
				.err())?;
			},
		}
		Ok(())
	}

	fn assure_all_saved(&self) -> R<()> {
		self.send(Reaction::SaveAll);
		match self.recv() {
			Impulse::SavedAll => {},
			impulse => Err(error::unexpected(impulse, "confirmation that all files were saved").err())?,
		}
		Ok(())
	}

	fn requires_compilation(&self, src: &Path) -> R<bool> {
		let exe = src.with_extension("e");
		self.assure_all_saved()?;
		let metasrc = src.metadata().context(format!("source {:?} does not exist", src))?;
		let metaexe = match exe.metadata() {
			Ok(metaexe) => metaexe,
			Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
			e => e?,
		};
		Ok(metasrc.modified()? >= metaexe.modified()?)
	}

	fn update_test_view(&self, tests: &[Test]) -> R<()> {
		let tree = testview::Tree::Directory {
			files: tests
				.into_iter()
				.map(|test| {
					Ok(testview::Tree::Test {
						name: util::without_extension(test.in_path.strip_prefix(self.directory.get_tests()?)?)
							.to_str()
							.ok_or(error::Category::NonUTF8Path.err())?
							.to_owned(),
						input: fs::read_to_string(&test.in_path)?,
						output: test.output.clone().expect("test output not recorded even though it should be"),
						desired: util::read_to_string_if_exists(test.in_path.with_extension("out"))?,
						timing: test.timing,
						in_path: test.in_path.clone(),
						outcome: test.outcome.clone(),
					})
				})
				.collect::<R<Vec<_>>>()?,
		};
		self.send(Reaction::TestviewUpdate { tree });
		Ok(())
	}

	fn make_ui(&self) -> impulse_ui::ImpulseCiUi {
		self.make_pausable_ui().ui
	}

	fn make_pausable_ui(&self) -> impulse_ui::PausableUi {
		let (send, recv) = mpsc::channel();
		impulse_ui::PausableUi {
			ui: impulse_ui::ImpulseCiUi {
				impulse: self.input_sender.clone(),
				pause: recv,
			},
			pause: send,
		}
	}

	fn worker<T: Send+'static, F: FnOnce() -> R<T>+Send+'static>(&self, f: F) -> thread::JoinHandle<Option<T>> {
		let is = self.input_sender.clone();
		thread::spawn(move || match f() {
			Ok(x) => Some(x),
			Err(error) => {
				is.send(Impulse::WorkerError { error }).expect("actor channel destroyed");
				None
			},
		})
	}

	fn gen_id(&self) -> String {
		let mut id_factory = self.id_factory.lock().unwrap();
		let id = id_factory.to_string();
		*id_factory += 1;
		id
	}

	fn progress_start(&self, title: Option<&str>) -> R<progress::Progress> {
		progress::Progress::start(title, &self.gen_id(), &self)
	}

	fn status(&self, msg: impl Into<String>) -> Status {
		let msg: String = msg.into();
		Status::new(&msg, &self)
	}

	fn info(&self, message: impl Into<String>) {
		self.send(Reaction::Message {
			message: message.into(),
			kind: vscode::MessageKind::Info,
			items: None,
			modal: None,
		});
	}

	fn error(&self, message: impl Into<String>) {
		self.send(Reaction::Message {
			message: message.into(),
			kind: vscode::MessageKind::Error,
			items: None,
			modal: None,
		});
	}

	fn log(&self, message: impl Into<String>) {
		self.send(Reaction::ConsoleLog { message: message.into() });
	}

	fn input_box(&self, options: InputBoxOptions) -> R<Option<String>> {
		self.send(Reaction::InputBox { options });
		match self.recv() {
			Impulse::InputBox { response } => Ok(response),
			impulse => Err(error::unexpected(impulse, "input box input").err())?,
		}
	}

	fn recv(&self) -> Impulse {
		self.input.recv().expect("actor channel destroyed")
	}

	fn send(&self, reaction: Reaction) {
		self.output.send(reaction).expect("actor channel destroyed");
	}
}

struct Directory {
	root: Option<PathBuf>,
}
impl Directory {
	pub fn new_empty() -> Directory {
		Directory { root: None }
	}

	pub fn new(root: PathBuf) -> Directory {
		Directory { root: Some(root) }
	}

	fn set_root_path(&mut self, root: Option<PathBuf>) {
		self.root = root;
	}

	pub fn need_root(&self) -> R<&std::path::Path> {
		Ok(self.root.as_ref().map(|pb| pb.as_path()).ok_or_else(|| error::Category::NoOpenFolder.err())?)
	}

	fn get_source(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("main.cpp"))
	}

	fn get_library_source(&self) -> R<Option<PathBuf>> {
		let path = self.need_root()?.join("lib.cpp");
		Ok(if path.exists() { Some(path) } else { None })
	}

	fn get_checker_source(&self) -> R<Option<PathBuf>> {
		let path = self.need_root()?.join("checker.cpp");
		Ok(if path.exists() { Some(path) } else { None })
	}

	fn get_checker(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("checker.e"))
	}

	fn get_gen_source(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("gen.cpp"))
	}

	fn get_gen(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("gen.e"))
	}

	fn get_brut_source(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("brut.cpp"))
	}

	fn get_brut(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("brut.e"))
	}

	fn get_executable(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("main.e"))
	}

	fn get_tests(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("tests"))
	}

	fn get_manifest(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join(".icie"))
	}

	fn get_custom_tests(&self) -> R<PathBuf> {
		Ok(self.get_tests()?.join("user"))
	}

	fn is_open(&self) -> bool {
		self.root.is_some()
	}
}

#[derive(Debug)]
pub struct Test {
	in_path: PathBuf,
	outcome: Outcome,
	timing: Option<Duration>,
	output: Option<String>,
}
