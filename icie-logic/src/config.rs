use crate::error::{self, R};
use std::{
	collections::HashSet, fs::{self, File}, path::{Path, PathBuf}
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Cursor {
	pub row: i64,
	pub column: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Template {
	pub id: String,
	pub name: String,
	pub path: PathBuf,
	pub cursor: Cursor,
	pub default_filename: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
	pub project_directory: PathBuf,
	pub main_template_id: String,
	pub templates: Vec<Template>,
}

impl Config {
	pub fn template_main(&self) -> R<&Template> {
		Ok(self.templates.iter().find(|template| template.id == self.main_template_id).ok_or_else(|| {
			error::Category::TemplateDoesNotExist {
				id: self.main_template_id.clone(),
			}
			.err()
		})?)
	}

	pub fn load_or_create() -> R<Config> {
		let config_dir = dirs::config_dir()
			.ok_or_else(|| error::Category::DegenerateEnvironment { detail: "no config directory" }.err())?
			.join("icie");
		let config_path = config_dir.join("config.json");
		let template_main_path = config_dir.join("template-main.cpp");
		if !config_path.exists() {
			fs::create_dir_all(&config_dir)?;
			let cursor = if template_main_path.exists() {
				Cursor { row: 1, column: 1 }
			} else {
				fs::write(
					&template_main_path,
					format!(
						r"#include <bits/stdc++.h>
using namespace std;
// Edit your config and template at {} 😄 💖
int main() {{
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

}}
",
						config_dir.display()
					),
				)?;
				Cursor { row: 8, column: 5 }
			};
			let config = Config {
				project_directory: dirs::home_dir().ok_or_else(|| error::Category::DegenerateEnvironment { detail: "no config directory" }.err())?,
				main_template_id: "main".to_string(),
				templates: vec![Template {
					id: "main".to_string(),
					name: "C++ General code".to_string(),
					path: template_main_path,
					cursor,
					default_filename: "main.cpp".to_string(),
				}],
			};
			serde_json::to_writer_pretty(File::create(&config_path)?, &config)?;
		}
		Config::load(&config_path)
	}

	fn load(path: &Path) -> R<Config> {
		let config: Config = serde_json::from_reader(File::open(path)?)?;
		config.verify()?;
		Ok(config)
	}

	fn verify(&self) -> R<()> {
		let id_set = self.templates.iter().map(|template| &template.id).collect::<HashSet<_>>();
		if id_set.len() != self.templates.len() {
			Err(error::Category::MalformedConfig {
				detail: "template ids have to be unique",
			}
			.err())?;
		}
		if !id_set.contains(&self.main_template_id) {
			Err(error::Category::MalformedConfig {
				detail: "main template does not exist",
			}
			.err())?;
		}
		Ok(())
	}

	pub fn library_path(&self) -> R<PathBuf> {
		Ok(dirs::config_dir()
			.ok_or_else(|| error::Category::DegenerateEnvironment { detail: "no config directory" }.err())?
			.join("icie")
			.join("library.json"))
	}
}
