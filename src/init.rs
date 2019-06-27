use crate::{dir, interpolation::Interpolation, util};
use evscode::{E, R};
use std::{
	fmt, path::{Path, PathBuf}, str::FromStr
};
use unijudge::Example;

#[evscode::config(
	description = "The name of the code template used for initializing new projects. The list of code templates' names and paths can be found under the icie.template.list \
	               configuration entry."
)]
static SOLUTION_TEMPLATE: evscode::Config<String> = "C++";

fn init(root: &Path, url: Option<String>, meta: Option<TaskMeta>) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let examples = meta.map(|meta| meta.examples.unwrap_or_default()).unwrap_or_default();
	init_manifest(root, &url)?;
	init_template(root)?;
	init_examples(root, &examples)?;
	evscode::open_folder(root, false);
	Ok(())
}

fn init_manifest(root: &Path, url: &Option<String>) -> R<()> {
	let manifest = crate::manifest::Manifest::new_project(url.clone());
	manifest.save(root)?;
	Ok(())
}
fn init_template(root: &Path) -> R<()> {
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
fn init_examples(root: &Path, examples: &[Example]) -> R<()> {
	let examples_dir = root.join("tests").join("example");
	util::fs_create_dir_all(&examples_dir)?;
	for (i, test) in examples.iter().enumerate() {
		util::fs_write(examples_dir.join(format!("{}.in", i + 1)), &test.input)?;
		util::fs_write(examples_dir.join(format!("{}.out", i + 1)), &test.output)?;
	}
	Ok(())
}

#[evscode::command(title = "ICIE Init", key = "alt+f11")]
fn new() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let url = ask_task_url()?;
	let meta = fetch_task_meta(&url)?;
	let variables = InitVariableMap {
		task_symbol: meta.as_ref().map(|meta| meta.symbol.clone()),
		task_name: meta.as_ref().map(|meta| meta.name.clone()),
		contest_id: meta.as_ref().map(|meta| meta.contest_id.clone()),
		site_short: meta.as_ref().map(|meta| meta.site_short.clone()),
	};
	let (codename, all_good) = PROJECT_NAME_TEMPLATE.get().interpolate(&variables);
	let config_strategy = ASK_FOR_PATH.get();
	let strategy = match (&*config_strategy, all_good) {
		(_, false) => &PathDialog::InputBox,
		(s, true) => s,
	};
	let root = strategy.query(&*dir::PROJECT_DIRECTORY.get(), &codename)?;
	let dir = util::TransactionDir::new(&root)?;
	init(&root, url, meta)?;
	dir.commit();
	Ok(())
}

#[evscode::command(title = "ICIE Init existing")]
fn existing() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let url = ask_task_url()?;
	let meta = fetch_task_meta(&url)?;
	let root = evscode::workspace_root()?;
	init(&root, url, meta)?;
	Ok(())
}

struct TaskMeta {
	symbol: String,
	name: String,
	contest_id: String,
	site_short: String,
	examples: Option<Vec<Example>>,
}
fn fetch_task_meta(url: &Option<String>) -> R<Option<TaskMeta>> {
	let url = match url {
		Some(url) => url,
		None => return Ok(None),
	};
	let (sess, url, _) = crate::net::connect(&url)?;
	let meta = {
		let _status = crate::STATUS.push("Fetching task");
		sess.run(|sess| {
			let cont = sess.contest(&url.contest)?;
			let task = cont.task(&url.task)?;
			let details = task.details()?;
			Ok(TaskMeta {
				symbol: details.symbol,
				name: details.title,
				contest_id: details.contest_id,
				site_short: details.site_short,
				examples: details.examples,
			})
		})?
	};
	Ok(Some(meta))
}

fn ask_task_url() -> R<Option<String>> {
	Ok(evscode::InputBox::new()
		.prompt("Enter task URL or leave empty")
		.placeholder("https://codeforces.com/contest/.../problem/...")
		.ignore_focus_out()
		.build()
		.wait()
		.map(|url| if url.trim().is_empty() { None } else { Some(url) })
		.ok_or_else(E::cancel)?)
}

#[evscode::config(
	description = "Default project directory name. This key uses special syntax to allow using dynamic content, like task names. See example list:\n\n{task.symbol \
	               case.upper}-{task.name case.kebab} -> A-diverse-strings (default)\n{random.cute}-{random.animal} -> kawaii-hedgehog\n{site.short}/{contest.id \
	               case.kebab}/{task.symbol case.upper}-{task.name case.kebab} -> cf/1144/A-diverse-strings\n{task.symbol case.upper}-{{ -> A-{\n\n{random.cute} -> \
	               kawaii\n{random.animal} -> hedgehog\n{task.symbol} -> A\n{task.name} -> Diverse Strings\n{contest.id} -> 1144\n{site.short} -> cf\n\n{task.name} -> Diverse \
	               Strings\n{task.name case.camel} -> diverseStrings\n{task.name case.pascal} -> DiverseStrings\n{task.name case.snake} -> diverse_strings\n{task.name \
	               case.kebab} -> diverse-strings\n{task.name case.upper} -> DIVERSE_STRINGS"
)]
static PROJECT_NAME_TEMPLATE: evscode::Config<Interpolation<InitVariable>> = "{task.symbol case.upper}-{task.name case.kebab}".parse().unwrap();

