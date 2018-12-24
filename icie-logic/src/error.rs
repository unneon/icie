use ci;
use failure;

#[derive(Debug, Fail)]
pub enum Category {
	#[fail(display = "{:?} on test {:?}", verdict, path)]
	TestFailure { verdict: ci::testing::TestResult, path: std::path::PathBuf },
	#[fail(display = "unexpected impulse {:?}", description)]
	UnexpectedImpulse { description: String },
	#[fail(display = "degenerate environment: {}", detail)]
	DegenerateEnvironment { detail: &'static str },
	#[fail(display = "no directory opened")]
	NoOpenFolder,
	#[fail(display = "operation cancelled due to lack of input")]
	LackOfInput,
	#[fail(display = "thread has suddenly panicked")]
	ThreadPanicked,
	#[fail(display = "ran out of cute animals")]
	NoCuteAnimals,
}

pub type R<T> = Result<T, failure::Error>;
