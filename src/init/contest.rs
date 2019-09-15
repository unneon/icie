use crate::{
	auth, init::{
		init_task, names::{design_contest_name, design_task_name}
	}, net::{interpret_url, require_contest, Session}, telemetry::TELEMETRY, util::{fmt_time_left, fs_read_to_string, fs_write, plural, TransactionDir}
};
use evscode::{error::ResultExt, E, R};
use futures::{select, FutureExt};
use serde::{Deserialize, Serialize};
use std::{
	cmp::min, path::Path, sync::Arc, time::{Duration, Instant, SystemTime}
};
use tokio::timer::delay;
use unijudge::{
	boxed::{BoxedContest, BoxedTask}, Backend, Resource, TaskDetails
};

/// Wait for the contest, set up the first task, save a contest manifest and switch to the directory.  
/// This will likely kill the extension process, so do not do anything important after calling this function.
/// Post-setup steps will either be done from this function or the extension activation function.
pub async fn sprint(sess: Arc<Session>, contest: &BoxedContest, contest_title: Option<&str>) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let root_dir = design_contest_name(
		sess.backend.contest_id(contest),
		match contest_title {
			Some(title) => title.to_owned(),
			None => sess.run(|backend, sess| backend.contest_title(sess, contest)).await?,
		},
		sess.backend.name_short(),
	)
	.await?;
	let root_dir = TransactionDir::new(&root_dir).await?;
	let url_raw = sess.backend.contest_url(contest);
	let (url, _) = interpret_url(&url_raw)?;
	let url = require_contest(url)?;
	wait_for_contest(&url_raw, &url.site, &sess).await?;
	let Resource::Contest(contest) = url.resource;
	let tasks = fetch_tasks(&sess, &contest).await?;
	let task0 = tasks.get(0).wrap("could not find any tasks in contest")?;
	let task0_name = format!("1/{}", tasks.len());
	let task0_details = fetch_task(task0, &task0_name, &sess).await?;
	let task0_url = sess.run(|backend, sess| async move { backend.task_url(sess, task0) }).await?;
	let task0_path = design_task_name(root_dir.path(), Some(&task0_details)).await?;
	init_task(&task0_path, Some(task0_url), Some(task0_details)).await?;
	let manifest = Manifest { contest_url: url_raw };
	let manifest_path = task0_path.join(".icie-contest");
	fs_write(&manifest_path, serde_json::to_string(&manifest).wrap("serialization of contest manifest failed")?).await?;
	root_dir.commit();
	evscode::open_folder(task0_path, false);
	Ok(())
}

/// Check if a contest manifest exists, and if it does, start the rest of the contest setup.
pub async fn check_for_manifest() -> R<()> {
	if let Ok(workspace) = evscode::workspace_root() {
		let manifest = workspace.join(".icie-contest");
		if manifest.exists() {
			inner_sprint(&manifest).await?;
		}
	}
	Ok(())
}

/// Do the setup for the rest of the contest tasks.
async fn inner_sprint(manifest: &Path) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let manifest = pop_manifest(manifest).await?;
	let (url, backend) = interpret_url(&manifest.contest_url)?;
	let url = require_contest(url)?;
	let sess = Session::connect(&url.domain, backend.backend).await?;
	let Resource::Contest(contest) = url.resource;
	let tasks = fetch_tasks(&sess, &contest).await?;
	let task_dir = evscode::workspace_root()?;
	let contest_dir = task_dir.parent().wrap("task directory has no parent contest directory")?;
	for (i, task) in tasks.iter().enumerate() {
		if i > 0 {
			let taski_name = format!("{}/{}", i + 1, tasks.len());
			let details = fetch_task(task, &taski_name, &sess).await?;
			let root = design_task_name(contest_dir, Some(&details)).await?;
			init_task(&root, Some(sess.run(|_, _| async { sess.backend.task_url(&sess.session, &task) }).await?), Some(details)).await?;
		}
	}
	Ok(())
}

