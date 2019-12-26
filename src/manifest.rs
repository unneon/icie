use crate::util::{fs, path::Path};
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
	pub async fn save(&self, root: &Path) -> R<()> {
		fs::create_dir_all(&root.parent()).await?;
		let written = serde_json::to_string(self).wrap("failed to serialize the manifest")?;
		let path = root.join(".icie");
		fs::write(&path, written).await?;
		Ok(())
	}

	pub async fn load() -> R<Manifest> {
		let path = Path::from_native(evscode::workspace_root()?).join(".icie");
		let s = fs::read_to_string(&path)
			.await
			.map_err(|e| e.context("project not created with Alt+F9 or Alt+F11"))?;
		let manifest =
			serde_json::from_str(&s).wrap(".icie is not a valid icie::manifest::Manifest")?;
		Ok(manifest)
	}

	pub fn req_statement(&self) -> R<&Statement> {
		self.statement.as_ref().wrap(
			"could not find statement, make sure site supports it and task was opened with Alt+F9 \
			 or Alt+F11",
		)
	}

	pub fn req_task_url(&self) -> R<&str> {
		Ok(self
			.task_url
			.as_ref()
			.wrap("could not find task url, make sure task was opened with Alt+F9 or Alt+F11")?
			.as_str())
	}
}
