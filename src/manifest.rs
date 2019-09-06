use evscode::{error::ResultExt, R};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::Path};
use unijudge::Statement;

#[derive(Serialize, Deserialize)]
pub struct Manifest {
	#[serde(default)]
	pub task_url: Option<String>,
	#[serde(default)]
	pub statement: Option<Statement>,
}

impl Manifest {
	pub fn save(&self, root: &Path) -> R<()> {
		crate::util::fs_create_dir_all(root.parent().unwrap())?;
		let f = File::create(root.join(".icie")).wrap("failed to create manifest file")?;
		serde_json::to_writer(f, &self).wrap("failed to write the manifest to file")?;
		Ok(())
	}

	pub fn load() -> R<Manifest> {
		let s = crate::util::fs_read_to_string(evscode::workspace_root()?.join(".icie"))
			.map_err(|e| e.context("project not created with Alt+F9 or Alt+F11"))?;
		let manifest = serde_json::from_str(&s).wrap(".icie is not a valid icie::manifest::Manifest")?;
		Ok(manifest)
	}

	pub fn req_statement(&self) -> R<&Statement> {
		self.statement.as_ref().wrap("could not find statement, make sure site supports it and task was opened with Alt+F9 or Alt+F11")
	}

	pub fn req_task_url(&self) -> R<&str> {
		Ok(self.task_url.as_ref().wrap("could not find task url, make sure task was opened with Alt+F9 or Alt+F11")?.as_str())
	}
}
