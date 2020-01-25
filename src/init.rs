use crate::{
	interpolation::Interpolation, net::{self, BackendMeta}, telemetry::TELEMETRY, util::{fs, path::Path}
};
use evscode::{quick_pick, QuickPick, E, R};
use std::sync::Arc;
use unijudge::{
	boxed::{BoxedContestURL, BoxedTaskURL}, chrono::Local, Backend, Resource, TaskDetails, URL
};

pub mod contest;
mod files;
pub mod names;
mod scan;

/// The name of the code template used for initializing new projects. The list of code templates'
/// names and paths can be found under the icie.template.list configuration entry.
#[evscode::config]
static SOLUTION_TEMPLATE: evscode::Config<String> = "C++";

#[evscode::command(title = "ICIE Init Scan", key = "alt+f9")]
async fn scan() -> R<()> {
	TELEMETRY.init_scan.spark();
	let mut contests = scan::fetch_contests().await;
	contests.sort_by_key(|contest| contest.1.start);
	let pick = QuickPick::new()
		.items(contests.iter().enumerate().map(|(index, (sess, contest, _))| {
			let site_prefix = sess.backend.contest_site_prefix();
			let label = if contest.title.starts_with(site_prefix) {
				contest.title.clone()
			} else {
				format!("{} {}", site_prefix, contest.title)
			};
			let start = contest.start.with_timezone(&Local).to_rfc2822();
			quick_pick::Item::new(index.to_string(), label).description(start)
		}))
		.match_on_description()
		.ignore_focus_out()
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let (sess, contest, _) = &contests[pick.parse::<usize>().unwrap()];
	TELEMETRY.init_scan_ok.spark();
	contest::sprint(sess.clone(), &contest.id, Some(&contest.title)).await?;
	Ok(())
}

#[evscode::command(title = "ICIE Init URL", key = "alt+f11")]
async fn url() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	TELEMETRY.init_url.spark();
	let raw_url = ask_url().await?;
	match url_to_command(raw_url.as_ref())? {
		InitCommand::Task(url) => {
			TELEMETRY.init_url_task.spark();
			let meta = match url {
				Some((url, backend)) => Some(fetch_task_details(url, backend).await?),
				None => None,
			};
			let root = crate::dir::PROJECT_DIRECTORY.get();
			let root = names::design_task_name(root.as_ref(), meta.as_ref()).await?;
			fs::create_dir_all(root.as_ref()).await?;
			init_task(root.as_ref(), raw_url, meta).await?;
			evscode::open_folder(root.to_str().unwrap(), false).await;
		},
		InitCommand::Contest { url, backend } => {
			TELEMETRY.init_url_contest.spark();
			let sess = net::Session::connect(&url.domain, backend).await?;
			let Resource::Contest(contest) = url.resource;
			drop(_status);
			contest::sprint(Arc::new(sess), &contest, None).await?;
		},
	}
	Ok(())
}

#[evscode::command(title = "ICIE Init URL (current directory)")]
async fn url_existing() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	TELEMETRY.init_url_existing.spark();
	let raw_url = ask_url().await?;
	let url = match url_to_command(raw_url.as_ref())? {
		InitCommand::Task(task) => task,
		InitCommand::Contest { .. } => {
			return Err(E::error("it is forbidden to init a contest in an existing directory"));
		},
	};
	let meta = match url {
		Some((url, backend)) => Some(fetch_task_details(url, backend).await?),
		None => None,
	};
	let root = Path::from_native(evscode::workspace_root()?);
	init_task(root.as_ref(), raw_url, meta).await?;
	Ok(())
}

async fn ask_url() -> R<Option<String>> {
	Ok(evscode::InputBox::new()
		.prompt("Enter task/contest URL or leave empty")
		.placeholder("https://codeforces.com/contest/.../problem/...")
		.ignore_focus_out()
		.show()
		.await
		.map(|url| if url.trim().is_empty() { None } else { Some(url) })
		.ok_or_else(E::cancel)?)
}

