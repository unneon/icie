use crate::{
	test::{
		self, view::{render::render, SCROLL_TO_FIRST_FAILED, SKILL_ACTIONS}, TestRun
	}, util
};
use evscode::{
	goodies::{webview_resultmap::Computation, WebviewHandle}, Webview, WebviewResultmap, E, R
};
use std::{fs, path::PathBuf};

lazy_static::lazy_static! {
	pub static ref COLLECTION: WebviewResultmap<TestViewLogic> = WebviewResultmap::new(TestViewLogic);
}

pub fn touch_input(webview: &Webview) {
	webview.post_message(json::object! {
		"tag" => "new_start",
	});
}

pub struct TestViewLogic;

impl Computation for TestViewLogic {
	type K = Option<PathBuf>;
	type V = Report;

	fn compute(&self, source: &Option<PathBuf>) -> R<Report> {
		Ok(Report { runs: test::run(source)? })
	}

	fn create_empty_webview(&self, source: &Option<PathBuf>) -> R<Webview> {
		let title = util::fmt_verb("ICIE Test View", &source);
		let webview = evscode::Webview::new("icie.test.view", title, 2).enable_scripts().retain_context_when_hidden().create();
		Ok(webview)
	}

	fn update(&self, _: &Option<PathBuf>, report: &Report, webview: &Webview) -> R<()> {
		webview.set_html(render(&report.runs)?);
		webview.reveal(2);
		if *SCROLL_TO_FIRST_FAILED.get() {
			webview.post_message(json::object! {
				"tag" => "scroll_to_wa",
			});
		}
		Ok(())
	}

	fn manage(&self, source: &Option<PathBuf>, _: &Report, webview: WebviewHandle) -> R<Box<dyn FnOnce()+Send+'static>> {
		let webview = webview.lock().unwrap();
		let stream = webview.listener().spawn().cancel_on(webview.disposer());
		let source = source.clone();
		Ok(Box::new(move || {
			for note in stream {
				match note["tag"].as_str() {
					Some("trigger_rr") => evscode::runtime::spawn({
						let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
						let source = source.clone();
						move || crate::debug::rr(in_path, source)
					}),
					Some("trigger_gdb") => evscode::runtime::spawn({
						let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
						let source = source.clone();
						move || crate::debug::gdb(in_path, source)
					}),
					Some("new_test") => evscode::runtime::spawn(move || crate::test::add(note["input"].as_str().unwrap(), note["desired"].as_str().unwrap())),
					Some("set_alt") => evscode::runtime::spawn({
						let source = source.clone();
						move || {
							let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
							let out = note["out"].as_str().unwrap();
							fs::write(in_path.with_extension("alt.out"), format!("{}\n", out.trim()))
								.map_err(|e| E::from_std(e).context("failed to save alternative out as a file"))?;
							COLLECTION.get_force(source)?;
							Ok(())
						}
					}),
					Some("del_alt") => evscode::runtime::spawn({
						let source = source.clone();
						move || {
							let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
							fs::remove_file(in_path.with_extension("alt.out")).map_err(|e| E::from_std(e).context("failed to remove alternative out file"))?;
							COLLECTION.get_force(source)?;
							Ok(())
						}
					}),
					Some("edit") => evscode::open_editor(note["path"].as_str().unwrap(), None, None),
					Some("action_notice") => evscode::runtime::spawn(|| Ok(SKILL_ACTIONS.add_use())),
					_ => log::error!("unrecognied testview webview food `{}`", note.dump()),
				}
			}
		}))
	}
}

pub struct Report {
	pub runs: Vec<TestRun>,
}
