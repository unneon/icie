use crate::error::{self, R};
use std::{
	ffi, fs, path::{Path, PathBuf}
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
