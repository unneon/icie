use std::{
	fs::{self, File}, path::Path
};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
	pub task_url: String,
}

impl Manifest {
	pub fn save(&self, path: &Path) {
		fs::write(path, serde_json::to_string(&self).unwrap()).unwrap();
	}

	pub fn load(path: &Path) -> Manifest {
		serde_json::from_reader(File::open(path).unwrap()).unwrap()
	}
}
