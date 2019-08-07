pub mod view;

use crate::{build, ci, dir, util, STATUS};
use evscode::{E, R};
use std::{
	path::{Path, PathBuf}, time::Duration
};

#[derive(Debug)]
pub struct TestRun {
	in_path: PathBuf,
	out_path: PathBuf,
	outcome: ci::test::Outcome,
}
impl TestRun {
	pub fn success(&self) -> bool {
		self.outcome.success()
	}
}

/// The maximum time an executable can run before getting a Time Limit Exceeded verdict, specified in milliseconds. Leaving this empty(which denotes no limit) is not recommended, because this will cause stuck processes to run indefinitely, wasting system resources.
#[evscode::config]
static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);

pub fn run(main_source: &Option<PathBuf>) -> R<Vec<TestRun>> {
	let _status = STATUS.push("Testing");
	let solution = build::build(main_source, &ci::cpp::Codegen::Debug)?;
	let task = ci::task::Task { checker: crate::checker::get_checker()?, environment: ci::exec::Environment { time_limit: time_limit() } };
	let test_dir = dir::tests()?;
	let ins = ci::scan::scan_and_order(&test_dir);
	let mut runs = Vec::new();
	let test_count = ins.len();
	let progress = evscode::Progress::new().title(util::fmt_verb("Testing", &main_source)).cancellable().show();
	let worker = run_thread(ins, task, solution).cancel_on(progress.canceler());
	for _ in 0..test_count {
		let run = worker.wait()??;
		let name = run.in_path.strip_prefix(&test_dir).map_err(|e| E::from_std(e).context("found test outside of test directory"))?;
		progress.update_inc(
			100.0 / test_count as f64,
			format!("{} on `{}` in {}", run.outcome.verdict, name.display(), util::fmt_time_short(&run.outcome.time)),
		);
		runs.push(run);
	}
	Ok(runs)
}

pub fn time_limit() -> Option<Duration> {
	TIME_LIMIT.get().map(|ms| Duration::from_millis(ms as u64))
}

fn run_thread(ins: Vec<PathBuf>, task: ci::task::Task, solution: ci::exec::Executable) -> evscode::Future<R<TestRun>> {
	evscode::LazyFuture::new_worker(move |carrier| {
		let _status = STATUS.push("Executing");
		for in_path in ins {
			let out_path = in_path.with_extension("out");
			let alt_path = in_path.with_extension("alt.out");
			let input = util::fs_read_to_string(&in_path)?;
			let output = match std::fs::read_to_string(&out_path) {
				Ok(output) => Some(output),
				Err(e) => {
					if e.kind() == std::io::ErrorKind::NotFound {
						None
					} else {
						return Err(E::from_std(e).context(format!("failed to read test out {}", out_path.display())));
					}
				},
			};
			let alt = if alt_path.exists() { Some(util::fs_read_to_string(&alt_path)?) } else { None };
			let outcome = ci::test::simple_test(&solution, &input, output.as_ref().map(String::as_str), alt.as_ref().map(|p| p.as_str()), &task)
				.map_err(|e| e.context("failed to run test"))?;
			let run = TestRun { in_path, out_path, outcome };
			if !carrier.send(run) {
				break;
			}
		}
		Ok(())
	})
	.spawn()
}

#[evscode::command(title = "ICIE Open Test View", key = "alt+0")]
pub fn view() -> R<()> {
	view::manage::COLLECTION.get_force(None)?;
	Ok(())
}

#[evscode::command(title = "ICIE Open Test View (current editor)", key = "alt+\\ alt+0")]
fn view_current() -> R<()> {
	view::manage::COLLECTION.get_force(util::active_tab()?)?;
	Ok(())
}

fn add(input: &str, desired: &str) -> evscode::R<()> {
	let tests = dir::custom_tests()?;
	util::fs_create_dir_all(&tests)?;
	let id = unused_test_id(&tests)?;
	util::fs_write(tests.join(format!("{}.in", id)), input)?;
	util::fs_write(tests.join(format!("{}.out", id)), desired)?;
	view::manage::COLLECTION.update_all();
	Ok(())
}

#[evscode::command(title = "ICIE New Test", key = "alt+-")]
fn input() -> evscode::R<()> {
	let view = if let Some(view) = view::manage::COLLECTION.find_active() { view } else { view::manage::COLLECTION.get_lazy(None)? };
	view::manage::touch_input(&*view.lock().unwrap());
	Ok(())
}

fn unused_test_id(dir: &Path) -> evscode::R<i64> {
	let mut taken = std::collections::HashSet::new();
	for test in dir.read_dir().map_err(|e| E::from_std(e).context("failed to read tests directory"))? {
		let test = test.map_err(|e| E::from_std(e).context("failed to read a test file entry in tests directory"))?;
		if let Ok(id) = test.path().file_stem().unwrap().to_str().unwrap().parse::<i64>() {
			taken.insert(id);
		}
	}
	let mut id = 1;
	while taken.contains(&id) {
		id += 1;
	}
	Ok(id)
}
