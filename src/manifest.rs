use serde::{Deserialize, Serialize};
use std::{fs::File, path::Path};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
	#[serde(default)]
	pub task_url: Option<String>,
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

	pub fn load() -> evscode::R<Manifest> {
		let s = crate::util::fs_read_to_string(evscode::workspace_root()?.join(".icie"))?;
		let manifest = serde_json::from_str(&s)?;
		Ok(manifest)
	}
}
