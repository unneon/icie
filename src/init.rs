use crate::{
	interpolation::Interpolation, net::{self, Backend}, util::{self, fs_create_dir_all}
};
use evscode::{quick_pick, QuickPick, E, R};
use std::path::{Path, PathBuf};
use unijudge::{
	boxed::{BoxedContestURL, BoxedTaskURL}, chrono::Local, Resource, TaskDetails, URL
};

pub mod contest;
mod files;
pub mod names;
mod scan;

/// The name of the code template used for initializing new projects. The list of code templates' names and paths can be found under the icie.template.list configuration entry.
#[evscode::config]
static SOLUTION_TEMPLATE: evscode::Config<String> = "C++";

#[evscode::command(title = "ICIE Init Scan", key = "alt+f9")]
fn scan() -> R<()> {
	let mut contests = scan::fetch_contests();
	contests.sort_by_key(|contest| contest.1.start);
	let pick = QuickPick::new()
		.items(contests.iter().enumerate().map(|(index, (_, contest))| {
			let start = contest.start.with_timezone(&Local).to_rfc2822();
			quick_pick::Item::new(index.to_string(), &contest.title).description(start)
		}))
		.match_on_description()
		.build()
		.wait()
		.ok_or_else(E::cancel)?;
	let (sess, contest) = &contests[pick.parse::<usize>().unwrap()];
	contest::setup_sprint(&*sess, &contest.id)?;
	Ok(())
}

#[evscode::command(title = "ICIE Init URL", key = "alt+f11")]
fn url() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let raw_url = ask_url()?;
	match url_to_command(raw_url.as_ref())? {
		InitCommand::Task(url) => {
			let meta = url.map(|(url, backend)| fetch_task_details(url, backend)).transpose()?;
			let root = names::design_task_name(&*crate::dir::PROJECT_DIRECTORY.get(), meta.as_ref())?;
			let dir = util::TransactionDir::new(&root)?;
			init_task(&root, raw_url, meta)?;
			dir.commit();
			evscode::open_folder(root, false);
		},
		InitCommand::Contest { url, backend } => {
			let sess = net::Session::connect(&url, backend)?;
			let Resource::Contest(contest) = url.resource;
			contest::setup_sprint(&sess, &contest)?;
		},
	}
	Ok(())
}

#[evscode::command(title = "ICIE Init URL (current directory)")]
fn url_existing() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let raw_url = ask_url()?;
	let url = match url_to_command(raw_url.as_ref())? {
		InitCommand::Task(task) => task,
		InitCommand::Contest { .. } => return Err(E::error("it is forbidden to init a contest in an existing directory")),
	};
	let meta = url.map(|(url, backend)| fetch_task_details(url, backend)).transpose()?;
	let root = evscode::workspace_root()?;
	init_task(&root, raw_url, meta)?;
	Ok(())
}

fn ask_url() -> R<Option<String>> {
	Ok(evscode::InputBox::new()
		.prompt("Enter task/contest URL or leave empty")
		.placeholder("https://codeforces.com/contest/.../problem/...")
		.ignore_focus_out()
		.build()
		.wait()
		.map(|url| if url.trim().is_empty() { None } else { Some(url) })
		.ok_or_else(E::cancel)?)
}

#[allow(unused)]
enum InitCommand {
	Task(Option<(BoxedTaskURL, &'static Backend)>),
	Contest { url: BoxedContestURL, backend: &'static Backend },
}
fn url_to_command(url: Option<&String>) -> R<InitCommand> {
	Ok(match url {
		Some(raw_url) => {
			let (URL { domain, site, resource }, backend) = net::interpret_url(&raw_url)?;
			match resource {
				Resource::Task(task) => InitCommand::Task(Some((URL { domain, site, resource: Resource::Task(task) }, backend))),
				Resource::Contest(contest) => InitCommand::Contest { url: URL { domain, site, resource: Resource::Contest(contest) }, backend },
			}
		},
		None => InitCommand::Task(None),
	})
}

fn fetch_task_details(url: BoxedTaskURL, backend: &'static Backend) -> R<TaskDetails> {
	let Resource::Task(task) = &url.resource;
	let sess = net::Session::connect(&url, backend)?;
	let meta = {
		let _status = crate::STATUS.push("Fetching task");
		sess.run(|sess| sess.task_details(&task))?
	};
	Ok(meta)
}

fn init_task(root: &Path, url: Option<String>, meta: Option<TaskDetails>) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	fs_create_dir_all(root)?;
	let examples = meta.as_ref().and_then(|meta| meta.examples.as_ref()).map(|examples| examples.as_slice()).unwrap_or(&[]);
	let statement = meta.as_ref().and_then(|meta| meta.statement.clone());
	files::init_manifest(root, &url, statement)?;
	files::init_template(root)?;
	files::init_examples(root, examples)?;
	Ok(())
}

/// Default project directory name. This key uses special syntax to allow using dynamic content, like task names. See example list:
///
/// {task.symbol case.upper}-{task.name case.kebab} -> A-diverse-strings (default)
/// {random.cute}-{random.animal} -> kawaii-hedgehog
/// {site.short}/{contest.id case.kebab}/{task.symbol case.upper}-{task.name case.kebab} -> cf/1144/A-diverse-strings
/// {task.symbol case.upper}-{{ -> A-{
///
/// {random.cute} -> kawaii
/// {random.animal} -> hedgehog
/// {task.symbol} -> A
/// {task.name} -> Diverse Strings
/// {contest.id} -> 1144
/// {site.short} -> cf
///
/// {task.name} -> Diverse Strings
/// {task.name case.camel} -> diverseStrings
/// {task.name case.pascal} -> DiverseStrings
/// {task.name case.snake} -> diverse_strings
/// {task.name case.kebab} -> diverse-strings
/// {task.name case.upper} -> DIVERSE_STRINGS
#[evscode::config]
static PROJECT_NAME_TEMPLATE: evscode::Config<Interpolation<names::TaskVariable>> =
	"{task.symbol case.upper}-{task.name case.kebab}".parse().unwrap();

/// By default, when initializing a project, the project directory will be created in the directory determined by icie.dir.projectDirectory configuration entry, and the name will be chosen according to the icie.init.projectNameTemplate configuration entry. This option allows to instead specify the directory every time.
#[evscode::config]
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
