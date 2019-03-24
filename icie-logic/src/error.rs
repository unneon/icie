use crate::util;
use ci::{self, commands::build::CppVer};
use failure;
use std::{fmt, fs::File, io::Write, path::PathBuf};

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
	PerfEventParanoid,
	MalformedLibrary { detail: &'static str },
}

#[derive(Debug)]
pub struct Error {
	category: Category,
	trace: failure::Backtrace,
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
			trace: failure::Backtrace::new(),
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
				PerfEventParanoid => format!(
					"rr complains about priviledges; try running `echo kernel.perf_event_paranoid = 1 | sudo tee -a /etc/sysctl.conf && echo 1 | sudo tee \
					 /proc/sys/kernel/perf_event_paranoid`"
				),
				MalformedLibrary { detail } => format!("malformed library description: {}", detail),
			}
		)?;
		Ok(())
	}
}

pub fn save_details(err: &failure::Error) -> R<PathBuf> {
	let i = find_log_number()?;
	let p = log_file(i)?;
	util::assure_dir(p.parent().unwrap())?;
	let mut f = File::create(&p)?;
	writeln!(f, "icie error report")?;
	writeln!(f, "{}", err)?;
	writeln!(f, "{}", err.backtrace())?;
	Ok(p)
}
fn find_log_number() -> R<i32> {
	binsearch(0, std::i32::MAX, |i| Ok(!log_file(i)?.exists()))
}
fn binsearch<F: Fn(i32) -> R<bool>>(a: i32, b: i32, f: F) -> R<i32> {
	Ok(if b - a == 0 {
		a
	} else if b - a == 1 {
		if f(a)? {
			a
		} else {
			b
		}
	} else {
		let m = (a + b) / 2;
		if f(m)? {
			binsearch(a, m, f)?
		} else {
			binsearch(m + 1, b, f)?
		}
	})
}
fn log_file(i: i32) -> R<PathBuf> {
	Ok(dirs::cache_dir()
		.ok_or_else(|| Category::DegenerateEnvironment { detail: "no cache directory" }.err())?
		.join("icie")
		.join(format!("error{}.log", i)))
}

impl failure::Fail for Error {
	fn backtrace(&self) -> Option<&failure::Backtrace> {
		Some(&self.trace)
	}
}
