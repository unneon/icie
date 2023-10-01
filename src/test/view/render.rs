use crate::{
	assets, test::{
		view::{SKILL_ACTIONS, SKILL_ADD}, TestRun, Verdict
	}, util, util::fs
};
use evscode::R;
use std::cmp::max;
use evscode::webview::WebviewRef;
struct Action {
	onclick: &'static str,
	icon: &'static str,
	hint: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, evscode::Configurable)]
enum HideBehaviour {
	#[evscode(name = "Always")]
	Always,
	#[evscode(name = "If any test failed")]
	IfAnyFailed,
	#[evscode(name = "Never")]
	Never,
}

const ACTION_COPY: Action = Action { onclick: "action_copy()", icon: "file_copy", hint: "Copy" };
const ACTION_EDIT: Action = Action { onclick: "action_edit()", icon: "edit", hint: "Edit" };
const ACTION_GDB: Action = Action { onclick: "action_gdb()", icon: "skip_previous", hint: "Debug in GDB" };
const ACTION_RR: Action = Action { onclick: "action_rr()", icon: "fast_rewind", hint: "Debug in RR" };
const ACTION_SET_ALT: Action = Action { onclick: "action_setalt()", icon: "check", hint: "Mark as correct" };
const ACTION_DEL_ALT: Action = Action { onclick: "action_delalt()", icon: "close", hint: "Unmark as correct" };

const MIN_CELL_LINES: i64 = 2;

/// This controls when to hide passing tests in test view by collapsing them into a thin color line. Even if this is not
/// set, any failing tests will still be visible if the icie.test.view.scrollToFirstFailed option is enabled(as is by
/// default).
#[evscode::config]
static FOLD_AC: evscode::Config<HideBehaviour> = HideBehaviour::Never;

/// This controls when to hide passing tests in test view by not displaying them at all. Even if this is not set, any
/// failing tests will still be visible if the icie.test.view.scrollToFirstFailed option is enabled(as is by default).
#[evscode::config]
static HIDE_AC: evscode::Config<HideBehaviour> = HideBehaviour::Never;

/// Whether to hide the "Copy" action in test view. Instead of using it, you can hover over the test cell and press
/// Ctrl+C; if nothing else is selected, the cell contents will be copied automatically.
#[evscode::config]
static HIDE_COPY: evscode::Config<bool> = false;

/// The maximum height of a test case, expressed in pixels. If the test case would take up more than that, it will be
/// clipped. The full test case can be seen by scrolling. Leave empty to denote no limit.
#[evscode::config]
static MAX_TEST_HEIGHT: evscode::Config<Option<u64>> = 720;

/// If a solution takes longer to execute than the specified number of milliseconds, a note with the execution duration
/// will be displayed. Set to 0 to always display the timings, or to a large value to never display the timings.
#[evscode::config]
static TIME_DISPLAY_THRESHOLD: evscode::Config<u64> = 100u64;

pub async fn render(tests: &[TestRun],webview:WebviewRef) -> R<String> {
	Ok(format!(
		r#"
		<html>
			<head>
				{js}
				{material_icons}
				{css_layout}
				{css_paint}
			</head>
			<body>
				<table class="table">
					{table}
				</table>
				{new_test}
			</body>
		</html>
		"#,
		js = assets::html_js_dynamic(webview.clone(),"script_view.js"),
		material_icons = assets::html_material_icons(webview.clone()),
		css_layout = assets::html_css_dynamic(include_str!("layout.css")),
		css_paint = assets::html_css_dynamic(include_str!("paint.css")),
		table = render_test_table(tests).await?,
		new_test = render_new_test().await,
	))
}

async fn render_test_table(tests: &[TestRun]) -> R<String> {
	let any_failed = tests.iter().any(|test| !test.success());
	let mut html = String::new();
	for test in tests {
		html += &render_test(test, any_failed).await?;
	}
	Ok(html)
}

