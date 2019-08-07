use crate::{
	auth, interpolation::Interpolation, net::{self, Backend}, util
};
use evscode::{quick_pick, QuickPick, E, R};
use std::{
	path::{Path, PathBuf}, sync::Arc, time::{Duration, SystemTime}
};
use unijudge::{
	boxed::{BoxedContest, BoxedContestDetails, BoxedContestURL, BoxedTask, BoxedTaskURL}, chrono::Local, Resource, TaskDetails, URL
};

mod files;
pub mod names;
mod scan;

/// The name of the code template used for initializing new projects. The list of code templates' names and paths can be found under the icie.template.list configuration entry.
#[evscode::config]
static SOLUTION_TEMPLATE: evscode::Config<String> = "C++";

#[evscode::command(title = "ICIE Init Scan", key = "alt+f9")]
fn scan() -> R<()> {
	#[evscode::status("Fetching")]
	let mut contests = scan::fetch_contests()?;
	contests.sort_by_key(|contest| contest.1.start);
	#[evscode::status("Picking contest")]
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
	wait_for_contest(contest, &sess.site, sess)?;
	start_contest(&*sess, &contest.id)?;
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
			start_contest(&sess, &contest)?;
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

fn wait_for_contest(contest: &BoxedContestDetails, site: &str, sess: &Arc<net::Session>) -> R<()> {
	let deadline = SystemTime::from(contest.start);
	let total = match deadline.duration_since(SystemTime::now()) {
		Ok(total) => total,
		Err(_) => return Ok(()),
	};
	let _status = crate::STATUS.push("Waiting for contest");
	let progress = evscode::Progress::new().title(format!("Waiting for {}", contest.title)).cancellable().show();
	let canceler = progress.canceler().spawn();
	let site = site.to_owned();
	let sess = sess.clone();
	evscode::internal::executor::spawn(move || {
		if !auth::has_any_saved(&site) {
			if evscode::Message::new(format!("You are not logged in to {}, maybe do it now to save time when submitting?", site))
				.item("log-in", "Log in", false)
				.build()
				.wait()
				.is_some()
			{
				let _status = crate::STATUS.push("Logging in");
				sess.force_login()?;
				evscode::Message::new("Logged in successfully").build().spawn();
			}
		}
		Ok(())
	});
	loop {
		if canceler.try_wait().is_some() {
			return Err(E::cancel());
		}
		let now = SystemTime::now();
		let left = match deadline.duration_since(now) {
			Ok(left) => left,
			Err(_) => break,
		};
		progress.update_set(100.0 - 100.0 * left.as_millis() as f64 / total.as_millis() as f64, fmt_time_left(left));
		std::thread::sleep(Duration::from_millis(1000));
	}
	progress.end();
	Ok(())
}
fn fmt_time_left(mut t: Duration) -> String {
	let mut s = {
		let x = t.as_secs() % 60;
		t -= Duration::from_secs(x);
		format!("{} seconds left", x)
	};
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 % 60;
		t -= Duration::from_secs(x * 60);
		s = format!("{} minutes, {}", x, s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 % 24;
		t -= Duration::from_secs(x * 60 * 60);
		s = format!("{} hours, {}", x, s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 / 24;
		t -= Duration::from_secs(x * 60 * 60 * 24);
		s = format!("{} days, {}", x, s)
	}
	s
}

fn start_contest(sess: &net::Session, contest: &BoxedContest) -> R<()> {
	#[evscode::status("Fetching contest")]
	let meta = sess.run(|sess| sess.contest_tasks(&contest))?;
	let (contest_id, site_short) = sess.run(|sess| Ok((sess.contest_id(&contest)?, sess.site_short())))?;
	let root = names::design_contest_name(&contest_id, site_short)?;
	let dir = util::TransactionDir::new(&root)?;
	let root_task = init_contest(&root, &meta, &sess)?;
	dir.commit();
	evscode::open_folder(root_task, false);
	Ok(())
}

fn fetch_task_details(url: BoxedTaskURL, backend: &'static Backend) -> R<TaskDetails> {
	let Resource::Task(task) = &url.resource;
	let sess = net::Session::connect(&url, backend)?;
	#[evscode::status("Fetching task")]
	let meta = sess.run(|sess| sess.task_details(&task))?;
	Ok(meta)
}

fn init_task(root: &Path, url: Option<String>, meta: Option<TaskDetails>) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let examples = meta.as_ref().and_then(|meta| meta.examples.as_ref()).map(|examples| examples.as_slice()).unwrap_or(&[]);
	let statement = meta.as_ref().and_then(|meta| meta.statement.clone());
	files::init_manifest(root, &url, statement)?;
	files::init_template(root)?;
	files::init_examples(root, examples)?;
	Ok(())
}
fn init_contest(root: &Path, tasks: &[BoxedTask], sess: &net::Session) -> R<PathBuf> {
	let tasks: Vec<(TaskDetails, PathBuf)> = tasks
		.iter()
		.enumerate()
		.map(|(index, task)| {
			#[evscode::status("Fetching task {}/{}", index+1, tasks.len())]
			let details = sess.run(|sess| sess.task_details(task))?;
			let root = names::design_task_name(root, Some(&details))?;
			Ok((details, root))
		})
		.collect::<R<Vec<_>>>()?;
	let first_root = tasks[0].1.clone();
	for task in tasks {
		util::fs_create_dir_all(&task.1)?;
		init_task(&task.1, Some(task.0.url.clone()), Some(task.0))?;
	}
	Ok(first_root)
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
