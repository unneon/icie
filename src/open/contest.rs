use crate::{
	auth, compile::suggest_install_compiler, net::{interpret_url, require_contest, Session}, open::{
		files, names::{design_contest_name, design_task_name}
	}, util::{self, fs, path::Path, sleep, time_now, workspace_root}
};
use evscode::{error::ResultExt, E, R};
use futures::{select, FutureExt};
use serde::{Deserialize, Serialize};
use std::{
	cmp::min, sync::Arc, time::{Duration, SystemTime}
};
use unijudge::{
	boxed::{BoxedContest, BoxedContestDetails, BoxedTask}, Backend, ContestTime, ErrorCode, Resource, TaskDetails
};

/// Contains information about the contest necessary to start waiting for it to start. When created, this manifest is
/// saved to a file in a different directory. When ICIE notices this file exists, it reads it, deletes it and switches
/// into contest mode.
#[derive(Deserialize, Serialize)]
struct Manifest {
	contest_url: String,
}

const NOT_YET_STARTED_RETRY_LIMIT: usize = 15;
const NOT_YET_STARTED_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Wait for the contest, set up the first task, save a contest manifest and switch to the directory. This will likely
/// kill the extension process, so do not do anything important after calling this function. Post-setup steps will
/// either be done from this function or the extension activation function.
pub async fn sprint(sess: Arc<Session>, contest: &BoxedContest, contest_title: Option<&str>) -> R<()> {
	let _status = crate::STATUS.push("Opening");
	let contest_title = fetch_contest_title(&sess, contest, contest_title).await?;
	let projects = design_contest_name(&contest_title).await?;
	fs::create_dir_all(&projects).await?;
	let url_raw = sess.backend.backend.contest_url(contest);
	let url = require_contest(interpret_url(&url_raw)?.0)?;
	wait_for_contest(&url_raw, &url.site, &sess).await?;
	let Resource::Contest(contest) = url.resource;
	let tasks = fetch_tasks(&sess, &contest).await?;
	let task0 = tasks.get(0).wrap("could not find any tasks in contest")?;
	let task0_path = open_task(task0, 0, tasks.len(), &projects, &sess).await?;
	create_contest_manifest(&task0_path, &url_raw).await?;
	evscode::open_folder(task0_path.as_str(), false).await;
	Ok(())
}

async fn fetch_contest_title(sess: &Session, contest: &BoxedContest, title: Option<&str>) -> R<String> {
	let title = match title {
		Some(title) => title.to_owned(),
		None => sess.run(|backend, sess| backend.contest_title(sess, contest)).await?,
	};
	Ok(title)
}

async fn wait_for_contest(url: &str, site: &str, sess: &Arc<Session>) -> R<()> {
	let details = match fetch_contest_details(url, sess).await? {
		Some(details) => details,
		None => return Ok(()),
	};
	let deadline = match details.time {
		ContestTime::Upcoming { start } => SystemTime::from(start),
		ContestTime::Ongoing { .. } => return Ok(()),
	};
	let total = match deadline.duration_since(time_now()) {
		Ok(total) => total,
		Err(_) => return Ok(()),
	};
	let _status = crate::STATUS.push("Waiting");
	let (progress, on_cancel) =
		evscode::Progress::new().title(format!("Waiting for {}", details.title)).cancellable().show();
	let mut on_cancel = on_cancel.boxed().fuse();
	spawn_suggest_login(site, sess);
	spawn_suggest_install_compiler();
	while let Ok(left) = deadline.duration_since(time_now()) {
		let left_ratio = left.as_millis() as f64 / total.as_millis() as f64;
		progress.update_set(100. * (1. - left_ratio), util::fmt::time_left(left));
		let delay = min(left, Duration::from_secs(1));
		let mut delay = Box::pin(sleep(delay).fuse());
		select! {
			() = delay => (),
			() = on_cancel => return Err(E::cancel()),
		}
	}
	progress.end();
	Ok(())
}

async fn fetch_contest_details(url: &str, sess: &Session) -> R<Option<BoxedContestDetails>> {
	sess.run(|backend, sess| async move {
		let contests = backend.contests(sess).await?;
		let details = contests.into_iter().find(|details| backend.contest_url(&details.id) == url);
		Ok(details)
	})
	.await
}