async fn fetch_task(task: &BoxedTask, name: &str, sess: &Session) -> R<TaskDetails> {
	let _status = crate::STATUS.push(format!("Fetching task {}", name));
	sess.run(|backend, sess| backend.task_details(sess, task)).await
}

async fn wait_for_contest(url: &str, site: &str, sess: &Arc<Session>) -> R<()> {
	let details = match sess
		.run(|backend, sess| async move { Ok(backend.contests(sess).await?.into_iter().find(|details| backend.contest_url(&details.id) == url)) })
		.await?
	{
		Some(details) => details,
		None => return Ok(()),
	};
	let deadline = SystemTime::from(details.start);
	let total = match deadline.duration_since(SystemTime::now()) {
		Ok(total) => total,
		Err(_) => return Ok(()),
	};
	TELEMETRY.init_countdown.spark();
	let _status = crate::STATUS.push("Waiting");
	let progress = evscode::Progress::new().title(format!("Waiting for {}", details.title)).cancellable().show();
	let mut on_cancel = progress.on_cancel().boxed().fuse();
	spawn_login_suggestion(site, sess);
	loop {
		let now = SystemTime::now();
		let left = match deadline.duration_since(now) {
			Ok(left) => left,
			Err(_) => break,
		};
		progress.update_set(100.0 - 100.0 * left.as_millis() as f64 / total.as_millis() as f64, fmt_time_left(left));
		let mut delay = tokio::timer::delay(Instant::now() + min(left, Duration::from_secs(1))).fuse();
		select! {
			() = delay => (),
			() = on_cancel => return Err(E::cancel()),
		}
	}
	drop(on_cancel);
	progress.end();
	TELEMETRY.init_countdown_ok.spark();
	Ok(())
}

/// Parse the manifest and removes it.
async fn pop_manifest(path: &Path) -> R<Manifest> {
	let manifest = serde_json::from_str(&fs_read_to_string(path).await?).wrap("malformed contest manifest")?;
	tokio::fs::remove_file(path).await.wrap("could not delete contest manifest after use")?;
	Ok(manifest)
}

const NOT_YET_STARTED_RETRY_LIMIT: usize = 15;
const NOT_YET_STARTED_RETRY_DELAY: Duration = Duration::from_secs(1);

async fn fetch_tasks(sess: &Session, contest: &BoxedContest) -> R<Vec<BoxedTask>> {
	let _status = crate::STATUS.push("Fetching contest");
	let mut wait_retries = NOT_YET_STARTED_RETRY_LIMIT;
	sess.run(|backend, sess| {
		async move {
			loop {
				match backend.contest_tasks(sess, &contest).await {
					Err(unijudge::Error::NotYetStarted) if wait_retries > 0 => {
						let _status = crate::STATUS.push(format!("Waiting for contest start, {} left", plural(wait_retries, "retry", "retries")));
						wait_retries -= 1;
						delay(Instant::now() + NOT_YET_STARTED_RETRY_DELAY).await;
					},
					tasks => break tasks,
				}
			}
		}
	})
	.await
}

fn spawn_login_suggestion(site: &str, sess: &Arc<Session>) {
	let site = site.to_owned();
	let sess = sess.clone();
	evscode::spawn(async move {
		if !auth::has_any_saved(&site) {
			let message = format!("You are not logged in to {}, maybe do it now to save time when submitting?", site);
			let dec = evscode::Message::new(&message).item("log-in".to_owned(), "Log in", false).show().await;
			if let Some("log-in") = dec.as_ref().map(String::as_str) {
				sess.force_login().await?;
				evscode::Message::new("Logged in successfully").show().await;
			}
		}
		Ok(())
	});
}

/// Contains information about the contest necessary to start waiting for it to start.
/// When created, this manifest is saved to a file in a different directory.
/// When ICIE notices this file exists, it reads it, deletes it and switches into contest mode.
#[derive(Deserialize, Serialize)]
struct Manifest {
	contest_url: String,
}