#[allow(unused)]
enum InitCommand {
	Task(Option<(BoxedTaskURL, &'static BackendMeta)>),
	Contest { url: BoxedContestURL, backend: &'static BackendMeta },
}
fn url_to_command(url: Option<&String>) -> R<InitCommand> {
	Ok(match url {
		Some(raw_url) => {
			let (URL { domain, site, resource }, backend) = net::interpret_url(&raw_url)?;
			match resource {
				Resource::Task(task) => InitCommand::Task(Some((
					URL { domain, site, resource: Resource::Task(task) },
					backend,
				))),
				Resource::Contest(contest) => InitCommand::Contest {
					url: URL { domain, site, resource: Resource::Contest(contest) },
					backend,
				},
			}
		},
		None => InitCommand::Task(None),
	})
}

async fn fetch_task_details(url: BoxedTaskURL, backend: &'static BackendMeta) -> R<TaskDetails> {
	let Resource::Task(task) = &url.resource;
	let sess = net::Session::connect(&url.domain, backend).await?;
	let meta = {
		let _status = crate::STATUS.push("Fetching task");
		sess.run(|backend, sess| backend.task_details(sess, &task)).await?
	};
	Ok(meta)
}

async fn init_task(root: &Path, url: Option<String>, meta: Option<TaskDetails>) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	fs::create_dir_all(root).await?;
	let examples = meta
		.as_ref()
		.and_then(|meta| meta.examples.as_ref())
		.map(|examples| examples.as_slice())
		.unwrap_or(&[]);
	let statement = meta.as_ref().and_then(|meta| meta.statement.clone());
	files::init_manifest(root, &url, statement).await?;
	files::init_template(root).await?;
	files::init_examples(root, examples).await?;
	Ok(())
}

pub async fn help_init() -> R<()> {
	evscode::open_external("https://github.com/pustaczek/icie/blob/master/README.md#quick-start")
		.await
}

/// Default project directory name. This key uses special syntax to allow using dynamic content,
/// like task names. Variable contest.title is not available in this context. See example list:
///
/// {task.symbol case.upper}-{task.name case.kebab} -> A-diverse-strings (default)
/// {site.short}/{contest.id case.kebab}/{task.symbol case.upper}-{task.name case.kebab} ->
/// cf/1144/A-diverse-strings {task.symbol case.upper}-{{ -> A-{
///
/// {task.symbol} -> A
/// {task.name} -> Diverse Strings
/// {contest.id} -> 1144
/// {contest.title} -> Codeforces Round #550 (Div. 3)
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

/// By default, when initializing a project, the project directory will be created in the directory
/// determined by icie.dir.projectDirectory configuration entry, and the name will be chosen
/// according to the icie.init.projectNameTemplate configuration entry. This option allows to
/// instead specify the directory every time.
#[evscode::config]
static ASK_FOR_PATH: evscode::Config<PathDialog> = PathDialog::None;

#[derive(Clone, Debug, PartialEq, Eq, evscode::Configurable)]
enum PathDialog {
	#[evscode(name = "No")]
	None,
	#[evscode(name = "With a VS Code input box")]
	InputBox,
	#[evscode(name = "With a system dialog")]
	SystemDialog,
}

impl PathDialog {
	async fn query(&self, directory: &Path, codename: &str) -> R<Path> {
		let basic = directory.join(codename);
		match self {
			PathDialog::None => Ok(basic),
			PathDialog::InputBox => Ok(Path::from_native(
				evscode::InputBox::new()
					.ignore_focus_out()
					.prompt("New project directory")
					.value(basic.to_str().unwrap())
					.value_selection(
						basic.to_str().unwrap().len() - codename.len(),
						basic.to_str().unwrap().len(),
					)
					.show()
					.await
					.ok_or_else(E::cancel)?,
			)),
			PathDialog::SystemDialog => Ok(Path::from_native(
				evscode::OpenDialog::new()
					.directory()
					.action_label("Init")
					.show()
					.await
					.ok_or_else(E::cancel)?,
			)),
		}
	}
}
