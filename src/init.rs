use crate::{dir, util};
use failure::ResultExt;
use std::{
	fs, path::{Path, PathBuf}
};

#[evscode::config(description = "Which code template to use as the solution file?")]
static SOLUTION_TEMPLATE: evscode::Config<String> = "C++";

fn init(root: &Path) -> evscode::R<()> {
	let _status = crate::STATUS.push("Initializing");
	let url = match evscode::InputBox::new()
		.prompt("Enter task URL or leave empty")
		.placeholder("https://codeforces.com/contest/.../problem/...")
		.ignore_focus_out()
		.spawn()
		.wait()
	{
		Some(ref url) if url.trim().is_empty() => None,
		Some(url) => Some(url),
		None => return Err(evscode::E::cancel()),
	};
	init_manifest(root, &url)?;
	init_template(root)?;
	init_examples(root, &url)?;
	evscode::open_folder(root, false);
	Ok(())
}

fn init_manifest(root: &Path, url: &Option<String>) -> evscode::R<()> {
	let manifest = crate::manifest::Manifest::new_project(url.clone());
	manifest.save(root)?;
	Ok(())
}
fn init_template(root: &Path) -> evscode::R<()> {
	let solution = root.join(format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()));
	if !solution.exists() {
		let req_id = SOLUTION_TEMPLATE.get();
		let list = crate::template::LIST.get();
		let path = match list.iter().find(|(id, _)| **id == *req_id) {
			Some((_, path)) => path,
			None => {
				return Err(evscode::E::error(format!(
					"template '{}' does not exist; go to the settings(Ctrl+,), and either change the template(icie.init.solutionTemplate) or add a template with this \
					 name(icie.template.list)",
					req_id
				)))
			},
		};
		let tpl = crate::template::load(&path)?;
		fs::write(solution, tpl.code)?;
	}
	Ok(())
}
fn init_examples(root: &Path, url: &Option<String>) -> evscode::R<()> {
	if let Some(url) = url {
		let url = unijudge::TaskUrl::deconstruct(&url).compat()?;
		let (username, password) = {
			let _status = crate::STATUS.push("Remembering passwords");
			crate::auth::site_credentials(&url.site)?
		};
		let sess = {
			let _status = crate::STATUS.push("Logging in");
			unijudge::connect_login(&url.site, &username, &password).compat()?
		};
		let cont = sess.contest(&url.contest);
		let examples_dir = root.join("tests").join("example");
		fs::create_dir_all(&examples_dir)?;
		let tests = {
			let _status = crate::STATUS.push("Downloading tests");
			cont.examples(&url.task).compat()?
		};
		for (i, test) in tests.into_iter().enumerate() {
			fs::write(examples_dir.join(format!("{}.in", i + 1)), &test.input)?;
			fs::write(examples_dir.join(format!("{}.out", i + 1)), &test.output)?;
		}
	}
	Ok(())
}

#[evscode::command(title = "ICIE Init", key = "alt+f11")]
fn new() -> evscode::R<()> {
	let _status = crate::STATUS.push("Initializing");
	let root = ASK_FOR_PATH.get().query(&*dir::PROJECT_DIRECTORY.get(), &dir::random_codename())?;
	let dir = util::TransactionDir::new(&root)?;
	init(&root)?;
	dir.commit();
	Ok(())
}

#[evscode::command(title = "ICIE Init existing")]
fn existing() -> evscode::R<()> {
	let _status = crate::STATUS.push("Initializing");
	let root = evscode::workspace_root();
	init(&root)?;
	Ok(())
}

#[evscode::config(description = "Ask for path before initializing?")]
static ASK_FOR_PATH: evscode::Config<PathDialog> = PathDialog::None;

#[derive(Clone, Debug, evscode::Configurable)]
enum PathDialog {
	#[evscode(name = "No")]
	None,
	#[evscode(name = "With a VS Code input box")]
	InputBox,
	#[evscode(name = "With a system dialog")]
	SystemDialog,
}

impl PathDialog {
	fn query(&self, directory: &Path, codename: &str) -> evscode::R<PathBuf> {
		let basic = directory.join(codename);
		let basic_str = basic.display().to_string();
		match self {
			PathDialog::None => Ok(basic),
			PathDialog::InputBox => Ok(PathBuf::from(
				evscode::InputBox::new()
					.ignore_focus_out()
					.prompt("New project directory")
					.value(basic_str.as_str())
					.value_selection(basic_str.len() - codename.len(), basic_str.len())
					.spawn()
					.wait()
					.ok_or_else(|| evscode::E::cancel())?,
			)),
			PathDialog::SystemDialog => Ok(evscode::OpenDialog::new()
				.directory()
				.action_label("Init")
				.spawn()
				.wait()
				.ok_or_else(|| evscode::E::cancel())?),
		}
	}
}
