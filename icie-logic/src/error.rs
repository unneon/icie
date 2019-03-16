use ci::{self, commands::build::CppVer};
use failure;
use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub enum Category {
	TestFailure { verdict: ci::testing::Outcome, path: std::path::PathBuf },
	UnexpectedImpulse { description: String, target: &'static str },
	DegenerateEnvironment { detail: &'static str },
	NoOpenFolder,
	LackOfInput,
	ThreadPanicked,
	NoCuteAnimals,
	MalformedConfig { detail: &'static str },
	TemplateDoesNotExist { id: String },
	FileAlreadyExists { path: std::path::PathBuf },
	NonUTF8Path,
	AppNotInstalled { apps: Vec<String>, suggestion: &'static str },
	CompilationError { message: Option<String>, file: PathBuf, mode: CppVer },
}

#[derive(Debug)]
pub struct Error {
	category: Category,
	trace: backtrace::Backtrace,
}

pub type R<T> = Result<T, failure::Error>;

pub fn unexpected(impulse: crate::Impulse, target: &'static str) -> Category {
	Category::UnexpectedImpulse {
		description: format!("{:?}", impulse),
		target,
	}
}

impl Category {
	pub fn err(self) -> Error {
		Error {
			category: self,
			trace: backtrace::Backtrace::new(),
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use self::Category::*;
		writeln!(
			f,
			"{}",
			match &self.category {
				TestFailure { verdict, path } => format!("{:?} on test {:?}", verdict, path),
				UnexpectedImpulse { description, target } => format!("unexpected impulse {:?} when waiting for {}", description, target),
				DegenerateEnvironment { detail } => format!("degenerate environment: {}", detail),
				NoOpenFolder => format!("no directory opened"),
				LackOfInput => format!("operation cancelled due to lack of input"),
				ThreadPanicked => format!("thread has suddenly panicked"),
				NoCuteAnimals => format!("ran out of cute animals"),
				MalformedConfig { detail } => format!("malformed config: {}", detail),
				TemplateDoesNotExist { id } => format!("template {:?} does not exist", id),
				FileAlreadyExists { path } => format!("file {:?} already exists", path),
				NonUTF8Path => format!("tried to process non-UTF8 path"),
				AppNotInstalled { apps, suggestion } => format!("none of {:?} are installed; try running `{}`", apps, suggestion),
				CompilationError { message, file, mode } => format!(
					"{}failed to compile {} in {} mode",
					message.as_ref().map(|message| format!("{}\n", message)).unwrap_or("".to_owned()),
					file.display(),
					mode.flag()
				),
			}
		)?;
		Ok(())
	}
}

impl failure::Fail for Error {}
