use crate::{dir, init::SOLUTION_TEMPLATE, util};
use evscode::{error::ResultExt, R};
use std::path::Path;
use unijudge::{Example, Statement};

pub async fn init_manifest(root: &Path, url: &Option<String>, statement: Option<Statement>) -> R<()> {
	let manifest = crate::manifest::Manifest { task_url: url.clone(), statement };
	manifest.save(root).await?;
	Ok(())
}

pub async fn init_template(root: &Path) -> R<()> {
	let solution = root.join(format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()));
	if !solution.exists() {
		let req_id = SOLUTION_TEMPLATE.get();
		let list = crate::template::LIST.get();
		let path = list
			.iter()
			.find(|(id, _)| **id == *req_id)
			.wrap(format!(
				"template '{}' does not exist; go to the settings(Ctrl+,), and either change the template(icie.init.solutionTemplate) or add a template with this \
				 name(icie.template.list)",
				req_id
			))?
			.1;
		let tpl = crate::template::load(&path).await?;
		util::fs_write(&solution, tpl.code).await?;
	}
	Ok(())
}

pub async fn init_examples(root: &Path, examples: &[Example]) -> R<()> {
	let examples_dir = root.join("tests").join("example");
	util::fs_create_dir_all(&examples_dir).await?;
	for (i, test) in examples.iter().enumerate() {
		let in_path = examples_dir.join(format!("{}.in", i + 1));
		let out_path = examples_dir.join(format!("{}.out", i + 1));
		util::fs_write(&in_path, &test.input).await?;
		util::fs_write(&out_path, &test.output).await?;
	}
	Ok(())
}
