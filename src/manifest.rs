use serde::Serialize;
use std::{fs::File, path::Path};

#[derive(Serialize)]
pub struct Manifest {
	#[serde(default)]
	task_url: Option<String>,
}

impl Manifest {
	pub fn new_project(task_url: Option<String>) -> Manifest {
		Manifest { task_url }
	}

	pub fn save(&self, root: &Path) -> evscode::R<()> {
		let f = File::create(root.join(".icie"))?;
		serde_json::to_writer(f, &self)?;
		Ok(())
	}
}
