use crate::{
	checker::{get_checker, Checker}, executable::Environment, test::time_limit, util::path::Path
};
use evscode::R;
use std::{fmt, time::Duration};

#[derive(Debug)]
pub struct Outcome {
	pub verdict: Verdict,
	pub out: String,
	pub stderr: String,
	pub time: Duration,
}

#[derive(Debug)]
pub struct Task {
	pub checker: Box<dyn Checker+Send+Sync>,
	pub environment: Environment,
}

#[derive(Debug)]
pub struct TestRun {
	pub in_path: Path,
	pub out_path: Path,
	pub outcome: Outcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
	Accepted { alternative: bool },
	WrongAnswer,
	RuntimeError,
	TimeLimitExceeded,
	IgnoredNoOut,
}

impl Outcome {
	pub fn success(&self) -> bool {
		self.verdict.success()
	}
}

impl Task {
	pub async fn simple() -> R<Task> {
		let checker = get_checker().await?;
		let environment = Environment { time_limit: time_limit(), cwd: None };
		Ok(Task { checker, environment })
	}
}

impl TestRun {
	pub fn success(&self) -> bool {
		self.outcome.success()
	}
}

impl Verdict {
	pub fn success(self) -> bool {
		matches!(self, Verdict::Accepted { .. })
	}
}

impl fmt::Display for Verdict {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let message = match self {
			Verdict::Accepted { .. } => "Accept",
			Verdict::WrongAnswer => "Wrong Answer",
			Verdict::RuntimeError => "Runtime Error",
			Verdict::TimeLimitExceeded => "Time Limit Exceeded",
			Verdict::IgnoredNoOut => "Ignored (no output file)",
		};
		write!(f, "{}", message)
	}
}