#[evscode::config(
	description = "By default, when initializing a project, the project directory will be created in the directory determined by icie.dir.projectDirectory configuration entry, \
	               and the name will be chosen according to the icie.init.projectNameTemplate configuration entry. This options allows to instead specify the directory every \
	               time."
)]
static ASK_FOR_PATH: evscode::Config<PathDialog> = PathDialog::None;

#[derive(Debug, evscode::Configurable)]
enum PathDialog {
	#[evscode(name = "No")]
	None,
	#[evscode(name = "With a VS Code input box")]
	InputBox,
	#[evscode(name = "With a system dialog")]
	SystemDialog,
}

impl PathDialog {
	fn query(&self, directory: &Path, codename: &str) -> R<PathBuf> {
		let basic = format!("{}/{}", directory.to_str().unwrap(), codename);
		match self {
			PathDialog::None => Ok(PathBuf::from(basic)),
			PathDialog::InputBox => Ok(PathBuf::from(
				evscode::InputBox::new()
					.ignore_focus_out()
					.prompt("New project directory")
					.value(&basic)
					.value_selection(basic.len() - codename.len(), basic.len())
					.build()
					.wait()
					.ok_or_else(E::cancel)?,
			)),
			PathDialog::SystemDialog => Ok(evscode::OpenDialog::new().directory().action_label("Init").build().wait().ok_or_else(E::cancel)?),
		}
	}
}

enum InitVariable {
	RandomCute,
	RandomAnimal,
	TaskSymbol,
	TaskName,
	ContestId,
	SiteShort,
}
struct InitVariableMap {
	task_symbol: Option<String>,
	task_name: Option<String>,
	contest_id: Option<String>,
	site_short: Option<String>,
}

impl crate::interpolation::VariableSet for InitVariable {
	type Map = InitVariableMap;

	fn expand(&self, map: &Self::Map) -> Option<String> {
		match self {
			InitVariable::RandomCute => Some(crate::dir::random_adjective().to_owned()),
			InitVariable::RandomAnimal => Some(crate::dir::random_animal().to_owned()),
			InitVariable::TaskSymbol => map.task_symbol.clone(),
			InitVariable::TaskName => map.task_name.clone(),
			InitVariable::ContestId => map.contest_id.clone(),
			InitVariable::SiteShort => map.site_short.clone(),
		}
	}
}

impl FromStr for InitVariable {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"random.cute" => Ok(InitVariable::RandomCute),
			"random.animal" => Ok(InitVariable::RandomAnimal),
			"task.symbol" => Ok(InitVariable::TaskSymbol),
			"task.name" => Ok(InitVariable::TaskName),
			"contest.id" => Ok(InitVariable::ContestId),
			"site.short" => Ok(InitVariable::SiteShort),
			_ => Err(format!(
				"unrecognized variable name {:?}, see icie.init.projectNameTemplate for a full list of available variables",
				s
			)),
		}
	}
}

impl fmt::Display for InitVariable {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str(match self {
			InitVariable::RandomCute => "random.cute",
			InitVariable::RandomAnimal => "random.animal",
			InitVariable::TaskSymbol => "task.symbol",
			InitVariable::TaskName => "task.name",
			InitVariable::ContestId => "contest.id",
			InitVariable::SiteShort => "site.short",
		})
	}
}

#[test]
fn test_interpolate() {
	let vars = InitVariableMap {
		task_symbol: Some("A".to_owned()),
		task_name: Some("Diverse Strings".to_owned()),
		contest_id: Some("1144".to_owned()),
		site_short: Some("cf".to_owned()),
	};
	let expand = |pattern: &str| -> String {
		let interpolation: Interpolation<InitVariable> = pattern.parse().unwrap();
		assert_eq!(interpolation.to_string(), pattern);
		interpolation.interpolate(&vars).0
	};
	assert_eq!(expand("{task.symbol case.upper}-{task.name case.kebab}"), "A-diverse-strings");
	assert_eq!(
		expand("{site.short}/{contest.id case.kebab}/{task.symbol case.upper}-{task.name case.kebab}"),
		"cf/1144/A-diverse-strings"
	);
	assert_eq!(expand("{task.symbol case.upper}-{{"), "A-{");
	assert_eq!(expand("{task.symbol}"), "A");
	assert_eq!(expand("{task.name}"), "Diverse Strings");
	assert_eq!(expand("{contest.id}"), "1144");
	assert_eq!(expand("{site.short}"), "cf");
	assert_eq!(expand("{task.name}"), "Diverse Strings");
	assert_eq!(expand("{task.name case.camel}"), "diverseStrings");
	assert_eq!(expand("{task.name case.pascal}"), "DiverseStrings");
	assert_eq!(expand("{task.name case.snake}"), "diverse_strings");
	assert_eq!(expand("{task.name case.kebab}"), "diverse-strings");
	assert_eq!(expand("{task.name case.upper}"), "DIVERSE_STRINGS");
	assert_eq!(expand("{{task.name case.kebab}}"), "{task.name case.kebab}");
	assert_eq!(expand("cp{contest.id}{task.symbol case.kebab}-icie"), "cp1144a-icie");
}
