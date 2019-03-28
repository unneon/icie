use crate::error::R;
use std::{
	fs::{self, File}, path::Path
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
	pub task_url: Option<String>,
}

impl Manifest {
	pub fn save(&self, path: &Path) -> R<()> {
		fs::write(path, serde_json::to_string(&self)?)?;
		Ok(())
	}

	pub fn load(path: &Path) -> R<Manifest> {
		Ok(serde_json::from_reader(File::open(path)?)?)
	}
}