async fn render_test(test: &TestRun, any_failed: bool) -> R<String> {
	if test.success() && HIDE_AC.get().should(any_failed) {
		return Ok(String::new());
	}
	let folded = test.success() && FOLD_AC.get().should(any_failed);
	Ok(format!(
		r#"
		<tr class="row {status} {verdict}" data-path_in="{path_in}" data-raw_out="{raw_out}">
			{input}
			{output}
			{desired}
		</tr>
		"#,
		status = match test.outcome.verdict {
			Verdict::Accepted { .. } => "status-passed",
			Verdict::WrongAnswer | Verdict::RuntimeError | Verdict::TimeLimitExceeded => "status-failed",
			Verdict::IgnoredNoOut => "status-ignore",
		},
		verdict = match test.outcome.verdict {
			Verdict::Accepted { alternative: false } => "verdict-accept",
			Verdict::Accepted { alternative: true } => "verdict-alternative",
			Verdict::WrongAnswer => "verdict-wrong-answer",
			Verdict::RuntimeError => "verdict-runtime-error",
			Verdict::TimeLimitExceeded => "verdict-time-limit-exceeded",
			Verdict::IgnoredNoOut => "verdict-ignored",
		},
		path_in = html_escape(test.in_path.as_str()),
		raw_out = html_escape(&test.outcome.out),
		input = render_in_cell(test, folded).await?,
		output = render_out_cell(test, folded).await?,
		desired = render_desired_cell(test, folded).await?,
	))
}

async fn render_in_cell(test: &TestRun, folded: bool) -> R<String> {
	let data = fs::read_to_string(&test.in_path).await?;
	let attrs = [("data-raw", data.as_str())];
	let actions = [(!HIDE_COPY.get(), ACTION_COPY), (true, ACTION_EDIT)];
	Ok(render_cell("input", &attrs, &actions, None, &data, None, folded).await)
}

async fn render_out_cell(test: &TestRun, folded: bool) -> R<String> {
	let note_time = prepare_time_note(test);
	let note_verdict = match test.outcome.verdict {
		Verdict::Accepted { .. } | Verdict::WrongAnswer | Verdict::IgnoredNoOut => None,
		Verdict::RuntimeError => Some("RE"),
		Verdict::TimeLimitExceeded => Some("TLE"),
	};
	let notes = vec![note_time.as_deref(), note_verdict].into_iter().flatten().collect::<Vec<_>>();
	let note = if notes.is_empty() { None } else { Some(notes.join("\n")) };
	let attrs = [("data-raw", test.outcome.out.as_str())];
	let actions = [
		(!HIDE_COPY.get(), ACTION_COPY),
		(test.outcome.verdict == Verdict::WrongAnswer, ACTION_SET_ALT),
		(test.outcome.verdict == Verdict::Accepted { alternative: true }, ACTION_DEL_ALT),
		(true, ACTION_GDB),
		(true, ACTION_RR),
	];
	Ok(render_cell(
		"output",
		&attrs,
		&actions,
		Some(test.outcome.stderr.as_str()),
		&test.outcome.out,
		note.as_deref(),
		folded,
	)
	.await)
}

fn prepare_time_note(test: &TestRun) -> Option<String> {
	if test.outcome.time.as_millis() >= u128::from(TIME_DISPLAY_THRESHOLD.get())
		|| test.outcome.verdict == Verdict::TimeLimitExceeded
	{
		Some(util::fmt::time(&test.outcome.time))
	} else {
		None
	}
}

async fn render_desired_cell(test: &TestRun, folded: bool) -> R<String> {
	let data = fs::read_to_string(&test.out_path).await.unwrap_or_default();
	let attrs = [("data-raw", data.as_str())];
	let actions =
		[(test.outcome.verdict != Verdict::IgnoredNoOut && !HIDE_COPY.get(), ACTION_COPY), (true, ACTION_EDIT)];
	Ok(render_cell("desired", &attrs, &actions, None, &data, None, folded).await)
}

async fn render_cell(
	class: &str,
	attrs: &[(&str, &str)],
	actions: &[(bool, Action)],
	stderr: Option<&str>,
	stdout: &str,
	note: Option<&str>,
	folded: bool,
) -> String {
	if !folded {
		render_cell_raw(class, attrs, actions, stderr, stdout, note).await
	} else {
		let class = format!("{} folded", class);
		render_cell_raw(&class, attrs, &[], None, "", None).await
	}
}

