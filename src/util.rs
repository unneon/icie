use crate::util::path::Path;
use evscode::{error::ResultExt, Position, E, R};
use futures::channel::oneshot;
use std::time::{Duration, SystemTime};
use wasm_bindgen::{closure::Closure, JsValue};

pub mod fs;
pub mod path;
pub mod tempfile;

pub use tempfile::Tempfile;

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

pub fn fmt_verb(verb: &'static str, path: impl MaybePath) -> String {
	if let Some(path) = path.as_option_path() {
		let file = match evscode::workspace_root() {
			Ok(root) => path.strip_prefix(&Path::from_native(root)).unwrap(),
			Err(_) => path.clone(),
		};
		format!("{} {}", verb, file)
	} else {
		String::from(verb)
	}
}

pub async fn active_tab() -> R<Option<Path>> {
	let source = Path::from_native(evscode::active_editor_file().await.ok_or_else(E::cancel)?);
	Ok(if source != crate::dir::solution()? { Some(source) } else { None })
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

pub async fn is_installed(app: &'static str) -> R<bool> {
	let exec_lookups = env("PATH")?;
	for exec_lookup in exec_lookups.split(&String::from(node_sys::path::DELIMITER.clone())) {
		let path = Path::from_native(exec_lookup.to_owned()).join(app);
		if fs::exists(&path).await? {
			return Ok(true);
		}
	}
	Ok(false)
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
	r#"<link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet">"#
		.to_owned()
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

pub fn without_extension(path: &Path) -> Path {
	let path = path.as_ref();
	path.parent().join(path.file_stem())
}

#[test]
fn test_pathmanip() {
	assert_eq!(without_extension("/home/wizard/file.txt"), Path::new("/home/wizard/file"));
	assert_eq!(
		without_extension("/home/wizard/source.old.cpp"),
		Path::new("/home/wizard/source.old")
	);
	assert_eq!(without_extension("../manifest.json"), Path::new("../manifest"));
	assert_eq!(without_extension("./inner/dev0"), Path::new("./inner/dev0"));
}

pub fn node_hrtime() -> Duration {
	let raw_time = node_sys::process::hrtime();
	match raw_time
		.values()
		.into_iter()
		.map(|v| v.unwrap().as_f64().unwrap())
		.collect::<Vec<_>>()
		.as_slice()
	{
		[seconds, nanoseconds] => Duration::new(*seconds as u64, *nanoseconds as u32),
		_ => unreachable!(),
	}
}

pub trait MaybePath {
	fn as_option_path(&self) -> Option<&Path>;
}
impl<'a> MaybePath for Option<&'a Path> {
	fn as_option_path(&self) -> Option<&Path> {
		*self
	}
}
impl MaybePath for Path {
	fn as_option_path(&self) -> Option<&Path> {
		Some(&self)
	}
}
impl MaybePath for Option<Path> {
	fn as_option_path(&self) -> Option<&Path> {
		self.as_ref()
	}
}
impl<'a, T: MaybePath> MaybePath for &'a T {
	fn as_option_path(&self) -> Option<&Path> {
		(*self).as_option_path()
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
			(platform, arch) => {
				Err(E::error(format!("running on unrecognized platform {}-{}", platform, arch)))
			},
		}
	}
}
