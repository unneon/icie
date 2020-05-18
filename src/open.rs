use crate::{
	dir::PROJECT_DIRECTORY, net::{self, BackendMeta}, open::scan::ContestMeta, telemetry::TELEMETRY, util::fs
};
use evscode::{quick_pick, QuickPick, E, R};
use std::sync::Arc;
use unijudge::{
	boxed::{BoxedContestURL, BoxedTaskURL}, chrono::Local, Backend, ContestTime, Resource, TaskDetails, URL
};

pub mod contest;
mod files;
pub mod names;
mod scan;

enum Command {
	Task(Option<(BoxedTaskURL, &'static BackendMeta)>),
	Contest(BoxedContestURL, &'static BackendMeta),
}

#[evscode::command(title = "ICIE Open Scan", key = "alt+f9")]
pub async fn scan() -> R<()> {
	TELEMETRY.open_scan.spark();
	let mut contests = scan::fetch_contests().await;
	order_contests(&mut contests);
	let contest = select_contest(&contests).await?;
	TELEMETRY.open_scan_ok.spark();
	contest::sprint(contest.sess.clone(), &contest.details.id, Some(&contest.details.title)).await?;
	Ok(())
}

fn order_contests(contests: &mut [ContestMeta]) {
	contests.sort_by_key(|contest| match contest.details.time {
		ContestTime::Upcoming { start } => (0, start),
		ContestTime::Ongoing { finish } => (1, finish),
	});
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
	let site_prefix = contest.sess.backend.backend.contest_site_prefix();
	let label = if contest.details.title.starts_with(site_prefix) {
		contest.details.title.clone()
	} else {
		format!("{} {}", site_prefix, contest.details.title)
	};
	let time = match contest.details.time {
		ContestTime::Upcoming { start } => start,
		ContestTime::Ongoing { finish } => finish,
	};
	let time = time.with_timezone(&Local).to_rfc2822();
	let time = match contest.details.time {
		ContestTime::Upcoming { .. } => time,
		ContestTime::Ongoing { .. } => format!("[ONGOING] {}", time),
	};
	quick_pick::Item::new(index, label).description(time)
}

#[evscode::command(title = "ICIE Open URL", key = "alt+f11")]
pub async fn url() -> R<()> {
	let _status = crate::STATUS.push("Opening");
	TELEMETRY.open_url.spark();
	let raw_url = ask_url().await?;
	match Command::from_url(raw_url.as_ref())? {
		Command::Task(url) => {
			TELEMETRY.open_url_task.spark();
			let details = fetch_task_details(&url).await?;
			let projects_dir = PROJECT_DIRECTORY.get();
			let workspace = names::design_task_name(&projects_dir, details.as_ref()).await?;
			fs::create_dir_all(&workspace).await?;
			files::open_task(&workspace, raw_url, details).await?;
			evscode::open_folder(workspace.as_str(), false).await;
		},
		Command::Contest(url, backend) => {
			TELEMETRY.open_url_contest.spark();
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

impl Command {
	fn from_url(url: Option<&String>) -> R<Command> {
		Ok(match url {
			Some(raw_url) => {
				let (url, backend) = net::interpret_url(&raw_url)?;
				let URL { domain, site, resource } = url;
				match resource {
					Resource::Task(task) => {
						Command::Task(Some((URL { domain, site, resource: Resource::Task(task) }, backend)))
					},
					Resource::Contest(contest) => {
						Command::Contest(URL { domain, site, resource: Resource::Contest(contest) }, backend)
					},
				}
			},
			None => Command::Task(None),
		})
	}
}

async fn fetch_task_details(url: &Option<(BoxedTaskURL, &'static BackendMeta)>) -> R<Option<TaskDetails>> {
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
