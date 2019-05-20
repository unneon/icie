use crate::ci::{
	exec::{Executable, ExitKind}, task::Task, util::{self, R}
};
use std::{fmt, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
	Accepted,
	WrongAnswer,
	RuntimeError,
	TimeLimitExceeded,
	IgnoredNoOut,
}
#[derive(Clone, Debug)]
pub struct Outcome {
	pub verdict: Verdict,
	pub out: String,
	pub time: Duration,
}

pub fn simple_test(exec: &Executable, input: &str, desired: Option<&str>, task: &Task) -> R<Outcome> {
	let (time, run) = util::time_fn(|| exec.run(input, &task.environment));
	let run = run?;
	let verdict = match run.exit_kind {
		ExitKind::Normal => {
			if run.status.success() {
				if let Some(desired) = desired {
					if task.checker.judge(input, desired, &run.stdout) {
						Verdict::Accepted
					} else {
						Verdict::WrongAnswer
					}
				} else {
					Verdict::IgnoredNoOut
				}
			} else {
				Verdict::RuntimeError
			}
		},
		ExitKind::TimeLimitExceeded => Verdict::TimeLimitExceeded,
	};
	let out = run.stdout;
	Ok(Outcome { verdict, out, time })
}

impl fmt::Display for Verdict {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use Verdict::*;
		write!(
			f,
			"{}",
			match self {
				Accepted => "Accept",
				WrongAnswer => "Wrong Answer",
				RuntimeError => "Runtime Error",
				TimeLimitExceeded => "Time Limit Exceeded",
				IgnoredNoOut => "Ignored (no out)",
			}
		)
	}
}
