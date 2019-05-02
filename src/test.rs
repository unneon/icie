mod view;

use crate::{build, dir, util, STATUS};
use std::{
	fs, path::{Path, PathBuf}
};

#[derive(Debug)]
pub struct TestRun {
	in_path: PathBuf,
	out_path: PathBuf,
	outcome: ci::test::Outcome,
}

fn run() -> evscode::R<Vec<TestRun>> {
	let _status = STATUS.push("Testing");
	let solution = build::solution()?;
	let progress: evscode::ActiveProgress = evscode::Progress::new().title("Testing").show();
	let checker = ci::task::FreeWhitespaceChecker;
	let environment = ci::exec::Environment { time_limit: None };
	let task = ci::task::Task {
		checker: &checker,
		environment: &environment,
	};
	let test_dir = dir::tests();
	let ins = ci::scan::scan_and_order(&test_dir);
	let mut runs = Vec::new();
	let test_count = ins.len();
	for in_path in ins {
		let name = in_path.strip_prefix(&test_dir)?;
		let out_path = in_path.with_extension("out");
		let input = fs::read_to_string(&in_path)?;
		let output = fs::read_to_string(&out_path)?;
		let outcome = ci::test::simple_test(&solution, &input, Some(&output), &task)?;
		progress.update(
			100.0 / test_count as f64,
			format!("{} on `{}` in {}", outcome.verdict, name.display(), util::fmt_time_short(&outcome.time)),
		);
		let run = TestRun { in_path, out_path, outcome };
		runs.push(run)
	}
	Ok(runs)
}

#[evscode::command(title = "ICIE Open test view", key = "alt+0")]
fn view() -> evscode::R<()> {
	let _status = STATUS.push("Testing");
	let outcomes = run()?;
	let mut lck = view::WEBVIEW.lock()?;
	let webview = view::prepare_webview(&mut lck);
	webview.set_html(&view::render(&outcomes)?);
	webview.reveal(evscode::ViewColumn::Beside);
	Ok(())
}

fn add(input: &str, desired: &str) -> evscode::R<()> {
	let _status = STATUS.push("Adding new test");
	let tests = dir::tests().join("user");
	std::fs::create_dir_all(&tests)?;
	let id = unused_test_id(&tests)?;
	fs::write(tests.join(format!("{}.in", id)), input)?;
	fs::write(tests.join(format!("{}.out", id)), desired)?;
	view()?;
	Ok(())
}

#[evscode::command(title = "ICIE Input new test", key = "alt+-")]
fn input() -> evscode::R<()> {
	if !view::webview_exists()? {
		let outcomes = run()?;
		let mut lck = view::WEBVIEW.lock()?;
		let webview = view::prepare_webview(&mut lck);
		webview.set_html(&view::render(&outcomes)?);
		webview.reveal(evscode::ViewColumn::Beside);
		drop(lck);
		std::thread::sleep(std::time::Duration::from_millis(750));
	}
	let mut lck = view::WEBVIEW.lock()?;
	let webview = view::prepare_webview(&mut lck);
	webview.reveal(evscode::ViewColumn::Beside);
	webview.post_message(json::object! {
		"tag" => "new_start",
	});
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
