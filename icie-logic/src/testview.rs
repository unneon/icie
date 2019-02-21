use std::time::Duration;

#[derive(Debug)]
pub enum Tree {
	Test {
		name: String,
		input: String,
		output: String,
		desired: Option<String>,
		timing: Option<Duration>,
	},
	Directory {
		files: Vec<Tree>,
	},
}
