use crate::ci::util;
use evscode::{E, R};
use std::{
	io::Write, path::PathBuf, process::{Command, ExitStatus, Stdio}, time::{Duration, Instant}
};
use wait_timeout::ChildExt;

#[derive(Debug)]
pub struct Environment {
	pub time_limit: Option<Duration>,
}
#[derive(Debug, Eq, PartialEq)]
pub enum ExitKind {
	Normal,
	TimeLimitExceeded,
}
#[derive(Debug)]
pub struct Run {
	pub stdout: String,
	pub stderr: String,
	pub status: ExitStatus,
	pub exit_kind: ExitKind,
	pub time: Duration,
}

impl Run {
	pub fn success(&self) -> bool {
		self.status.success() && self.exit_kind == ExitKind::Normal
	}
}

#[derive(Debug)]
pub struct Executable {
	pub path: PathBuf,
}
impl Executable {
	pub fn new(path: PathBuf) -> Executable {
		Executable { path }
	}

	pub fn run(&self, input: &str, args: &[&str], environment: &Environment) -> R<Run> {
		let mut cmd = Command::new(&self.path);
		cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).args(args);
		let t1 = Instant::now();
		let mut kid = cmd.spawn().map_err(|e| E::from_std(e).context(format!("failed to execute {:?}", self.path)))?;
		let _ = kid.stdin.as_mut().unwrap().write_all(input.as_bytes());
		let _ = kid.stdin.as_mut().unwrap().flush();
		let (status, exit_kind) = if let Some(time_limit) = environment.time_limit {
			if let Some(status) =
				kid.wait_timeout(time_limit).map_err(|e| E::from_std(e).context(format!("lost child process of {:?}", self.path)))?
			{
				(status, ExitKind::Normal)
			} else {
				kid.kill().map_err(|e| E::from_std(e).context(format!("could not kill process of {:?} after time limit", self.path)))?;
				(kid.wait().map_err(|e| E::from_std(e).context(format!("lost child process of {:?}", self.path)))?, ExitKind::TimeLimitExceeded)
			}
		} else {
			(kid.wait().map_err(|e| E::from_std(e).context(format!("lost child process of {:?}", self.path)))?, ExitKind::Normal)
		};
		let t2 = Instant::now();
		Ok(Run {
			stdout: String::from_utf8(
				util::io_read(kid.stdout.unwrap())
					.map_err(|e| E::from_std(e).context(format!("could not extract stdout of process of {:?}", self.path)))?,
			)
			.unwrap(),
			stderr: String::from_utf8(
				util::io_read(kid.stderr.unwrap())
					.map_err(|e| E::from_std(e).context(format!("could not extract stderr of process of {:?}", self.path)))?,
			)
			.unwrap(),
			status,
			exit_kind,
			time: t2 - t1,
		})
	}
}
