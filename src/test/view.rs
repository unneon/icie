use crate::{test::TestRun, util};
use std::{
	collections::HashMap, fs, path::PathBuf, sync::{Arc, Mutex}
};

lazy_static::lazy_static! {
	pub static ref COLLECTION: Collection = Collection::new();
}
pub struct Collection {
	entries: Mutex<HashMap<Option<PathBuf>, Arc<Mutex<View>>>>,
}
pub struct View {
	webview: evscode::Webview,
	source: Option<PathBuf>,
}

impl Collection {
	fn new() -> Collection {
		Collection {
			entries: Mutex::new(HashMap::new()),
		}
	}

	pub fn get(&self, source: Option<PathBuf>, updated: bool) -> evscode::R<(Arc<Mutex<View>>, bool)> {
		let mut entries_lck = self.entries.lock()?;
		let (view, just_created) = match entries_lck.entry(source.clone()) {
			std::collections::hash_map::Entry::Occupied(e) => (e.get().clone(), false),
			std::collections::hash_map::Entry::Vacant(e) => (e.insert(Arc::new(Mutex::new(View::create(source.clone())))).clone(), true),
		};
		let lck = view.lock()?;
		drop(entries_lck);
		if just_created || updated {
			lck.update()?;
		}
		lck.focus();
		drop(lck);
		Ok((view, just_created))
	}

	pub fn find_active(&self) -> evscode::R<Option<Arc<Mutex<View>>>> {
		let lck = self.entries.lock()?;
		for view in lck.values() {
			if view.lock()?.is_active().wait() {
				return Ok(Some(view.clone()));
			}
		}
		Ok(None)
	}

	pub fn update_all(&self) -> evscode::R<()> {
		let lck = self.entries.lock()?;
		for view in lck.values() {
			let view = view.clone();
			evscode::spawn(move || Ok(view.lock()?.update()?));
		}
		Ok(())
	}
}
impl View {
	pub fn create(source: Option<PathBuf>) -> View {
		let title = util::fmt_verb("ICIE Test View", &source);
		let webview: evscode::Webview = evscode::Webview::new("icie.test.view", title, evscode::webview::Column::Beside)
			.enable_scripts()
			.retain_context_when_hidden()
			.create();
		let stream = webview.listener().spawn().cancel_on(webview.disposer());
		let source2 = source.clone();
		evscode::spawn(move || Ok(handle_events(source2, stream)));
		View { webview, source }
	}

	pub fn touch_input(&self) {
		self.webview.post_message(json::object! {
			"tag" => "new_start",
		});
	}

	pub fn update(&self) -> evscode::R<()> {
		let runs = crate::test::run(self.source.as_ref().map(|p| p.as_path()))?;
		self.webview.set_html(render(&runs)?);
		if *SCROLL_TO_FIRST_FAILED.get() {
			self.webview.post_message(json::object! {
				"tag" => "scroll_to_wa",
			});
		}
		Ok(())
	}

	pub fn focus(&self) {
		self.webview.reveal(evscode::webview::Column::Beside);
	}

	pub fn is_active(&self) -> evscode::Future<bool> {
		self.webview.is_active().spawn()
	}
}

fn handle_events(key: Option<PathBuf>, stream: evscode::Future<evscode::future::Cancellable<json::JsonValue>>) {
	for note in stream {
		match note["tag"].as_str() {
			Some("trigger_rr") => evscode::spawn({
				let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
				let key = key.clone();
				move || crate::debug::rr(in_path, key)
			}),
			Some("trigger_gdb") => evscode::spawn({
				let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
				let key = key.clone();
				move || crate::debug::gdb(in_path, key)
			}),
			Some("new_test") => evscode::spawn(move || crate::test::add(note["input"].as_str().unwrap(), note["desired"].as_str().unwrap())),
			_ => log::error!("unrecognied testview webview food `{}`", note.dump()),
		}
	}
	let mut lck = COLLECTION.entries.lock().unwrap();
	lck.remove(&key);
}

