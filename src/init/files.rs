use crate::{dir, init::SOLUTION_TEMPLATE, util};
use evscode::{E, R};
use std::path::Path;
use unijudge::Example;

pub fn init_manifest(root: &Path, url: &Option<String>) -> R<()> {
	let manifest = crate::manifest::Manifest::new_project(url.clone());
	manifest.save(root)?;
	Ok(())
}

pub fn init_template(root: &Path) -> R<()> {
	let solution = root.join(format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()));
	if !solution.exists() {
		let req_id = SOLUTION_TEMPLATE.get();
		let list = crate::template::LIST.get();
		let path = list
			.iter()
			.find(|(id, _)| **id == *req_id)
			.ok_or_else(|| {
				E::error(format!(
					"template '{}' does not exist; go to the settings(Ctrl+,), and either change the template(icie.init.solutionTemplate) or add a template with this \
					 name(icie.template.list)",
					req_id
				))
			})?
			.1;
		let tpl = crate::template::load(&path)?;
		util::fs_write(solution, tpl.code)?;
	}
	Ok(())
}

pub fn init_examples(root: &Path, examples: &[Example]) -> R<()> {
	let examples_dir = root.join("tests").join("example");
	util::fs_create_dir_all(&examples_dir)?;
	for (i, test) in examples.iter().enumerate() {
		util::fs_write(examples_dir.join(format!("{}.in", i + 1)), &test.input)?;
		util::fs_write(examples_dir.join(format!("{}.out", i + 1)), &test.output)?;
	}
	Ok(())
}
