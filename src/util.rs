use evscode::{Position, E, R};
use std::{
	path::{Path, PathBuf}, time::Duration
};

pub fn fmt_time_short(t: &Duration) -> String {
	let s = t.as_secs();
	let ms = t.as_millis() % 1000;
	format!("{}.{:03}s", s, ms)
}

pub fn fmt_time_left(mut t: Duration) -> String {
	let mut s = {
		let x = t.as_secs() % 60;
		t -= Duration::from_secs(x);
		format!("{} seconds left", x)
	};
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 % 60;
		t -= Duration::from_secs(x * 60);
		s = format!("{} minutes, {}", x, s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 % 24;
		t -= Duration::from_secs(x * 60 * 60);
		s = format!("{} hours, {}", x, s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 / 24;
		t -= Duration::from_secs(x * 60 * 60 * 24);
		s = format!("{} days, {}", x, s)
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
			Ok(root) => path.strip_prefix(root).unwrap(),
			Err(_) => path,
		};
		format!("{} {}", verb, file.display())
	} else {
		String::from(verb)
	}
}

pub fn active_tab() -> evscode::R<Option<PathBuf>> {
	let source = evscode::active_editor_file().wait().ok_or_else(E::cancel)?;
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

pub fn is_installed(app: &'static str) -> evscode::R<bool> {
	let exec_lookups = std::env::var("PATH").map_err(|e| E::from_std(e).context("env var PATH does not exist"))?;
	for exec_lookup in std::env::split_paths(&exec_lookups) {
		if exec_lookup.join(app).exists() {
			return Ok(true);
		}
	}
	Ok(false)
}

#[test]
fn test_is_installed() {
	assert_eq!(is_installed("cargo").unwrap(), true);
	assert_eq!(is_installed("icie-this-executable-does-no-exist").unwrap(), false);
}

pub fn html_material_icons() -> String {
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

pub fn fs_read_to_string(path: impl AsRef<Path>) -> evscode::R<String> {
	std::fs::read_to_string(path.as_ref()).map_err(|e| {
		let is_not_found = e.kind() == std::io::ErrorKind::NotFound;
		evscode::E::from_std(e).context(if is_not_found {
			format!("file {} does not exist", path.as_ref().display())
		} else {
			format!("failed to read file {}", path.as_ref().display())
		})
	})
}

pub fn fs_write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> R<()> {
	std::fs::write(path.as_ref(), content.as_ref()).map_err(|e| E::from_std(e).context(format!("failed to write to {}", path.as_ref().display())))
}

pub fn fs_create_dir_all(path: impl AsRef<Path>) -> R<()> {
	std::fs::create_dir_all(path.as_ref()).map_err(|e| E::from_std(e).context(format!("failed to create directory {}", path.as_ref().display())))
}

pub fn find_cursor_place(path: &Path) -> Option<Position> {
	let doc = std::fs::read_to_string(path).unwrap_or_default();
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

pub fn without_extension(path: impl AsRef<Path>) -> PathBuf {
	let path = path.as_ref();
	path.parent().unwrap().join(path.file_stem().unwrap())
}

#[test]
fn test_pathmanip() {
	assert_eq!(without_extension("/home/wizard/file.txt"), Path::new("/home/wizard/file"));
	assert_eq!(without_extension("/home/wizard/source.old.cpp"), Path::new("/home/wizard/source.old"));
	assert_eq!(without_extension("../manifest.json"), Path::new("../manifest"));
	assert_eq!(without_extension("./inner/dev0"), Path::new("./inner/dev0"));
}

pub struct TransactionDir<'a> {
	path: &'a Path,
	good: bool,
}
impl<'a> TransactionDir<'a> {
	pub fn new(path: &'a Path) -> evscode::R<TransactionDir<'a>> {
		fs_create_dir_all(path)?;
		Ok(TransactionDir { path, good: false })
	}

	pub fn path(&self) -> &'a Path {
		self.path
	}

	pub fn commit(mut self) -> &'a Path {
		self.good = true;
		self.path
	}
}
impl Drop for TransactionDir<'_> {
	fn drop(&mut self) {
		if !self.good {
			std::fs::remove_dir_all(self.path).expect("failed to delete uncommited directory");
		}
	}
}

pub trait MaybePath {
	fn as_option_path(&self) -> Option<&Path>;
}
impl<'a> MaybePath for &'a Path {
	fn as_option_path(&self) -> Option<&Path> {
		Some(self)
	}
}
impl<'a> MaybePath for Option<&'a Path> {
	fn as_option_path(&self) -> Option<&Path> {
		*self
	}
}
impl MaybePath for PathBuf {
	fn as_option_path(&self) -> Option<&Path> {
		Some(self.as_path())
	}
}
impl MaybePath for Option<PathBuf> {
	fn as_option_path(&self) -> Option<&Path> {
		self.as_ref().map(|p| p.as_path())
	}
}
impl<'a, T: MaybePath> MaybePath for &'a T {
	fn as_option_path(&self) -> Option<&Path> {
		(*self).as_option_path()
	}
}