#[derive(Clone, Debug, evscode::Configurable)]
enum HideBehaviour {
	#[evscode(name = "Always")]
	Always,
	#[evscode(name = "If any test failed")]
	IfAnyFailed,
	#[evscode(name = "Never")]
	Never,
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

#[evscode::config(description = "Fold AC in test view")]
static FOLD_AC: evscode::Config<HideBehaviour> = HideBehaviour::Never;

#[evscode::config(description = "Hide AC in test view")]
static HIDE_AC: evscode::Config<HideBehaviour> = HideBehaviour::Never;

#[evscode::config(description = "Auto-scroll to first failed test")]
static SCROLL_TO_FIRST_FAILED: evscode::Config<bool> = true;

pub fn render(tests: &[TestRun]) -> evscode::R<String> {
	Ok(format!(
		r#"
		<html>
			<head>
				<style>{css}</style>
				{material_icons}
				<script>{js}</script>
			</head>
			<body>
				<table class="test-table">
					{test_table}
				</table>
				<br/>
				<div id="new-container" class="new">
					<textarea id="new-input" class="new"></textarea>
					<textarea id="new-desired" class="new"></textarea>
					<div id="new-start" class="material-icons button new" onclick="new_start()">add</div>
					<div id="new-confirm" class="material-icons button new" onclick="new_confirm()">done</div>
				</div>
			</body>
		</html>
	"#,
		css = include_str!("view.css"),
		material_icons = util::html_material_icons(),
		js = include_str!("view.js"),
		test_table = render_test_table(tests)?
	))
}

fn render_test_table(tests: &[TestRun]) -> evscode::R<String> {
	let any_failed = tests.iter().any(|test| test.outcome.verdict != ci::test::Verdict::Accepted);
	let mut html = String::new();
	for test in tests {
		html += &render_test(test, any_failed)?;
	}
	Ok(html)
}

fn render_test(test: &TestRun, any_failed: bool) -> evscode::R<String> {
	if test.outcome.verdict == ci::test::Verdict::Accepted && HIDE_AC.get().should(any_failed) {
		return Ok(String::new());
	}
	let folded = test.outcome.verdict == ci::test::Verdict::Accepted && FOLD_AC.get().should(any_failed);
	Ok(format!(
		r#"
		<tr class="test-row {failed_flag}" data-in_path="{in_path}">
			{input}
			{out}
			{desired}
		</tr>
	"#,
		failed_flag = if test.outcome.verdict != ci::test::Verdict::Accepted { "test-row-failed" } else { "" },
		in_path = test.in_path.display(),
		input = render_in_cell(test, folded)?,
		out = render_out_cell(test, folded)?,
		desired = render_desired_cell(test, folded)?
	))
}

fn render_in_cell(test: &TestRun, folded: bool) -> evscode::R<String> {
	Ok(render_cell("", &[ACTION_COPY], &fs::read_to_string(&test.in_path)?, None, folded))
}

fn render_out_cell(test: &TestRun, folded: bool) -> evscode::R<String> {
	use ci::test::Verdict::*;
	let outcome_class = match test.outcome.verdict {
		Accepted => "test-good",
		WrongAnswer | RuntimeError | TimeLimitExceeded => "test-bad",
		IgnoredNoOut => "test-warn",
	};
	let note = match test.outcome.verdict {
		Accepted | WrongAnswer => None,
		RuntimeError => Some("Runtime Error"),
		TimeLimitExceeded => Some("Time Limit Exceeded"),
		IgnoredNoOut => Some("Ignored"),
	};
	Ok(render_cell(outcome_class, &[ACTION_COPY, ACTION_GDB, ACTION_RR], &test.outcome.out, note, folded))
}

fn render_desired_cell(test: &TestRun, folded: bool) -> evscode::R<String> {
	Ok(if test.out_path.exists() {
		render_cell("", &[ACTION_COPY], &fs::read_to_string(&test.out_path)?, None, folded)
	} else {
		render_cell("", &[], "", Some("File does not exist"), folded)
	})
}

struct Action {
	onclick: &'static str,
	icon: &'static str,
}
const ACTION_COPY: Action = Action {
	onclick: "clipcopy()",
	icon: "file_copy",
};
const ACTION_GDB: Action = Action {
	onclick: "trigger_gdb()",
	icon: "skip_previous",
};
const ACTION_RR: Action = Action {
	onclick: "trigger_rr()",
	icon: "fast_rewind",
};

fn render_cell(class: &str, actions: &[Action], data: &str, note: Option<&str>, folded: bool) -> String {
	if folded {
		return format!(
			r#"
			<td class="test-cell {class} folded">
			</td>
		"#,
			class = class
		);
	}
	let note_div = if let Some(note) = note {
		format!(r#"<div class="test-note">{note}</div>"#, note = note)
	} else {
		format!("")
	};
	let mut action_list = String::new();
	for action in actions {
		action_list += &format!(r#"<div class="test-action material-icons" onclick="{}">{}</div>"#, action.onclick, action.icon);
	}
	format!(
		r#"
		<td style="height: {lines_em}em; line-height: 1.1em;" class="test-cell {class}">
			<div class="test-actions">
				{action_list}
			</div>
			<div class="test-data">
				{data}
			</div>
			{note_div}
		</td>
	"#,
		lines_em = 1.1 * lines(data) as f64,
		class = class,
		action_list = action_list,
		data = html_escape(data.trim()),
		note_div = note_div
	)
}

fn lines(s: &str) -> usize {
	s.trim().chars().filter(|c| char::is_whitespace(*c)).count()
}
fn html_escape(s: &str) -> String {
	translate(s, &[('\n', "<br/>"), ('&', "&amp;"), ('<', "&lt;"), ('>', "&gt;"), ('"', "&quot;"), ('\'', "&#39;")])
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
