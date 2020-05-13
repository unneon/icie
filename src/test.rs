mod data;
pub mod judge;
pub mod scan;
pub mod view;

use crate::{
	compile::{self, Codegen}, dir, telemetry::TELEMETRY, test::{judge::simple_test, scan::scan_for_tests}, util, util::{fs, path::Path, SourceTarget}
};
use evscode::R;
use std::time::Duration;

pub use data::{Outcome, Task, TestRun, Verdict};

/// The maximum time an executable can run before getting a Time Limit Exceeded verdict, specified in milliseconds.
/// Leaving this empty(which denotes no limit) is not recommended, because this will cause stuck processes to run
/// indefinitely, wasting system resources.
#[evscode::config]
static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);

pub async fn run(source: SourceTarget) -> R<Vec<TestRun>> {
	let _status = crate::STATUS.push("Testing");
	TELEMETRY.test_run.spark();
	let solution = compile::compile(&source, Codegen::Debug, false).await?;
	let task = Task::simple().await?;
	let inputs = scan_for_tests(&dir::TESTS_DIRECTORY.get()).await;
	let progress = evscode::Progress::new().title(util::fmt::verb_on_source("Testing", &source)).show().0;
	let mut runs = Vec::new();
	for input_path in &inputs {
		let input = fs::read_to_string(&input_path).await?;
		let output = load_test_output(input_path, "out").await?;
		let output_alt = load_test_output(input_path, "alt.out").await?;
		let outcome = simple_test(&solution, &input, output.as_deref(), output_alt.as_deref(), &task).await?;
		let output_path = input_path.with_extension("out");
		let run = TestRun { in_path: input_path.clone(), out_path: output_path, outcome };
		update_test_progress(&run, inputs.len(), &progress)?;
		runs.push(run);
	}
	Ok(runs)
}

async fn load_test_output(input_path: &Path, ext: &str) -> R<Option<String>> {
	let path = input_path.with_extension(ext);
	match fs::read_to_string(&path).await {
		Ok(output) => Ok(Some(output)),
		Err(ref e) if e.human().contains("ENOENT: no such file or directory") => Ok(None),
		Err(e) => Err(e.context(format!("could not read test output {}", path))),
	}
}

fn update_test_progress(run: &TestRun, count: usize, progress: &evscode::Progress) -> R<()> {
	let name = run.in_path.fmt_relative(&dir::tests()?);
	let inc = 100. / count as f64;
	let msg_time = util::fmt::time(&run.outcome.time);
	let msg = format!("{} on `{}` in {}", run.outcome.verdict, name, msg_time);
	progress.update_inc(inc, msg);
	Ok(())
}

#[evscode::command(title = "ICIE Open Test View", key = "alt+0")]
pub async fn view() -> R<()> {
	TELEMETRY.test_alt0.spark();
	view::manage::COLLECTION.get_force(SourceTarget::Main).await?;
	Ok(())
}

#[evscode::command(title = "ICIE Open Test View (current editor)", key = "alt+\\ alt+0")]
async fn view_current() -> R<()> {
	TELEMETRY.test_current.spark();
	view::manage::COLLECTION.get_force(util::active_tab().await?).await?;
	Ok(())
}

#[evscode::command(title = "ICIE New Test", key = "alt+-")]
pub async fn input() -> evscode::R<()> {
	TELEMETRY.test_input.spark();
	let webview = view::manage::COLLECTION.active_or_lazy(SourceTarget::Main).await?;
	// FIXME: JS .focus() does not work despite the reveal.
	webview.reveal(2, false);
	webview.post_message(view::manage::Food::NewStart).await;
	Ok(())
}

pub async fn add_test(input: &str, desired: &str) -> R<()> {
	TELEMETRY.test_add.spark();
	let tests = dir::custom_tests()?;
	fs::create_dir_all(&tests).await?;
	let id = unused_test_id(&tests).await?;
	let in_path = tests.join(format!("{}.in", id));
	let out_path = tests.join(format!("{}.out", id));
	fs::write(&in_path, input).await?;
	fs::write(&out_path, desired).await?;
	view::manage::COLLECTION.update_all().await?;
	Ok(())
}

async fn unused_test_id(dir: &Path) -> R<i64> {
	let tests = fs::read_dir(dir).await?;
	let taken = tests.into_iter().filter_map(|test| test.file_stem().parse().ok()).collect();
	Ok(util::mex(1, taken))
}

pub fn time_limit() -> Option<Duration> {
	TIME_LIMIT.get().map(|ms| Duration::from_millis(ms as u64))
}
