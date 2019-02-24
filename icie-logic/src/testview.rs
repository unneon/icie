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
		outcome: ci::testing::TestResult,
	},
	Directory {
		files: Vec<Tree>,
	},
}
