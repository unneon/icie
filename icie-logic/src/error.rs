use ci;
use failure;
use std::fmt;

#[derive(Debug)]
pub enum Category {
	TestFailure { verdict: ci::testing::TestResult, path: std::path::PathBuf },
	UnexpectedImpulse { description: String, target: &'static str },
	DegenerateEnvironment { detail: &'static str },
	NoOpenFolder,
	LackOfInput,
	ThreadPanicked,
	NoCuteAnimals,
	MalformedConfig { detail: &'static str },
	TemplateDoesNotExist { id: String },
	FileAlreadyExists { path: std::path::PathBuf },
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
			}
		)?;
		for frame in self.trace.frames() {
			for symbol in frame.symbols() {
				writeln!(f, "{:?} {:?}:{:?}", symbol.name(), symbol.filename(), symbol.lineno())?;
			}
		}
		Ok(())
	}
}

impl failure::Fail for Error {}
