use crate::{dir, util::path::Path};
use evscode::{error::ResultExt, Position, E, R};
use futures::{channel::oneshot, future::join_all};
use std::{
	future::Future, sync::Arc, time::{Duration, SystemTime}
};
pub use tempfile::Tempfile;
use wasm_bindgen::{closure::Closure, JsValue};

pub mod fs;
pub mod letter_case;
pub mod path;
pub mod retries;
pub mod tempfile;

pub fn fmt_time_short(t: &Duration) -> String {
	let s = t.as_secs();
	let ms = t.as_millis() % 1000;
	format!("{}.{:03}s", s, ms)
}

pub fn fmt_time_left(mut t: Duration) -> String {
	let mut s = {
		let x = t.as_secs() % 60;
		t -= Duration::from_secs(x);
		format!("{} left", plural(x as usize, "second", "seconds"))
	};
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 % 60;
		t -= Duration::from_secs(x * 60);
		s = format!("{}, {}", plural(x as usize, "minute", "minutes"), s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 % 24;
		t -= Duration::from_secs(x * 60 * 60);
		s = format!("{}, {}", plural(x as usize, "hour", "hours"), s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 / 24;
		t -= Duration::from_secs(x * 60 * 60 * 24);
		s = format!("{}, {}", plural(x as usize, "day", "days"), s)
	}
	s
}

#[test]
fn test_fmt_time() {
	assert_eq!(fmt_time_short(&Duration::from_millis(2137)), "2.137s");
	assert_eq!(fmt_time_short(&Duration::from_millis(42)), "0.042s");
}

pub fn fmt_verb(verb: &'static str, source: &SourceTarget) -> String {
	match source {
		SourceTarget::Custom(source) => format!("{} {}", verb, source.fmt_workspace()),
		SourceTarget::Main => verb.to_owned(),
	}
}

pub async fn active_tab() -> R<SourceTarget> {
	let source = Path::from_native(evscode::active_editor_file().await.ok_or_else(E::cancel)?);
	Ok(if source != crate::dir::solution()? { SourceTarget::Custom(source) } else { SourceTarget::Main })
}

pub fn bash_escape(raw: &str) -> String {
	let mut escaped = String::from("\"");
	for c in raw.chars() {
		match c {
			'"' => escaped += "\\\"",
			'\\' => escaped += "\\\\",
			'$' => escaped += "\\$",
			'!' => escaped += "\\!",
			c => escaped.push(c),
		};
	}
	escaped += "\"";
	escaped
}

#[test]
fn test_bash_escape() {
	assert_eq!(bash_escape("\"Hello, world!\""), r#""\"Hello, world\!\"""#);
	assert_eq!(bash_escape("${HOME}\\Projects"), r#""\${HOME}\\Projects""#);
}

pub async fn is_installed(app: &str) -> R<bool> {
	Ok(find_app(app).await?.is_some())
}

pub async fn find_app(app: &str) -> R<Option<Path>> {
	let exec_lookups = env("PATH")?;
	for exec_lookup in exec_lookups.split(&String::from(node_sys::path::DELIMITER.clone())) {
		let path = Path::from_native(exec_lookup.to_owned()).join(app);
		if fs::exists(&path).await? {
			return Ok(Some(path));
		}
	}
	Ok(None)
}

pub fn env(key: &'static str) -> R<String> {
	Ok(js_sys::Reflect::get(&node_sys::process::ENV, &JsValue::from_str(key))
		.ok()
		.wrap(format!("env var {} does not exist", key))?
		.as_string()
		.unwrap())
}

pub fn html_material_icons() -> String {
	match OS::query() {
		// For whatever reason, bundled icons do not display on Windows.
		// I made sure the paths are correct and fully-backslashed, but to no avail.
		Ok(OS::Windows) => material_icons_cloud(),
		_ => material_icons_bundled(),
	}
}

pub fn material_icons_cloud() -> String {
	r#"<link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet">"#.to_owned()
}

pub fn material_icons_bundled() -> String {
	format!(
		r#"
		<style>
			@font-face {{
				font-family: 'Material Icons';
				font-style: normal;
				font-weight: 400;
				src: url({woff2_asset}) format('woff2');
			}}

			.material-icons {{
				font-family: 'Material Icons';
				font-weight: normal;
				font-style: normal;
				font-size: 24px;
				line-height: 1;
				letter-spacing: normal;
				text-transform: none;
				display: inline-block;
				white-space: nowrap;
				word-wrap: normal;
				direction: ltr;
				-webkit-font-feature-settings: 'liga';
				-webkit-font-smoothing: antialiased;
			}}
		</style>
	"#,
		woff2_asset = evscode::asset("material-icons.woff2")
	)
}

pub fn mex(x0: i64, mut xs: Vec<i64>) -> i64 {
	xs.sort();
	xs.dedup();
	for (i, x) in xs.iter().enumerate() {
		if x0 + i as i64 != *x {
			return x0 + i as i64;
		}
	}
	x0 + xs.len() as i64
}

#[test]
fn test_mex() {
	assert_eq!(mex(0, vec![5, 3, 2, 0, 1]), 4);
	assert_eq!(mex(0, vec![]), 0);
	assert_eq!(mex(5, vec![10, 5, 7, 9, 8]), 6);
	assert_eq!(mex(5, vec![]), 5);
}

pub fn time_now() -> SystemTime {
	SystemTime::UNIX_EPOCH + Duration::from_millis(js_sys::Date::now() as u64)
}

pub async fn find_cursor_place(path: &Path) -> Option<Position> {
	let doc = fs::read_to_string(path).await.unwrap_or_default();
	let mut found_main = false;
	for (line, content) in doc.lines().enumerate() {
		if !found_main && content.contains("int main(") {
			found_main = true;
		}
		if content.trim().is_empty() && (!content.is_empty() || found_main) {
			return Some(Position { line, column: 80 });
		}
	}
	None
}

pub fn plural(x: usize, singular: &str, plural: &str) -> String {
	format!("{} {}", x, if x == 1 { singular } else { plural })
}

pub fn expand_path(path: &str) -> Path {
	let expanded = if path == "~" || path.starts_with("~/") {
		format!("{}{}", node_sys::os::homedir(), &path[1..])
	} else {
		path.to_owned()
	};
	let normalized = node_sys::path::normalize(&expanded);
	Path::from_native(normalized)
}

pub fn node_hrtime() -> Duration {
	let raw_time = node_sys::process::hrtime();
	match raw_time.values().into_iter().map(|v| v.unwrap().as_f64().unwrap()).collect::<Vec<_>>().as_slice() {
		[seconds, nanoseconds] => Duration::new(*seconds as u64, *nanoseconds as u32),
		_ => unreachable!(),
	}
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum SourceTarget {
	Main,
	Custom(Path),
}

impl SourceTarget {
	pub fn into_path(self) -> R<Path> {
		match self {
			SourceTarget::Main => dir::solution(),
			SourceTarget::Custom(source) => Ok(source),
		}
	}
}

pub async fn sleep(delay: Duration) {
	let (tx, rx) = oneshot::channel();
	node_sys::timers::set_timeout(
		Closure::once_into_js(move || {
			let _ = tx.send(());
		}),
		delay.as_secs_f64() * 1000.0,
	);
	rx.await.unwrap();
}

pub enum OS {
	Windows,
	Linux,
	MacOS,
}

impl OS {
	pub fn query() -> R<OS> {
		match (node_sys::process::PLATFORM.as_str(), node_sys::process::ARCH.as_str()) {
			("linux", _) | ("freebsd", _) | ("openbsd", _) => Ok(OS::Linux),
			("win32", _) => Ok(OS::Windows),
			("darwin", _) => Ok(OS::MacOS),
			(platform, arch) => Err(E::error(format!("running on unrecognized platform {}-{}", platform, arch))),
		}
	}
}

pub fn workspace_root() -> R<Path> {
	let buf = evscode::workspace_root().map_err(suggest_open)?;
	Ok(Path::from_native(buf))
}

pub fn suggest_open(e: E) -> E {
	e.action("Open URL (Alt+F11)", crate::open::url())
		.action("Scan for contests (Alt+F9)", crate::open::scan())
		.action("How to use ICIE?", help_open())
}

async fn help_open() -> R<()> {
	evscode::open_external("https://github.com/pustaczek/icie/blob/master/README.md#quick-start").await
}

pub fn join_all_with_progress<I>(
	title: &str,
	i: I,
) -> impl Future<Output=Vec<<<I as IntoIterator>::Item as Future>::Output>>
where
	I: IntoIterator,
	<I as IntoIterator>::Item: Future,
{
	let (progress, _) = evscode::Progress::new().title(title).show();
	let progress = Arc::new(progress);
	let objects: Vec<_> = i.into_iter().collect();
	let increment = 100. / (objects.len() as f64);
	join_all(objects.into_iter().map(move |fut| {
		let progress = Arc::clone(&progress);
		async move {
			let result = fut.await;
			progress.increment(increment);
			result
		}
	}))
}
