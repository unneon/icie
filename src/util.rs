use evscode::{E, R};
use std::{
	path::{Path, PathBuf}, process::{Command, Stdio}, time::Duration
};

pub fn fmt_time_short(t: &Duration) -> String {
	let s = t.as_secs();
	let ms = t.as_millis() % 1000;
	format!("{}.{:03}s", s, ms)
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
			c => escaped.push(c),
		};
	}
	escaped += "\"";
	escaped
}

pub fn is_installed(app: &'static str) -> evscode::R<bool> {
	Ok(Command::new("which")
		.arg(app)
		.stdout(Stdio::null())
		.stdin(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.map_err(|e| evscode::E::from_std(e).context("failed to check whether a program in installed with which(1)"))?
		.success())
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

pub fn nice_open_editor(path: impl AsRef<Path>) -> evscode::R<()> {
	let doc = std::fs::read_to_string(path.as_ref()).unwrap_or_default();
	let mut found_main = false;
	for (i, line) in doc.lines().enumerate() {
		if !found_main && line.contains("int main(") {
			found_main = true;
		}
		if line.trim().is_empty() && (!line.is_empty() || found_main) {
			evscode::open_editor(path.as_ref(), Some(i), Some(80));
			return Ok(());
		}
	}
	evscode::open_editor(path.as_ref(), None, None);
	Ok(())
}

pub fn without_extension(path: impl AsRef<Path>) -> PathBuf {
	let path = path.as_ref();
	path.parent().unwrap().join(path.file_stem().unwrap())
}

pub struct TransactionDir {
	path: PathBuf,
	good: bool,
}
impl TransactionDir {
	pub fn new(path: &Path) -> evscode::R<TransactionDir> {
		fs_create_dir_all(path)?;
		Ok(TransactionDir {
			path: path.to_owned(),
			good: false,
		})
	}

	pub fn commit(mut self) {
		self.good = true;
	}
}
impl Drop for TransactionDir {
	fn drop(&mut self) {
		if !self.good {
			std::fs::remove_dir_all(&self.path).expect("failed to delete uncommited directory");
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
