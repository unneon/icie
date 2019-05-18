mod view;

use crate::{build, dir, util, STATUS};
use std::{
	fs, path::{Path, PathBuf}
};

#[derive(Clone, Debug)]
pub struct TestRun {
	in_path: PathBuf,
	out_path: PathBuf,
	outcome: ci::test::Outcome,
}

pub fn run(main_source: Option<&Path>) -> evscode::R<Vec<TestRun>> {
	let _status = STATUS.push("Testing");
	let solution = build::build(main_source, ci::lang::Codegen::Debug)?;
	let task = ci::task::Task {
		checker: Box::new(ci::task::FreeWhitespaceChecker),
		environment: ci::exec::Environment { time_limit: None },
	};
	let test_dir = dir::tests();
	let ins = ci::scan::scan_and_order(&test_dir);
	let mut runs = Vec::new();
	let test_count = ins.len();
	let progress: evscode::ActiveProgress = evscode::Progress::new().title(util::fmt_verb("Testing", &main_source)).cancellable().show();
	let worker = run_thread(ins, task, solution).cancel_on(progress.canceler());
	for _ in 0..test_count {
		let run = worker.wait()??;
		let name = run.in_path.strip_prefix(&test_dir)?;
		progress.update(
			100.0 / test_count as f64,
			format!("{} on `{}` in {}", run.outcome.verdict, name.display(), util::fmt_time_short(&run.outcome.time)),
		);
		runs.push(run);
	}
	Ok(runs)
}

fn run_thread(ins: Vec<PathBuf>, task: ci::task::Task, solution: ci::exec::Executable) -> evscode::Future<evscode::R<TestRun>> {
	evscode::LazyFuture::new_worker(move |carrier| {
		let _status = STATUS.push("Executing");
		for in_path in ins {
			let out_path = in_path.with_extension("out");
			let input = fs::read_to_string(&in_path)?;
			let output = fs::read_to_string(&out_path)?;
			let outcome = ci::test::simple_test(&solution, &input, Some(&output), &task)?;
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
pub fn view() -> evscode::R<()> {
	view::COLLECTION.get(None, true)?;
	Ok(())
}

#[evscode::command(title = "ICIE Open Test View (current editor)", key = "alt+\\ alt+0")]
fn view_current() -> evscode::R<()> {
	view::COLLECTION.get(util::active_tab()?, true)?;
	Ok(())
}

fn add(input: &str, desired: &str) -> evscode::R<()> {
	let tests = dir::tests().join("user");
	std::fs::create_dir_all(&tests)?;
	let id = unused_test_id(&tests)?;
	fs::write(tests.join(format!("{}.in", id)), input)?;
	fs::write(tests.join(format!("{}.out", id)), desired)?;
	view::COLLECTION.update_all()?;
	Ok(())
}

#[evscode::command(title = "ICIE New Test", key = "alt+-")]
fn input() -> evscode::R<()> {
	if let Some(view) = view::COLLECTION.find_active()? {
		view.lock()?.touch_input();
	} else {
		let (view, just_created) = view::COLLECTION.get(None, false)?;
		if just_created {
			std::thread::sleep(std::time::Duration::from_millis(100));
		}
		view.lock()?.touch_input();
	}
	Ok(())
}

fn unused_test_id(dir: &Path) -> evscode::R<i64> {
	let mut taken = std::collections::HashSet::new();
	for test in dir.read_dir()? {
		let test = test?;
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
