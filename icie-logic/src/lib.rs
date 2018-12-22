extern crate ci;
extern crate rand;

#[macro_use]
mod error;
mod handle;
mod impulse_ui;
mod vscode;

pub use self::handle::Handle;
use self::{error::R, vscode::*};
use rand::prelude::SliceRandom;
use std::{
	env, fs, path::PathBuf, sync::mpsc::{Receiver, Sender}, thread, time::Duration
};

#[derive(Debug)]
pub enum Impulse {
	Ping,
	TriggerBuild,
	TriggerTest,
	TriggerInit,
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
}

macro_rules! dbg {
	($x:expr) => {
		format!("{} = {:?}", stringify!($x), $x)
	};
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
				Err(err) => self
					.output
					.send(Reaction::ErrorMessage {
						message: format!("ICIE Error ❄ {}", err),
					})
					.unwrap(),
			}
		}
	}

	fn process(&mut self) -> R<()> {
		match self.input.recv().unwrap() {
			Impulse::Ping => self.info(dbg!(std::env::current_dir()))?,
			Impulse::WorkspaceInfo { root_path } => self.directory.set_root_path(root_path),
			Impulse::TriggerBuild => self.build()?,
			Impulse::TriggerTest => self.test()?,
			Impulse::TriggerInit => self.init()?,
			imp => er!("Unexpected impulse {:?}", imp),
		}
		Ok(())
	}

	fn build(&mut self) -> R<()> {
		let source = self.directory.get_source();
		let codegen = ci::commands::build::Codegen::Debug;
		let cppver = ci::commands::build::CppVer::Cpp17;
		let library = self.directory.get_library_source();
		let library: Option<&std::path::Path> = library.as_ref().map(|p| p.as_ref());
		self.log(format!("source = {:?}, codegen = {:?}, cppver = {:?}, library = {:?}", source, codegen, cppver, library))?;
		ci::commands::build::run(&source, &codegen, &cppver, library).unwrap();
		self.info("Compilation successful!")?;
		Ok(())
	}

	fn test(&mut self) -> R<()> {
		self.assure_compiled()?;
		let executable = self.directory.get_executable();
		let testdir = self.directory.get_tests();
		let mut ui = impulse_ui::ImpulseCiUi(self.input_sender.clone());
		let t1 = thread::spawn(move || {
			let _ = ci::commands::test::run(&executable, &testdir, &ci::checkers::CheckerDiffOut, false, false, &mut ui);
		});
		let success = loop {
			match self.input.recv().unwrap() {
				Impulse::CiTestSingle { outcome, timing, in_path } => self.log(format!("{:?} {:?} {:?}", outcome, timing, in_path))?,
				Impulse::CiTestFinish { success } => break success,
				imp => er!("Unexpected impulse {:?}", imp),
			}
		};
		if success {
			self.info("Tests run successfully")?;
		} else {
			self.output
				.send(Reaction::ErrorMessage {
					message: "Some tests failed".to_owned(),
				})
				.unwrap();
		}
		t1.join().unwrap();
		Ok(())
	}

	fn init(&mut self) -> R<()> {
		let name = self.random_codename();
		let root = env::home_dir().unwrap().join(&name);
		let url = match self.input_box(InputBoxOptions {
			ignore_focus_out: true,
			password: false,
			placeholder: Some("https://codeforces.com/contest/960/problem/D".to_owned()),
			prompt: Some("Enter task URL".to_owned()),
		})? {
			Some(url) => url,
			None => {
				self.info("ICIE Init cancelled")?;
				return Ok(());
			},
		};
		let mut ui = impulse_ui::ImpulseCiUi(self.input_sender.clone());
		ci::util::demand_dir(&root).unwrap();

		let root2 = root.clone();
		let t1 = thread::spawn(move || {
			let _ = ci::commands::init::run(&url, &root2, &mut ui);
		});
		loop {
			match self.input.recv().unwrap() {
				Impulse::CiAuthRequest { domain, channel } => {
					let username = self
						.input_box(InputBoxOptions {
							prompt: Some(format!("Username at {}", domain)),
							placeholder: None,
							ignore_focus_out: false,
							password: false,
						})?
						.unwrap();
					let password = self
						.input_box(InputBoxOptions {
							prompt: Some(format!("Password for {} at {}", username, domain)),
							placeholder: None,
							ignore_focus_out: false,
							password: true,
						})?
						.unwrap();
					channel.send(Some((username, password))).unwrap();
				},
				Impulse::CiInitFinish => break,
				imp => er!("Unexpected impulse {:?}", imp),
			}
		}
		t1.join().unwrap();

		fs::copy(&self.directory.get_template_main(), &root.join("main.cpp")).unwrap();
		self.output.send(Reaction::OpenFolder { path: root, in_new_window: false }).unwrap();
		Ok(())
	}

	fn random_codename(&mut self) -> String {
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
		format!("{}-{}", ADJECTIVES.choose(&mut rng).unwrap(), ANIMALS.choose(&mut rng).unwrap())
	}

	fn assure_compiled(&mut self) -> R<()> {
		if self.requires_compilation()? {
			self.build()?;
		}
		Ok(())
	}

	fn assure_all_saved(&mut self) -> R<()> {
		self.output.send(Reaction::SaveAll).unwrap();
		match self.input.recv().unwrap() {
			Impulse::SavedAll => {},
			imp => er!("Unexpected impulse {:?}", imp),
		}
		Ok(())
	}

	fn requires_compilation(&mut self) -> R<bool> {
		let src = self.directory.get_source();
		let exe = self.directory.get_executable();
		self.assure_all_saved()?;
		let metasrc = src.metadata().unwrap();
		let metaexe = exe.metadata().unwrap();
		Ok(metasrc.modified().unwrap() >= metaexe.modified().unwrap())
	}

	fn info(&mut self, message: impl Into<String>) -> R<()> {
		self.output.send(Reaction::InfoMessage { message: message.into() }).unwrap();
		Ok(())
	}

	fn log(&mut self, message: impl Into<String>) -> R<()> {
		self.output.send(Reaction::ConsoleLog { message: message.into() }).unwrap();
		Ok(())
	}

	fn input_box(&mut self, options: InputBoxOptions) -> R<Option<String>> {
		self.output.send(Reaction::InputBox { options }).unwrap();
		match self.input.recv().unwrap() {
			Impulse::InputBox { response } => Ok(response),
			imp => er!("Unexpected impulse {:?}", imp),
		}
	}
}

struct Directory {
	root: Option<String>,
}
impl Directory {
	pub(crate) fn new_empty() -> Directory {
		Directory { root: None }
	}

	fn set_root_path(&mut self, root: Option<String>) {
		self.root = root;
	}

	fn get_source(&self) -> PathBuf {
		PathBuf::from(format!("{}/main.cpp", self.root.as_ref().unwrap()))
	}

	fn get_library_source(&self) -> Option<PathBuf> {
		let path = PathBuf::from(format!("{}/lib.cpp", self.root.as_ref().unwrap()));
		if path.exists() {
			Some(path)
		} else {
			None
		}
	}

	fn get_executable(&self) -> PathBuf {
		PathBuf::from(format!("{}/main.e", self.root.as_ref().unwrap()))
	}

	fn get_tests(&self) -> PathBuf {
		PathBuf::from(format!("{}/tests", self.root.as_ref().unwrap()))
	}

	fn get_template_main(&self) -> PathBuf {
		// TODO use xdg config directory
		env::home_dir().unwrap().join(".config/icie/template-main.cpp")
	}
}