async fn render_cell_raw(
	class: &str,
	attrs: &[(&str, &str)],
	actions: &[(bool, Action)],
	stderr: Option<&str>,
	stdout: &str,
	note: Option<&str>,
) -> String {
	let actions = render_actions(actions).await;
	let note = match note {
		Some(note) => format!("<div class=\"note\">{}</div>", html_escape(note)),
		None => String::new(),
	};
	let lines = (stderr.as_ref().map_or(0, |stderr| count_lines(stderr)) + count_lines(stdout)) as i64;
	let stderr = match stderr {
		Some(stderr) => format!("<div class=\"stderr\">{}</div>", html_escape_spaced(stderr.trim())),
		None => String::new(),
	};
	let newline_fill = (0..max(MIN_CELL_LINES - lines + 1, 0)).map(|_| "<br/>").collect::<String>();
	let max_test_height = MAX_TEST_HEIGHT.get();
	let max_test_height = if let Some(max_test_height) = max_test_height {
		format!("style=\"max-height: {}px;\"", max_test_height)
	} else {
		String::new()
	};
	let mut attr_html = String::new();
	for (k, v) in attrs {
		attr_html += &format!(" {}=\"{}\"", k, html_escape(v));
	}
	let data = format!(
		"<div class=\"data\" {}>{}{}{}</div>",
		max_test_height,
		stderr,
		html_escape_spaced(stdout.trim()),
		newline_fill
	);
	format!("<td class=\"cell {}\" {}>{}{}{}</td>", class, attr_html, actions, note, data)
}

async fn render_actions(actions: &[(bool, Action)]) -> String {
	let buttons = actions.iter().filter(|action| action.0).map(|action| render_action(&action.1)).collect::<Vec<_>>();
	format!(
		"<div class=\"actions {}\">{}</div>",
		if !SKILL_ACTIONS.is_proficient().await { "tutorialize" } else { "" },
		buttons.join("\n")
	)
}

fn render_action(action: &Action) -> String {
	format!(
		"<div class=\"material-icons action\" onclick=\"{}\" title=\"{}\">{}</div>",
		action.onclick, action.hint, action.icon
	)
}

async fn render_new_test() -> String {
	let first = if !SKILL_ADD.is_proficient().await {
		"<p class=\"new-tutorial new-tutorial-start\">Press <kbd>Alt</kbd><kbd>-</kbd> to add a new test.</p>"
	} else {
		""
	};
	let instruction = if !SKILL_ADD.is_proficient().await {
		"<p class=\"new-tutorial\">... and press <kbd>Alt</kbd><kbd>-</kbd> to finish adding the test.</p>"
	} else {
		""
	};
	format!(
		r#"
		{first}
		<div class="new">
			<div class="new-areas">
				{input_area}
				{output_area}
			</div>
			{instruction}
		</div>
"#,
		first = first,
		input_area = render_new_test_area("new-input", "Write test input here...").await,
		output_area = render_new_test_area("new-desired", "Write test output here...").await,
		instruction = instruction,
	)
}

async fn render_new_test_area(id: &str, hint: &str) -> String {
	let placeholder =
		if !SKILL_ADD.is_proficient().await { format!("placeholder=\"{}\"", hint) } else { String::new() };
	format!("<textarea id=\"{}\" class=\"new-area\" {}></textarea>", id, placeholder)
}

impl HideBehaviour {
	fn should(&self, any_failed: bool) -> bool {
		match self {
			HideBehaviour::Always => true,
			HideBehaviour::IfAnyFailed => any_failed,
			HideBehaviour::Never => false,
		}
	}
}

fn count_lines(s: &str) -> usize {
	if !s.trim().is_empty() { s.trim().matches('\n').count() + 1 } else { 0 }
}

fn html_escape(s: &str) -> String {
	translate(s, &[('&', "&amp;"), ('<', "&lt;"), ('>', "&gt;"), ('"', "&quot;"), ('\'', "&#39;")])
}

fn html_escape_spaced(s: &str) -> String {
	translate(s, &[('&', "&amp;"), ('<', "&lt;"), ('>', "&gt;"), ('"', "&quot;"), ('\'', "&#39;"), ('\n', "<br/>")])
}

fn translate(s: &str, table: &[(char, &str)]) -> String {
	let mut buf = String::new();
	for c in s.chars() {
		match table.iter().find(|rule| rule.0 == c) {
			Some(rule) => buf += rule.1,
			_ => buf.push(c),
		}
	}
	buf
}
