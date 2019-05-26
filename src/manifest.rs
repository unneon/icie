use evscode::{E, R};
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

	pub fn save(&self, root: &Path) -> R<()> {
		let f = File::create(root.join(".icie")).map_err(|e| E::from_std(e).context("failed to create manifest file"))?;
		serde_json::to_writer(f, &self).map_err(|e| E::from_std(e).context("failed to write the manifest to file"))?;
		Ok(())
	}

	pub fn load() -> R<Manifest> {
		let s = crate::util::fs_read_to_string(evscode::workspace_root()?.join(".icie"))?;
		let manifest = serde_json::from_str(&s).map_err(|e| E::from_std(e).context(".icie is not a valid icie::manifest::Manifest"))?;
		Ok(manifest)
	}
}
