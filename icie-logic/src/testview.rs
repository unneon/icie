use std::{path::PathBuf, time::Duration};

#[derive(Debug)]
pub enum Tree {
	Test {
		name: String,
		input: String,
		output: String,
		desired: Option<String>,
		timing: Option<Duration>,
		in_path: PathBuf,
	},
	Directory {
		files: Vec<Tree>,
	},
}
