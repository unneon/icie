use crate::{
	build::{build, clang::Codegen}, dir, telemetry::TELEMETRY, test::exec::{Environment, Executable}
};
use async_trait::async_trait;
use evscode::{error::ResultExt, R};
use std::{fmt, io::Write, time::Duration};
use tempfile::NamedTempFile;

/// The maximum time a checker executable can run before getting killed, specified in milliseconds. Killing will cause the test to be classified as failed. Leaving this empty(which denotes no limit) is not recommended, because this will cause stuck processes to run indefinitely, wasting system resources.
#[evscode::config]
static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);

pub async fn get_checker() -> R<Box<dyn Checker+Send+Sync>> {
	let checker = dir::checker()?;
	Ok(if !checker.exists() {
		let bx: Box<dyn Checker+Send+Sync> = Box::new(FreeWhitespaceChecker);
		bx
	} else {
		TELEMETRY.checker_exists.spark();
		let environment = Environment { time_limit: (*TIME_LIMIT.get()).map(Duration::from_millis) };
		let executable = build(checker, &Codegen::Release, false).await?;
		Box::new(ExecChecker { executable, environment })
	})
}

#[async_trait]
pub trait Checker: fmt::Debug {
	async fn judge(&self, input: &str, desired: &str, out: &str) -> R<bool>;
}

#[derive(Debug)]
pub struct FreeWhitespaceChecker;

#[async_trait]
impl Checker for FreeWhitespaceChecker {
	async fn judge(&self, _input: &str, desired: &str, out: &str) -> R<bool> {
		Ok(self.equal_bew(desired, out))
	}
}

impl FreeWhitespaceChecker {
	fn equal_bew(&self, a: &str, b: &str) -> bool {
		let mut i = a.chars().peekable();
		let mut j = b.chars().peekable();
		while i.peek().is_some() && j.peek().is_some() {
			if i.peek().unwrap().is_whitespace() && j.peek().unwrap().is_whitespace() {
				while i.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
					i.next();
				}
				while j.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
					j.next();
				}
			} else {
				if i.peek() != j.peek() {
					return false;
				}
				i.next();
				j.next();
			}
		}
		for c in i {
			if !c.is_whitespace() {
				return false;
			}
		}
		for c in j {
			if !c.is_whitespace() {
				return false;
			}
		}
		true
	}
}

#[derive(Debug)]
pub struct ExecChecker {
	pub executable: Executable,
	pub environment: Environment,
}

#[async_trait]
impl Checker for ExecChecker {
	async fn judge(&self, input: &str, desired: &str, out: &str) -> R<bool> {
		let mut input_file = NamedTempFile::new().wrap("failed to create temporary input file")?;
		let mut desired_file = NamedTempFile::new().wrap("failed to create temporary correct-output file")?;
		let mut out_file = NamedTempFile::new().wrap("failed to create temporary output file")?;
		input_file.write_all(input.as_bytes()).wrap("failed to fill temporary input file")?;
		desired_file.write_all(desired.as_bytes()).wrap("failed to fill temporary correct-output file")?;
		out_file.write_all(out.as_bytes()).wrap("failed to fill temporary output file")?;
		let args = [input_file.path().to_str().unwrap(), out_file.path().to_str().unwrap(), desired_file.path().to_str().unwrap()];
		let run = self.executable.run("", &args, &self.environment).await?;
		Ok(run.success())
	}
}