fn spawn_suggest_login(site: &str, sess: &Arc<Session>) {
	let site = site.to_owned();
	let sess = sess.clone();
	evscode::spawn(async move { suggest_login(&site, &sess).await });
}

async fn suggest_login(site: &str, sess: &Session) -> R<()> {
	if !auth::has_any_saved(site).await {
		let message = format!("You are not logged in to {}, maybe do it now to save time when submitting?", site);
		let dec = evscode::Message::new(&message).item((), "Log in", false).warning().show().await;
		if dec.is_some() {
			sess.force_login().await?;
			evscode::Message::new::<()>("Logged in successfully").show().await;
		}
	}
	Ok(())
}

fn spawn_suggest_install_compiler() {
	evscode::spawn(suggest_install_compiler());
}

async fn open_task(task: &BoxedTask, index: usize, count: usize, projects: &Path, sess: &Session) -> R<Path> {
	let name = format!("{}/{}", index + 1, count);
	let details = fetch_task(task, &name, sess).await?;
	let url = sess.run(|backend, sess| async move { backend.task_url(sess, task) }).await?;
	let workspace = design_task_name(projects, Some(&details)).await?;
	files::open_task(&workspace, Some(url), Some(details)).await?;
	Ok(workspace)
}

async fn fetch_task(task: &BoxedTask, name: &str, sess: &Session) -> R<TaskDetails> {
	let _status = crate::STATUS.push(format!("Fetching task {}", name));
	sess.run(|backend, sess| backend.task_details(sess, task)).await
}

async fn create_contest_manifest(workspace: &Path, contest_url: &str) -> R<()> {
	let path = workspace.join(".icie-contest");
	let manifest_data = Manifest { contest_url: contest_url.to_owned() };
	let manifest = serde_json::to_string(&manifest_data).wrap("serialization of contest manifest failed")?;
	fs::write(&path, manifest).await?;
	Ok(())
}

/// Check if a contest manifest exists, and if it does, start the rest of the contest setup.
pub async fn check_for_manifest() -> R<()> {
	if let Ok(workspace) = workspace_root() {
		let manifest = workspace.join(".icie-contest");
		if fs::exists(&manifest).await? {
			open_remaining_tasks(&manifest).await?;
		}
	}
	Ok(())
}

/// Do the setup for the rest of the contest tasks.
async fn open_remaining_tasks(manifest: &Path) -> R<()> {
	let _status = crate::STATUS.push("Opening");
	let manifest = pop_manifest(manifest).await?;
	let (url, backend) = interpret_url(&manifest.contest_url)?;
	let url = require_contest(url)?;
	let sess = Session::connect(&url.domain, backend).await?;
	let Resource::Contest(contest) = url.resource;
	let tasks = fetch_tasks(&sess, &contest).await?;
	let projects = workspace_root()?.parent();
	for (i, task) in tasks.iter().enumerate() {
		if i > 0 {
			open_task(task, i, tasks.len(), &projects, &sess).await?;
		}
	}
	Ok(())
}

/// Parse the manifest and removes it.
async fn pop_manifest(path: &Path) -> R<Manifest> {
	let manifest = serde_json::from_str(&fs::read_to_string(path).await?).wrap("malformed contest manifest")?;
	fs::remove_file(path).await.map_err(|e| e.context("could not delete contest manifest after use"))?;
	Ok(manifest)
}

async fn fetch_tasks(sess: &Session, contest: &BoxedContest) -> R<Vec<BoxedTask>> {
	let _status = crate::STATUS.push("Fetching contest");
	let mut wait_retries = NOT_YET_STARTED_RETRY_LIMIT;
	sess.run(|backend, sess| async move {
		loop {
			match backend.contest_tasks(sess, contest).await {
				Err(e) if e.code == ErrorCode::NetworkFailure && wait_retries > 0 => {
					let fmt_retries = util::fmt::plural(wait_retries, "retry", "retries");
					let _status = crate::STATUS.push(format!("Waiting for contest start, {} left", fmt_retries));
					wait_retries -= 1;
					sleep(NOT_YET_STARTED_RETRY_DELAY).await;
				},
				tasks => break tasks,
			}
		}
	})
	.await
}
