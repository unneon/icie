extern crate ci;

#[macro_use]
mod error;
mod handle;
mod vscode;

pub use self::handle::Handle;
use self::{error::R, vscode::*};
use std::{
	path::PathBuf, sync::mpsc::{Receiver, Sender}
};

#[derive(Debug)]
pub enum Impulse {
	Ping,
	TriggerBuild,
	WorkspaceInfo { root_path: Option<String> },
	QuickPick { response: Option<String> },
	InputBox { response: Option<String> },
}
pub enum Reaction {
	Status { message: Option<String> },
	InfoMessage { message: String },
	ErrorMessage { message: String },
	QuickPick { items: Vec<QuickPickItem> },
	InputBox { options: InputBoxOptions },
	ConsoleLog { message: String },
}

macro_rules! dbg {
	($x:expr) => {
		format!("{} = {:?}", stringify!($x), $x)
	};
}

struct ICIE {
	input: Receiver<Impulse>,
	output: Sender<Reaction>,

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
			Impulse::TriggerBuild => {
				let source = self.directory.get_source();
				let codegen = ci::commands::build::Codegen::Debug;
				let cppver = ci::commands::build::CppVer::Cpp17;
				let library = self.directory.get_library_source();
				let library: Option<&std::path::Path> = library.as_ref().map(|p| p.as_ref());
				self.info(dbg!(source))?;
				self.info(dbg!(codegen))?;
				self.info(dbg!(cppver))?;
				self.info(dbg!(library))?;
				ci::commands::build::run(&source, &codegen, &cppver, library).unwrap();
				self.info("Compilation successful!")?;
			},
			imp => er!("Unexpected impulse {:?}", imp),
		}
		Ok(())
	}

	fn info(&mut self, message: impl Into<String>) -> R<()> {
		self.output.send(Reaction::InfoMessage { message: message.into() }).unwrap();
		Ok(())
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
}
