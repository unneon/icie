mod view;

use crate::{build, dir, STATUS};
use std::{fs, path::PathBuf};

#[derive(Debug)]
pub struct TestRun {
	in_path: PathBuf,
	out_path: PathBuf,
	outcome: ci::test::Outcome,
}

fn run() -> evscode::R<Vec<TestRun>> {
	let _status = STATUS.push("Testing");
	let solution = build::solution()?;
	let checker = ci::task::FreeWhitespaceChecker;
	let environment = ci::exec::Environment { time_limit: None };
	let task = ci::task::Task {
		checker: &checker,
		environment: &environment,
	};
	let ins = ci::scan::scan_and_order(&dir::tests());
	let mut runs = Vec::new();
	for in_path in ins {
		let out_path = in_path.with_extension("out");
		let input = fs::read_to_string(&in_path)?;
		let output = fs::read_to_string(&out_path)?;
		let outcome = ci::test::simple_test(&solution, &input, Some(&output), &task)?;
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
