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
		.status()?
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
	match std::fs::read_to_string(path.as_ref()) {
		Ok(s) => Ok(s),
		Err(e) => {
			if e.kind() == std::io::ErrorKind::NotFound {
				Err(evscode::E::from(e).reform(format!("file {} does not exist", path.as_ref().display())))
			} else {
				Err(evscode::E::from(e))
			}
		},
	}
}

pub fn nice_open_editor(path: impl AsRef<Path>) -> evscode::R<()> {
	let doc = std::fs::read_to_string(path.as_ref())?;
	for (i, line) in doc.lines().enumerate() {
		if !line.is_empty() && line.trim().is_empty() {
			evscode::open_editor(path.as_ref(), Some(i), Some(80));
			return Ok(());
		}
	}
	evscode::open_editor(path.as_ref(), None, None);
	Ok(())
}

pub struct TransactionDir {
	path: PathBuf,
	good: bool,
}
impl TransactionDir {
	pub fn new(path: &Path) -> evscode::R<TransactionDir> {
		std::fs::create_dir_all(path)?;
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
