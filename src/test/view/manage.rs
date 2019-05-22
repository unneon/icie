use crate::{
	test::{view::render::render, TestRun}, util
};
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

	fn impl_get(&self, source: Option<PathBuf>, updated: bool) -> evscode::R<(Arc<Mutex<View>>, Option<Vec<TestRun>>)> {
		let mut entries_lck = self.entries.lock()?;
		let (view, just_created) = match entries_lck.entry(source.clone()) {
			std::collections::hash_map::Entry::Occupied(e) => (e.get().clone(), false),
			std::collections::hash_map::Entry::Vacant(e) => (e.insert(Arc::new(Mutex::new(View::create(source.clone())))).clone(), true),
		};
		let lck = view.lock()?;
		drop(entries_lck);
		let runs = if just_created || updated { Some(lck.update()?) } else { None };
		lck.focus();
		drop(lck);
		Ok((view, runs))
	}

	pub fn force(&self, source: Option<PathBuf>) -> evscode::R<(Arc<Mutex<View>>, Vec<TestRun>)> {
		let (handle, runs) = self.impl_get(source, true)?;
		Ok((handle, runs.unwrap()))
	}

	pub fn tap(&self, source: Option<PathBuf>) -> evscode::R<(Arc<Mutex<View>>, bool)> {
		let (handle, runs) = self.impl_get(source, false)?;
		Ok((handle, runs.is_some()))
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
			evscode::spawn(move || {
				view.lock()?.update()?;
				Ok(())
			});
		}
		Ok(())
	}
}

#[evscode::config(description = "Auto-scroll to first failed test")]
static SCROLL_TO_FIRST_FAILED: evscode::Config<bool> = true;

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

	pub fn update(&self) -> evscode::R<Vec<TestRun>> {
		let runs = crate::test::run(self.source.as_ref().map(|p| p.as_path()))?;
		self.webview.set_html(render(&runs)?);
		if *SCROLL_TO_FIRST_FAILED.get() {
			self.webview.post_message(json::object! {
				"tag" => "scroll_to_wa",
			});
		}
		Ok(runs)
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
			Some("set_alt") => evscode::spawn({
				let key = key.clone();
				move || {
					let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
					let out = note["out"].as_str().unwrap();
					fs::write(in_path.with_extension("alt.out"), format!("{}\n", out.trim()))?;
					COLLECTION.force(key)?;
					Ok(())
				}
			}),
			Some("del_alt") => evscode::spawn({
				let key = key.clone();
				move || {
					let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
					fs::remove_file(in_path.with_extension("alt.out"))?;
					COLLECTION.force(key)?;
					Ok(())
				}
			}),
			_ => log::error!("unrecognied testview webview food `{}`", note.dump()),
		}
	}
	let mut lck = COLLECTION.entries.lock().unwrap();
	lck.remove(&key);
}
