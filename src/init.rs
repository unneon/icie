use crate::{
	dir::PROJECT_DIRECTORY, init::scan::ContestMeta, net::{self, BackendMeta}, telemetry::TELEMETRY, util::fs
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

enum InitCommand {
	Task(Option<(BoxedTaskURL, &'static BackendMeta)>),
	Contest(BoxedContestURL, &'static BackendMeta),
}

#[evscode::command(title = "ICIE Init Scan", key = "alt+f9")]
pub async fn scan() -> R<()> {
	TELEMETRY.init_scan.spark();
	let mut contests = scan::fetch_contests().await;
	contests.sort_by_key(|contest| contest.details.start);
	let contest = select_contest(&contests).await?;
	TELEMETRY.init_scan_ok.spark();
	contest::sprint(contest.sess.clone(), &contest.details.id, Some(&contest.details.title))
		.await?;
	Ok(())
}

async fn select_contest(contests: &[ContestMeta]) -> R<&ContestMeta> {
	let pick = QuickPick::new()
		.items(contests.iter().enumerate().map(fmt_contest_pick))
		.match_on_description()
		.ignore_focus_out()
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let contest = &contests[pick];
	Ok(contest)
}

fn fmt_contest_pick((index, contest): (usize, &ContestMeta)) -> quick_pick::Item<usize> {
	let site_prefix = contest.sess.backend.contest_site_prefix();
	let label = if contest.details.title.starts_with(site_prefix) {
		contest.details.title.clone()
	} else {
		format!("{} {}", site_prefix, contest.details.title)
	};
	let start = contest.details.start.with_timezone(&Local).to_rfc2822();
	quick_pick::Item::new(index, label).description(start)
}

#[evscode::command(title = "ICIE Init URL", key = "alt+f11")]
pub async fn url() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	TELEMETRY.init_url.spark();
	let raw_url = ask_url().await?;
	match InitCommand::from_url(raw_url.as_ref())? {
		InitCommand::Task(url) => {
			TELEMETRY.init_url_task.spark();
			let details = fetch_task_details(&url).await?;
			let projects_dir = PROJECT_DIRECTORY.get();
			let workspace = names::design_task_name(&projects_dir, details.as_ref()).await?;
			fs::create_dir_all(&workspace).await?;
			files::init_task(&workspace, raw_url, details).await?;
			evscode::open_folder(workspace.as_str(), false).await;
		},
		InitCommand::Contest(url, backend) => {
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

impl InitCommand {
	fn from_url(url: Option<&String>) -> R<InitCommand> {
		Ok(match url {
			Some(raw_url) => {
				let (url, backend) = net::interpret_url(&raw_url)?;
				let URL { domain, site, resource } = url;
				match resource {
					Resource::Task(task) => InitCommand::Task(Some((
						URL { domain, site, resource: Resource::Task(task) },
						backend,
					))),
					Resource::Contest(contest) => InitCommand::Contest(
						URL { domain, site, resource: Resource::Contest(contest) },
						backend,
					),
				}
			},
			None => InitCommand::Task(None),
		})
	}
}

async fn fetch_task_details(
	url: &Option<(BoxedTaskURL, &'static BackendMeta)>,
) -> R<Option<TaskDetails>> {
	match url {
		Some((url, backend)) => {
			let _status = crate::STATUS.push("Fetching task");
			let Resource::Task(task) = &url.resource;
			let sess = net::Session::connect(&url.domain, backend).await?;
			let details = sess.run(|backend, sess| backend.task_details(sess, &task)).await?;
			Ok(Some(details))
		},
		None => Ok(None),
	}
}
