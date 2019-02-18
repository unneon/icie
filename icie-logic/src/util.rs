use crate::error::R;
use std::{
	fs, path::{Path, PathBuf}
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
