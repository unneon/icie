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

#[macro_use]
mod error;
mod handle;
mod impulse_ui;
mod manifest;
mod vscode;

pub use self::handle::Handle;
use self::{error::R, vscode::*};
use failure::ResultExt;
use rand::prelude::SliceRandom;
use std::{
	fs, io, path::PathBuf, sync::mpsc::{Receiver, Sender}, thread, time::Duration
};

#[derive(Debug)]
pub enum Impulse {
	TriggerBuild,
	TriggerTest,
	TriggerInit,
	TriggerSubmit,
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
}

struct ICIE {
	input: Receiver<Impulse>,
	output: Sender<Reaction>,
	input_sender: Sender<Impulse>,

	directory: Directory,
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
			Impulse::WorkspaceInfo { root_path } => self.directory.set_root_path(root_path.map(PathBuf::from)),
			Impulse::TriggerBuild => self.build()?,
			Impulse::TriggerTest => self.test()?,
			Impulse::TriggerInit => self.init()?,
			Impulse::TriggerSubmit => self.submit()?,
			Impulse::TriggerManualSubmit => self.manual_submit()?,
			impulse => Err(error::Category::UnexpectedImpulse {
				description: format!("{:?}", impulse),
			})?,
		}
		Ok(())
	}

	fn build(&mut self) -> R<()> {
		let source = self.directory.get_source()?;
		let codegen = ci::commands::build::Codegen::Debug;
		let cppver = ci::commands::build::CppVer::Cpp17;
		let library = self.directory.get_library_source()?;
		let library = library.as_ref().map(|pb| pb.as_path());
		self.log(format!("source = {:?}, codegen = {:?}, cppver = {:?}, library = {:?}", source, codegen, cppver, library));
		ci::commands::build::run(&source, &codegen, &cppver, library)?;
		self.info("Compilation successful!");
		Ok(())
	}

	fn test(&mut self) -> R<()> {
		self.assure_passes_tests()?;
		self.info("Tests run successfully");
		Ok(())
	}

	fn init(&mut self) -> R<()> {
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
				Impulse::CiAuthRequest { domain, channel } => {
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
				},
				Impulse::CiInitFinish => break,
				impulse => Err(error::Category::UnexpectedImpulse {
					description: format!("{:?}", impulse),
				})?,
			}
		}
		t1.join().map_err(|_| error::Category::ThreadPanicked)?;

		fs::copy(&self.directory.get_template_main()?, &root.join("main.cpp"))?;
		manifest::Manifest { task_url: url }.save(&new_dir.get_manifest()?);
		self.send(Reaction::OpenFolder { path: root, in_new_window: false });
		Ok(())
	}

	fn submit(&mut self) -> R<()> {
		self.assure_passes_tests()?;
		let code = self.directory.get_source()?;
		let mut ui = self.make_ui();
		let manifest = manifest::Manifest::load(&self.directory.get_manifest()?);
		ci::commands::submit::run(&manifest.task_url, &code, &mut ui)?;
		Ok(())
	}

	fn manual_submit(&mut self) -> R<()> {
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

	fn random_codename(&mut self) -> R<String> {
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

	fn run_tests(&mut self) -> R<(bool, Option<(ci::testing::TestResult, PathBuf)>)> {
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
				impulse => Err(error::Category::UnexpectedImpulse {
					description: format!("{:?}", impulse),
				})?,
			}
		};
		t1.join().map_err(|_| error::Category::ThreadPanicked)?;
		Ok((success, first_failed))
	}

	fn assure_passes_tests(&mut self) -> R<()> {
		let (_, first_fail) = self.run_tests()?;
		if let Some((verdict, path)) = first_fail {
			Err(error::Category::TestFailure { verdict, path })?
		} else {
			Ok(())
		}
	}

	fn assure_compiled(&mut self) -> R<()> {
		if self.requires_compilation()? {
			self.build()?;
		}
		Ok(())
	}

	fn assure_all_saved(&mut self) -> R<()> {
		self.send(Reaction::SaveAll);
		match self.recv() {
			Impulse::SavedAll => {},
			impulse => Err(error::Category::UnexpectedImpulse {
				description: format!("{:?}", impulse),
			})?,
		}
		Ok(())
	}

	fn requires_compilation(&mut self) -> R<bool> {
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

	fn make_ui(&mut self) -> impulse_ui::ImpulseCiUi {
		impulse_ui::ImpulseCiUi(self.input_sender.clone())
	}

	fn info(&mut self, message: impl Into<String>) {
		self.send(Reaction::InfoMessage { message: message.into() });
	}

	fn error(&mut self, message: impl Into<String>) {
		self.send(Reaction::ErrorMessage { message: message.into() });
	}

	fn log(&mut self, message: impl Into<String>) {
		self.send(Reaction::ConsoleLog { message: message.into() });
	}

	fn input_box(&mut self, options: InputBoxOptions) -> R<Option<String>> {
		self.send(Reaction::InputBox { options });
		match self.recv() {
			Impulse::InputBox { response } => Ok(response),
			impulse => Err(error::Category::UnexpectedImpulse {
				description: format!("{:?}", impulse),
			})?,
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

	fn need_root(&self) -> R<&std::path::Path> {
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

	fn get_template_main(&self) -> R<PathBuf> {
		Ok(dirs::config_dir()
			.ok_or(error::Category::DegenerateEnvironment { detail: "no config directory " })?
			.join("icie/template-main.cpp"))
	}

	fn get_manifest(&self) -> R<PathBuf> {
		Ok(self.need_root()?.join(".icie"))
	}
}
