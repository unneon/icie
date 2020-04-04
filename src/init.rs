use crate::{
	dir::PROJECT_DIRECTORY, net::{self, BackendMeta}, telemetry::TELEMETRY, util::{fs, path::Path}
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

#[evscode::command(title = "ICIE Init Scan", key = "alt+f9")]
pub async fn scan() -> R<()> {
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
pub async fn url() -> R<()> {
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
			let projects_dir = PROJECT_DIRECTORY.get();
			let workspace = names::design_task_name(projects_dir.as_ref(), meta.as_ref()).await?;
			fs::create_dir_all(&workspace).await?;
			init_task(&workspace, raw_url, meta).await?;
			evscode::open_folder(workspace.as_str(), false).await;
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

async fn init_task(workspace: &Path, url: Option<String>, meta: Option<TaskDetails>) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	fs::create_dir_all(workspace).await?;
	let examples = meta
		.as_ref()
		.and_then(|meta| meta.examples.as_ref())
		.map(|examples| examples.as_slice())
		.unwrap_or(&[]);
	let statement = meta.as_ref().and_then(|meta| meta.statement.clone());
	files::init_manifest(workspace, &url, statement).await?;
	files::init_template(workspace).await?;
	files::init_examples(workspace, examples).await?;
	Ok(())
}
