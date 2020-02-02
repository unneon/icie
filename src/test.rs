pub mod judge;
pub mod scan;
pub mod view;

use crate::{
	build::{self, Codegen}, checker::Checker, dir, executable::{Environment, Executable}, telemetry::TELEMETRY, test::{
		judge::{simple_test, Outcome}, scan::scan_and_order
	}, util, util::{fs, path::Path}
};
use evscode::{error::ResultExt, webview::WebviewRef, R};
use futures::{SinkExt, Stream, StreamExt};
use std::time::Duration;

#[derive(Debug)]
pub struct TestRun {
	in_path: Path,
	out_path: Path,
	outcome: Outcome,
}
impl TestRun {
	pub fn success(&self) -> bool {
		self.outcome.success()
	}
}

#[derive(Debug)]
pub struct Task {
	pub checker: Box<dyn Checker+Send+Sync>,
	pub environment: Environment,
}

/// The maximum time an executable can run before getting a Time Limit Exceeded verdict, specified
/// in milliseconds. Leaving this empty(which denotes no limit) is not recommended, because this
/// will cause stuck processes to run indefinitely, wasting system resources.
#[evscode::config]
static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);

pub async fn run(main_source: &Option<Path>) -> R<Vec<TestRun>> {
	let _status = crate::STATUS.push("Testing");
	TELEMETRY.test_run.spark();
	let solution = build::build(main_source, Codegen::Debug, false).await?;
	let task = Task {
		checker: crate::checker::get_checker().await?,
		environment: Environment { time_limit: time_limit(), cwd: None },
	};
	let test_dir_name = dir::TESTS_DIRECTORY.get();
	let test_dir = dir::tests()?;
	let ins = scan_and_order(&test_dir_name).await;
	let mut runs = Vec::new();
	let test_count = ins.len();
	let progress = evscode::Progress::new().title(util::fmt_verb("Testing", &main_source)).show().0;
	let mut worker = run_thread(ins, task, solution);
	for _ in 0..test_count {
		let run = worker.next().await.wrap("did not ran all tests due to an internal panic")??;
		let name =
			run.in_path.strip_prefix(&test_dir).wrap("found test outside of test directory")?;
		progress.update_inc(
			100.0 / test_count as f64,
			format!(
				"{} on `{}` in {}",
				run.outcome.verdict,
				name,
				util::fmt_time_short(&run.outcome.time)
			),
		);
		runs.push(run);
	}
	Ok(runs)
}

pub fn time_limit() -> Option<Duration> {
	TIME_LIMIT.get().map(|ms| Duration::from_millis(ms as u64))
}

fn run_thread(ins: Vec<Path>, task: Task, solution: Executable) -> impl Stream<Item=R<TestRun>> {
	let (tx, rx) = futures::channel::mpsc::unbounded();
	evscode::spawn(async {
		let mut tx = tx;
		let task = task;
		let solution = solution;
		for in_path in ins {
			// TODO: Refactor try block into a function
			let r = try {
				let out_path = in_path.with_extension("out");
				let alt_path = in_path.with_extension("alt.out");
				let input = fs::read_to_string(&in_path).await?;
				let output = match fs::read_to_string(&out_path).await {
					Ok(output) => Some(output),
					// Matching on JS errors would be irritating, so let's just do this.
					Err(ref e) if e.human().contains("ENOENT: no such file or directory") => None,
					Err(e) => {
						return Err(e.context(format!("failed to read test out {}", out_path)));
					},
				};
				let alt = if fs::exists(&alt_path).await? {
					Some(fs::read_to_string(&alt_path).await?)
				} else {
					None
				};
				let outcome =
					simple_test(&solution, &input, output.as_deref(), alt.as_deref(), &task)
						.await
						.map_err(|e| e.context("failed to run test"))?;
				let run = TestRun { in_path, out_path, outcome };
				if tx.send(Ok(run)).await.is_err() {
					break;
				}
			};
			match r {
				Ok(()) => (),
				Err(e) => {
					let _ = tx.send(Err(e)).await;
				},
			}
		}
		Ok(())
	});
	rx
}

#[evscode::command(title = "ICIE Open Test View", key = "alt+0")]
async fn view() -> R<()> {
	TELEMETRY.test_alt0.spark();
	view::manage::COLLECTION.get_force(None).await?;
	Ok(())
}

#[evscode::command(title = "ICIE Open Test View (current editor)", key = "alt+\\ alt+0")]
async fn view_current() -> R<()> {
	TELEMETRY.test_current.spark();
	view::manage::COLLECTION.get_force(util::active_tab().await?).await?;
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

#[evscode::command(title = "ICIE New Test", key = "alt+-")]
pub async fn input() -> evscode::R<()> {
	TELEMETRY.test_input.spark();
	let view = if let Some(view) = view::manage::COLLECTION.find_active().await {
		view
	} else {
		view::manage::COLLECTION.get_lazy(None).await?
	};
	// FIXME: Despite this reveal, VS Code does not focus the webview hard enough for a .focus() in
	// the JS code to work.
	view.reveal(2, false);
	touch_input(view).await;
	Ok(())
}

async fn unused_test_id(dir: &Path) -> evscode::R<i64> {
	let mut taken = Vec::new();
	for test in fs::read_dir(dir).await? {
		if let Ok(id) = test.file_stem().parse::<i64>() {
			taken.push(id);
		}
	}
	let id = util::mex(1, taken);
	Ok(id)
}

pub async fn touch_input(webview: WebviewRef) {
	webview.post_message(view::manage::Food::NewStart).await;
}
