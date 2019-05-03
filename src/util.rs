use std::{
	path::{Path, PathBuf}, time::Duration
};

pub fn fmt_time_short(t: &Duration) -> String {
	let s = t.as_secs();
	let ms = t.as_millis() % 1000;
	format!("{}.{:03}s", s, ms)
}

pub fn fmt_verb(verb: &'static str, path: impl MaybePath) -> String {
	if let Some(path) = path.as_option_path() {
		let file = path.strip_prefix(evscode::workspace_root()).unwrap();
		format!("{} {}", verb, file.display())
	} else {
		String::from(verb)
	}
}

pub fn active_tab() -> evscode::R<Option<PathBuf>> {
	let source = match evscode::active_editor_file().wait() {
		Some(source) => source,
		None => return Err(evscode::E::cancel()),
	};
	Ok(if source != crate::dir::solution() { Some(source) } else { None })
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
		self.clone()
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
