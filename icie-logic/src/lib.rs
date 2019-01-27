extern crate backtrace;
extern crate ci;
extern crate dirs;
#[macro_use]
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
mod progress;
mod status;
mod vscode;

pub use self::handle::Handle;
use self::{error::R, status::Status, vscode::*};
use crate::config::Config;
use failure::ResultExt;
use rand::prelude::SliceRandom;
use std::{
	fs, io, path::PathBuf, str::from_utf8, sync::{
		mpsc::{Receiver, Sender}, Mutex
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
	ProgressReady {
		id: String,
	},
	CiTestSingle {
		outcome: ci::testing::TestResult,
		timing: Option<Duration>,
		in_path: PathBuf,
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
}
pub enum Reaction {
	Status { message: Option<String> },
	InfoMessage { message: String },
	ErrorMessage { message: String },
	QuickPick { items: Vec<QuickPickItem> },
	InputBox { options: InputBoxOptions },
	ConsoleLog { message: String },
	SaveAll,
	OpenFolder { path: PathBuf, in_new_window: bool },
	ConsoleError { message: String },
	OpenEditor { path: PathBuf, row: i64, column: i64 },
	ProgressStart { id: String, title: Option<String> },
	ProgressUpdate { id: String, increment: Option<f64>, message: Option<String> },
	ProgressEnd { id: String },
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
				Err(err) => self.error(format!("{}", err)),
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
			impulse => Err(error::unexpected(impulse, "trigger"))?,
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
		let root = dirs::home_dir().ok_or(error::Category::DegenerateEnvironment { detail: "no home directory" })?.join(&name);
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
		ci::util::demand_dir(&root)?;

		let root2 = root.clone();
		let url2 = url.clone();
		let t1 = thread::spawn(move || {
			let _ = ci::commands::init::run(&url2, &root2, &mut ui);
		});
		loop {
			match self.recv() {
				Impulse::CiAuthRequest { domain, channel } => self.respond_auth(domain, channel)?,
				Impulse::CiInitFinish => break,
				impulse => Err(error::unexpected(impulse, "ci init event"))?,
			}
		}
		t1.join().map_err(|_| error::Category::ThreadPanicked)?;

		fs::copy(&self.config.template_main().path, &root.join("main.cpp"))?;
		manifest::Manifest { task_url: url }.save(&new_dir.get_manifest()?);
		self.send(Reaction::OpenFolder { path: root, in_new_window: false });
		Ok(())
	}

	fn submit(&self) -> R<()> {
		let _status = self.status("Submitting");
		self.assure_passes_tests()?;
		let code = self.directory.get_source()?;
		let mut ui = self.make_ui();
		let manifest = manifest::Manifest::load(&self.directory.get_manifest()?);
		let task_url2 = manifest.task_url.clone();
		let t1 = thread::spawn(move || {
			let _ = ci::commands::submit::run(&task_url2, &code, &mut ui);
		});
		let id = loop {
			match self.recv() {
				Impulse::CiAuthRequest { domain, channel } => self.respond_auth(domain, channel)?,
				Impulse::CiSubmitSuccess { id } => break id,
				impulse => Err(error::unexpected(impulse, "ci submit event"))?,
			}
		};
		t1.join().map_err(|_| error::Category::ThreadPanicked)?;
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
		let response = loop {
			match self.recv() {
				Impulse::QuickPick { response } => break response.unwrap(),
				impulse => Err(error::unexpected(impulse, "template quick pick"))?,
			}
		};
		let template = self.config.templates.iter().find(|tpl| tpl.id == response).unwrap();
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
				Impulse::InputBox { response } => break response.unwrap(),
				impulse => Err(error::unexpected(impulse, "template name input box"))?,
			}
		};
		let path = self.directory.need_root()?.join(filename);
		if path.exists() && !from_utf8(&fs::read(&path)?)?.trim().is_empty() {
			panic!("File already exists and is not empty");
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
		let manifest = manifest::Manifest::load(&self.directory.get_manifest()?);
		let tu = unijudge::TaskUrl::deconstruct(&manifest.task_url);
		let mut ui = self.make_ui();
		let session = ci::connect(&manifest.task_url, &mut ui);
		let contest = session.contest(&tu.contest);
		let url = contest.manual_submit_url(&tu.task);
		open::that(url)?;
		Ok(())
	}

	fn setup_workspace(&mut self, root_path: Option<String>) -> R<()> {
		self.directory.set_root_path(root_path.map(PathBuf::from));
		self.launch()
	}

	fn launch(&self) -> R<()> {
		let _status = self.status("Launching");
		if self.directory.get_source()?.exists() {
			self.send(Reaction::OpenEditor {
				path: self.directory.get_source()?,
				row: self.config.template_main().cursor.row,
				column: self.config.template_main().cursor.column,
			});
		}
		Ok(())
	}

	fn track_submit(&self, id: String, url: String) -> R<()> {
		let _status = self.status("Tracking");
		let title = format!("Tracking submit #{}", id);
		let mut ui = self.make_ui();
		let t1 = thread::spawn(move || {
			let _ = ci::commands::tracksubmit::run(&url, id, Duration::from_millis(500), &mut ui).unwrap();
		});
		let progress = {
			let id = self.gen_id();
			self.progress_start(Some(&title), &id)?
		};
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
				impulse => Err(error::unexpected(impulse, "ci track event"))?,
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
				ignore_focus_out: false,
				password: false,
			})?
			.ok_or(error::Category::LackOfInput)?;
		let password = self
			.input_box(InputBoxOptions {
				prompt: Some(format!("Password for {} at {}", username, domain)),
				placeholder: None,
				ignore_focus_out: false,
				password: true,
			})?
			.ok_or(error::Category::LackOfInput)?;
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
			ADJECTIVES.choose(&mut rng).ok_or(error::Category::NoCuteAnimals)?,
			ANIMALS.choose(&mut rng).ok_or(error::Category::NoCuteAnimals)?
		))
	}

	fn run_tests(&self) -> R<(bool, Option<(ci::testing::TestResult, PathBuf)>)> {
		let _status = self.status("Testing");
		self.assure_compiled()?;
		let executable = self.directory.get_executable()?;
		let testdir = self.directory.get_tests()?;
		let mut ui = self.make_ui();
		let t1 = thread::spawn(move || {
			let _ = ci::commands::test::run(&executable, &testdir, &ci::checkers::CheckerDiffOut, false, false, &mut ui);
		});
		let mut first_failed = None;
		let success = loop {
			match self.recv() {
				Impulse::CiTestSingle { outcome, timing, in_path } => {
					self.log(format!("{:?} {:?} {:?}", outcome, timing, in_path));
					if outcome != ci::testing::TestResult::Accept {
						first_failed = first_failed.or(Some((outcome, in_path)));
					}
				},
				Impulse::CiTestFinish { success } => break success,
				impulse => Err(error::unexpected(impulse, "ci test event"))?,
			}
		};
		t1.join().map_err(|_| error::Category::ThreadPanicked)?;
		Ok((success, first_failed))
	}

	fn assure_passes_tests(&self) -> R<()> {
		let (_, first_fail) = self.run_tests()?;
		if let Some((verdict, path)) = first_fail {
			Err(error::Category::TestFailure { verdict, path })?
		} else {
			Ok(())
		}
	}

	fn assure_compiled(&self) -> R<()> {
		if self.requires_compilation()? {
			self.compile()?;
		}
		Ok(())
	}

	fn compile(&self) -> R<()> {
		let _status = self.status("Compiling");
		let source = self.directory.get_source()?;
		let codegen = ci::commands::build::Codegen::Debug;
		let cppver = ci::commands::build::CppVer::Cpp17;
		let library = self.directory.get_library_source()?;
		let library = library.as_ref().map(|pb| pb.as_path());
		self.log(format!("source = {:?}, codegen = {:?}, cppver = {:?}, library = {:?}", source, codegen, cppver, library));
		ci::commands::build::run(&source, &codegen, &cppver, library)?;
		Ok(())
	}

	fn assure_all_saved(&self) -> R<()> {
		self.send(Reaction::SaveAll);
		match self.recv() {
			Impulse::SavedAll => {},
			impulse => Err(error::unexpected(impulse, "confirmation that all files were saved"))?,
		}
		Ok(())
	}

	fn requires_compilation(&self) -> R<bool> {
		let src = self.directory.get_source()?;
		let exe = self.directory.get_executable()?;
		self.assure_all_saved()?;
		let metasrc = src.metadata()?;
		let metaexe = match exe.metadata() {
			Ok(metaexe) => metaexe,
			Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
			e => e?,
		};
		Ok(metasrc.modified()? >= metaexe.modified()?)
	}

	fn make_ui(&self) -> impulse_ui::ImpulseCiUi {
		impulse_ui::ImpulseCiUi(self.input_sender.clone())
	}

	fn gen_id(&self) -> String {
		let mut id_factory = self.id_factory.lock().unwrap();
		let id = id_factory.to_string();
		*id_factory += 1;
		id
	}

	fn progress_start<'a, 'b, 'c>(&'a self, title: Option<&'b str>, id: &'c str) -> R<progress::Progress<'a>> {
		progress::Progress::start(title, id, &self)
	}

	fn status(&self, msg: impl Into<String>) -> Status {
		let msg: String = msg.into();
		Status::new(&msg, &self)
	}

	fn info(&self, message: impl Into<String>) {
		self.send(Reaction::InfoMessage { message: message.into() });
	}

	fn error(&self, message: impl Into<String>) {
		self.send(Reaction::ErrorMessage { message: message.into() });
	}

	fn log(&self, message: impl Into<String>) {
		self.send(Reaction::ConsoleLog { message: message.into() });
	}

	fn input_box(&self, options: InputBoxOptions) -> R<Option<String>> {
		self.send(Reaction::InputBox { options });
		match self.recv() {
			Impulse::InputBox { response } => Ok(response),
			impulse => Err(error::unexpected(impulse, "input box input"))?,
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
		Ok(self.root.as_ref().map(|pb| pb.as_path()).ok_or(error::Category::NoOpenFolder)?)
	}

	fn get_source(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join("main.cpp"))
	}

	fn get_library_source(&self) -> R<Option<PathBuf>> {
		let path = self.need_root()?.join("lib.cpp");
		Ok(if path.exists() { Some(path) } else { None })
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
}
