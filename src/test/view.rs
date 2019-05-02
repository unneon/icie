use crate::{dir, test::TestRun};
use std::{
	fs, ops::{Deref, DerefMut}, sync::{Mutex, MutexGuard}
};

lazy_static::lazy_static! {
	pub static ref WEBVIEW: Mutex<Option<evscode::Webview>> = Mutex::new(None);
}

pub fn prepare_webview<'a>(lck: &'a mut MutexGuard<Option<evscode::Webview>>) -> &'a evscode::Webview {
	let requires_create = lck.as_ref().map(|webview| webview.was_disposed().wait()).unwrap_or(true);
	if requires_create {
		*MutexGuard::deref_mut(lck) = Some(
			evscode::Webview::new("icie.test.view", "ICIE Test view", evscode::ViewColumn::Beside)
				.retain_context_when_hidden()
				.create(),
		);
	}
	MutexGuard::deref(lck).as_ref().unwrap()
}

pub fn render(tests: &[TestRun]) -> evscode::R<String> {
	let css = include_str!("view.css");
	let test_table = render_test_table(tests)?;
	Ok(format!(
		r#"
		<html>
			<head>
				<style>
					{css}
				</style>
				<link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet">
			</head>
			<body>
				<table class="test-table">
					{test_table}
				</table>
			</body>
		</html>
	"#,
		css = css,
		test_table = test_table
	))
}

fn render_test_table(tests: &[TestRun]) -> evscode::R<String> {
	let mut html = String::new();
	for test in tests {
		html += &render_test(test)?;
	}
	Ok(html)
}

fn render_test(test: &TestRun) -> evscode::R<String> {
	use ci::test::Verdict::*;
	let raw_input = fs::read_to_string(&test.in_path)?;
	let lines_em = 1.1 * lines(&raw_input) as f64;
	let in_path = test.in_path.display();
	let name = test.in_path.strip_prefix(&dir::tests())?.display();
	let outcome_class = match test.outcome.verdict {
		Accepted => "test-good",
		WrongAnswer | RuntimeError | TimeLimitExceeded => "test-bad",
		IgnoredNoOut => "test-warn",
	};
	let out_note = render_out_note(&test.outcome.verdict);
	let input = html_escape(&raw_input);
	let output = html_escape(&test.outcome.out);
	let desired_cell = render_desired_cell(test)?;
	Ok(format!(
		r#"
		<tr class="test-row" data-in_path="{in_path}">
			<td style="height: {lines_em}em; line-height: 1.1em;" class="test-cell">
				<div class="test-actions">
					<div class="test-action material-icons" onclick="clipcopy()">file_copy</div>
					<div class="test-action material-icons" title={name}>info</div>
				</div>
				<div class="test-data">
					{input}
				</div>
			</td>
			<td class="test-cell {outcome_class}">
				<div class="test-actions">
					<div class="test-action material-icons" onclick="clipcopy()">file_copy</div>
					<div class="test-action material-icons" onclick="trigger_rr()">fast_rewind</div>
				</div>
				<div class="test-data">
					{output}
				</div>
				{out_note}
			</td>
			<td class="test-cell">
				{desired_cell}
			</td>
		</tr>
	"#,
		in_path = in_path,
		lines_em = lines_em,
		name = name,
		input = input,
		outcome_class = outcome_class,
		output = output,
		out_note = out_note,
		desired_cell = desired_cell
	))
}

fn render_desired_cell(test: &TestRun) -> evscode::R<String> {
	Ok(if test.out_path.exists() {
		let desired = html_escape(&fs::read_to_string(&test.out_path)?);
		format!(
			r#"
			<div class="test-actions">
				<div class="test-action material-icons" onclick="clipcopy()">file_copy</div>
			</div>
			<div class="test-data">
				{desired}
			</div>
		"#,
			desired = desired
		)
	} else {
		format!(
			r#"
			<div class="test-note">
				File does not exist
			</div>
		"#
		)
	})
}

fn render_out_note(verdict: &ci::test::Verdict) -> String {
	use ci::test::Verdict::*;
	let pretty = match verdict {
		Accepted | WrongAnswer => return String::new(),
		RuntimeError => "Runtime Error",
		TimeLimitExceeded => "Time Limit Exceeded",
		IgnoredNoOut => "Ignored",
	};
	format!(
		r#"
		<div class="test-note">
			{}
		</div>
	"#,
		pretty
	)
}

fn lines(s: &str) -> usize {
	s.trim().chars().filter(|c| char::is_whitespace(*c)).count()
}
fn html_escape(s: &str) -> String {
	s.replace("\n", "<br/>")
}
