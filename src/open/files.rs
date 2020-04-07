use crate::{
	dir, template, util::{fs, path::Path}
};
use evscode::R;
use unijudge::{Example, Statement, TaskDetails};

pub async fn open_task(workspace: &Path, url: Option<String>, meta: Option<TaskDetails>) -> R<()> {
	let _status = crate::STATUS.push("Opening");
	fs::create_dir_all(workspace).await?;
	let examples =
		meta.as_ref().and_then(|meta| meta.examples.as_ref()).map(|examples| examples.as_slice()).unwrap_or(&[]);
	let statement = meta.as_ref().and_then(|meta| meta.statement.clone());
	create_manifest(workspace, &url, statement).await?;
	create_template(workspace).await?;
	create_examples(workspace, examples).await?;
	Ok(())
}

async fn create_manifest(workspace: &Path, url: &Option<String>, statement: Option<Statement>) -> R<()> {
	let manifest = crate::manifest::Manifest { task_url: url.clone(), statement };
	manifest.save(workspace).await?;
	Ok(())
}

async fn create_template(workspace: &Path) -> R<()> {
	let solution = workspace.join(format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()));
	if !fs::exists(&solution).await? {
		let template = template::load_solution().await?;
		fs::write(&solution, template.code).await?;
	}
	Ok(())
}

async fn create_examples(workspace: &Path, examples: &[Example]) -> R<()> {
	let examples_dir = workspace.join("tests").join("example");
	fs::create_dir_all(&examples_dir).await?;
	for (i, test) in examples.iter().enumerate() {
		let in_path = examples_dir.join(format!("{}.in", i + 1));
		let out_path = examples_dir.join(format!("{}.out", i + 1));
		fs::write(&in_path, &test.input).await?;
		fs::write(&out_path, &test.output).await?;
	}
	Ok(())
}
