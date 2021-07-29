use crate::util::{fs, path::Path, suggest_open, workspace_root};
use evscode::{error::ResultExt, R};
use serde::{Deserialize, Serialize};
use unijudge::Statement;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
	#[serde(default)]
	pub task_url: Option<String>,
	#[serde(default)]
	pub statement: Option<Statement>,
}

impl Manifest {
	pub async fn save(&self, workspace: &Path) -> R<()> {
		fs::create_dir_all(&workspace.parent()).await?;
		let written = serde_json::to_string(self).wrap("failed to serialize the manifest")?;
		let path = workspace.join(".icie");
		fs::write(&path, written).await?;
		Ok(())
	}

	pub async fn load() -> R<Manifest> {
		let path = workspace_root()?.join(".icie");
		let s = fs::read_to_string(&path).await.map_err(|e| suggest_open(e.context("this folder has no task open")))?;
		let manifest = serde_json::from_str(&s).wrap(".icie is not a valid icie::manifest::Manifest")?;
		Ok(manifest)
	}

	pub fn req_statement(&self) -> R<&Statement> {
		self.statement.as_ref().wrap("this folder has no downloaded task description").map_err(suggest_open)
	}

	pub fn req_task_url(&self) -> R<&str> {
		Ok(self.task_url.as_ref().wrap("this folder has no task URL set").map_err(suggest_open)?.as_str())
	}
}
