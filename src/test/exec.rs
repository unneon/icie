use crate::checker::Checker;
use evscode::{error::ResultExt, R};
use futures::future::join3;
use std::{
	path::PathBuf, process::{ExitStatus, Stdio}, time::{Duration, Instant}
};
use tokio::{
	future::FutureExt, io::{AsyncReadExt, AsyncWriteExt}, timer::timeout::Elapsed
};
use tokio_net::process::Command;

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
pub struct Environment {
	pub time_limit: Option<Duration>,
}

#[derive(Debug)]
pub struct Task {
	pub checker: Box<dyn Checker+Send+Sync>,
	pub environment: Environment,
}

#[derive(Debug, Clone)]
pub struct Executable {
	pub path: PathBuf,
}

impl Executable {
	pub fn new(path: PathBuf) -> Executable {
		Executable { path }
	}

	pub async fn run(&self, input: &str, args: &[&str], environment: &Environment) -> R<Run> {
		let mut cmd = Command::new(&self.path);
		cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).args(args);
		let mut kid = cmd.spawn().wrap(format!("execution of {} aborted", self.path.display()))?;
		let t1 = Instant::now();
		let mut stdin_stream = kid.stdin().take().unwrap();
		let mut stdout_stream = kid.stdout().take().unwrap();
		let mut stderr_stream = kid.stderr().take().unwrap();
		let _ = stdin_stream.write_all(input.as_bytes()).await;
		let _ = stdin_stream.flush().await;
		let mut stdout = String::new();
		let mut stderr = String::new();
		let capture_stdout = stdout_stream.read_to_string(&mut stdout);
		let capture_stderr = stderr_stream.read_to_string(&mut stderr);
		let kid_ref = &mut kid;
		let drive_exec = async {
			let status = if let Some(time_limit) = environment.time_limit { kid_ref.timeout(time_limit).await } else { Ok(kid_ref.await) };
			let t2 = Instant::now();
			(status, t2)
		};
		let ((exec_summary, t2), stdout_cap, stderr_cap) = join3(drive_exec, capture_stdout, capture_stderr).await;
		stdout_cap.wrap(format!("extraction of {} output failed", self.path.display()))?;
		stderr_cap.wrap(format!("extraction of {} diagnostic output failed", self.path.display()))?;
		let (status, exit_kind) = match exec_summary {
			Ok(status) => (status.wrap(format!("lost child of {}", self.path.display()))?, ExitKind::Normal),
			Err(Elapsed { .. }) => {
				kid.kill().wrap(format!("killing of {} failed", self.path.display()))?;
				(kid.await.wrap(format!("lost {} child", self.path.display()))?, ExitKind::TimeLimitExceeded)
			},
		};
		Ok(Run { stdout, stderr, status, exit_kind, time: t2 - t1 })
	}
}
