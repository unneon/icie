use ci;
use failure;

#[derive(Debug, Fail)]
pub enum Category {
	#[fail(display = "{:?} on test {:?}", verdict, path)]
	TestFailure { verdict: ci::testing::TestResult, path: std::path::PathBuf },
	#[fail(display = "unexpected impulse {:?} when waiting for {}", description, target)]
	UnexpectedImpulse { description: String, target: &'static str },
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
	#[fail(display = "malformed config: {}", detail)]
	MalformedConfig { detail: &'static str },
}

pub type R<T> = Result<T, failure::Error>;

pub fn unexpected(impulse: crate::Impulse, target: &'static str) -> Category {
	Category::UnexpectedImpulse {
		description: format!("{:?}", impulse),
		target,
	}
}
