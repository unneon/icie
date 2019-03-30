use crate::{error::R, nice_duration, DEFAULT_JUDGE_TIME_LIMIT};
use std::{
	fs::{self, File}, path::Path, time::Duration
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
	pub task_url: Option<String>,
	#[serde(serialize_with = "nice_duration::serialize", deserialize_with = "nice_duration::deserialize")]
	pub time_limit: Option<Duration>,
}

impl Manifest {
	pub fn save(&self, path: &Path) -> R<()> {
		fs::write(path, serde_json::to_string(&self)?)?;
		Ok(())
	}

	pub fn load(path: &Path) -> R<Manifest> {
		if !path.exists() {
			Manifest::default().save(path)?;
		}
		Ok(serde_json::from_reader(File::open(path)?)?)
	}

	fn default() -> Manifest {
		Manifest {
			task_url: None,
			time_limit: Some(DEFAULT_JUDGE_TIME_LIMIT),
		}
	}
}
