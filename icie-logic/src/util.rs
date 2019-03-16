use crate::error::{self, R};
use failure::ResultExt;
use std::{
	ffi, fs, path::{Path, PathBuf}, process::{Child, Command}
};

pub struct TransactionDir {
	path: PathBuf,
	good: bool,
}
impl TransactionDir {
	pub fn new(path: &Path) -> R<TransactionDir> {
		ci::util::demand_dir(path)?;
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
			fs::remove_dir_all(&self.path).expect("failed to delete uncommited directory");
		}
	}
}

pub fn without_extension(path: &Path) -> PathBuf {
	path.parent().unwrap_or(Path::new("")).join(path.file_stem().unwrap_or(ffi::OsStr::new("")))
}

pub fn path_to_str(path: &Path) -> R<&str> {
	Ok(path.to_str().ok_or_else(|| error::Category::NonUTF8Path.err())?)
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

pub fn try_commands(commands: &[(&str, &[&str])], common: impl Fn(&mut Command) -> R<()>) -> R<Child> {
	for (app, args) in commands {
		let mut cmd = Command::new(app);
		cmd.args(args.iter());
		common(&mut cmd)?;
		match cmd.spawn() {
			Ok(child) => return Ok(child),
			Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => continue,
			Err(e) => return Err(e).context(format!("failed to execute {:?} {:?}", app, args))?,
		}
	}
	let apps = commands.iter().map(|cmd| cmd.0.to_owned()).collect::<Vec<_>>();
	return Err(error::Category::AppNotInstalled { apps }.err())?;
}

pub fn read_to_string_if_exists(p: impl AsRef<Path>) -> std::io::Result<Option<String>> {
	match std::fs::read_to_string(p) {
		Ok(s) => Ok(Some(s)),
		Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
		Err(e) => Err(e),
	}
}
