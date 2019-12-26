use crate::{
	dir, init::SOLUTION_TEMPLATE, util::{fs, path::Path}
};
use evscode::{error::ResultExt, R};
use unijudge::{Example, Statement};

pub async fn init_manifest(
	root: &Path,
	url: &Option<String>,
	statement: Option<Statement>,
) -> R<()>
{
	let manifest = crate::manifest::Manifest { task_url: url.clone(), statement };
	manifest.save(root).await?;
	Ok(())
}

pub async fn init_template(root: &Path) -> R<()> {
	let solution = root.join(format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()));
	if !fs::exists(&solution).await? {
		let req_id = SOLUTION_TEMPLATE.get();
		let list = crate::template::LIST.get();
		let path = list
			.iter()
			.find(|(id, _)| **id == *req_id)
			.wrap(format!(
				"template '{}' does not exist; go to the settings(Ctrl+,), and either change the \
				 template(icie.init.solutionTemplate) or add a template with this \
				 name(icie.template.list)",
				req_id
			))?
			.1;
		let tpl = crate::template::load(&path).await?;
		fs::write(&solution, tpl.code).await?;
	}
	Ok(())
}

pub async fn init_examples(root: &Path, examples: &[Example]) -> R<()> {
	let examples_dir = root.join("tests").join("example");
	fs::create_dir_all(&examples_dir).await?;
	for (i, test) in examples.iter().enumerate() {
		let in_path = examples_dir.join(format!("{}.in", i + 1));
		let out_path = examples_dir.join(format!("{}.out", i + 1));
		fs::write(&in_path, &test.input).await?;
		fs::write(&out_path, &test.output).await?;
	}
	Ok(())
}
