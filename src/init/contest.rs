use crate::{
	auth, init::{
		init_task, names::{design_contest_name, design_task_name}
	}, net::{interpret_url, require_contest, Session}, telemetry::TELEMETRY, util::{fmt_time_left, fs_read_to_string, fs_write, plural, TransactionDir}
};
use evscode::{error::ResultExt, E, R};
use serde::{Deserialize, Serialize};
use std::{
	fs, path::Path, sync::Arc, thread::sleep, time::{Duration, SystemTime}
};
use unijudge::{
	boxed::{BoxedContest, BoxedTask}, Backend, Resource, TaskDetails
};

/// Wait for the contest, set up the first task, save a contest manifest and switch to the directory.  
/// This will likely kill the extension process, so do not do anything important after calling this function.
/// Post-setup steps will either be done from this function or the extension activation function.
pub fn sprint(sess: Arc<Session>, contest: &BoxedContest, contest_title: Option<&str>) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let root_dir = design_contest_name(
		&sess.backend.contest_id(contest),
		&match contest_title {
			Some(title) => title.to_owned(),
			None => sess.run(|backend, sess| backend.contest_title(sess, contest))?,
		},
		sess.backend.name_short(),
	)?;
	let root_dir = TransactionDir::new(&root_dir)?;
	let url_raw = sess.backend.contest_url(contest);
	let (url, _) = interpret_url(&url_raw)?;
	let url = require_contest(url)?;
	wait_for_contest(&url_raw, &url.site, &sess)?;
	let Resource::Contest(contest) = url.resource;
	let tasks = fetch_tasks(&sess, &contest)?;
	let task0 = tasks.get(0).wrap("could not find any tasks in contest")?;
	let task0_details = fetch_task(task0, &format!("1/{}", tasks.len()), &sess)?;
	let task0_url = sess.run(|backend, sess| backend.task_url(sess, task0))?;
	let task0_path = design_task_name(root_dir.path(), Some(&task0_details))?;
	init_task(&task0_path, Some(task0_url), Some(task0_details))?;
	let manifest = Manifest { contest_url: url_raw };
	fs_write(task0_path.join(".icie-contest"), serde_json::to_string(&manifest).wrap("serialization of contest manifest failed")?)?;
	root_dir.commit();
	evscode::open_folder(task0_path, false);
	Ok(())
}

/// Check if a contest manifest exists, and if it does, start the rest of the contest setup.
pub fn check_for_manifest() -> R<()> {
	if let Ok(workspace) = evscode::workspace_root() {
		let manifest = workspace.join(".icie-contest");
		if manifest.exists() {
			inner_sprint(&manifest)?;
		}
	}
	Ok(())
}

/// Do the setup for the rest of the contest tasks.
fn inner_sprint(manifest: &Path) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let manifest = pop_manifest(manifest)?;
	let (url, backend) = interpret_url(&manifest.contest_url)?;
	let url = require_contest(url)?;
	let sess = Session::connect(&url.domain, backend.backend)?;
	let Resource::Contest(contest) = url.resource;
	let tasks = fetch_tasks(&sess, &contest)?;
	let task_dir = evscode::workspace_root()?;
	let contest_dir = task_dir.parent().wrap("task directory has no parent contest directory")?;
	for (i, task) in tasks.iter().enumerate() {
		if i > 0 {
			let details = fetch_task(task, &format!("{}/{}", i + 1, tasks.len()), &sess)?;
			let root = design_task_name(contest_dir, Some(&details))?;
			init_task(&root, Some(sess.run(|_, _| sess.backend.task_url(&sess.session, &task))?), Some(details))?;
		}
	}
	Ok(())
}

fn fetch_task(task: &BoxedTask, name: &str, sess: &Session) -> R<TaskDetails> {
	let _status = crate::STATUS.push(format!("Fetching task {}", name));
	sess.run(|backend, sess| backend.task_details(sess, task))
}

fn wait_for_contest(url: &str, site: &str, sess: &Arc<Session>) -> R<()> {
	let details = match sess.run(|backend, sess| Ok(backend.contests(sess)?.into_iter().find(|details| backend.contest_url(&details.id) == url)))? {
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
	let canceler = progress.canceler().spawn();
	spawn_login_suggestion(site, sess);
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
		std::thread::sleep(Duration::from_secs(1));
	}
	progress.end();
	TELEMETRY.init_countdown_ok.spark();
	Ok(())
}

/// Parse the manifest and removes it.
fn pop_manifest(path: &Path) -> R<Manifest> {
	let manifest = serde_json::from_str(&fs_read_to_string(path)?).wrap("malformed contest manifest")?;
	fs::remove_file(path).wrap("could not delete contest manifest after use")?;
	Ok(manifest)
}

const NOT_YET_STARTED_RETRY_LIMIT: usize = 15;
const NOT_YET_STARTED_RETRY_DELAY: Duration = Duration::from_secs(1);

fn fetch_tasks(sess: &Session, contest: &BoxedContest) -> R<Vec<BoxedTask>> {
	let _status = crate::STATUS.push("Fetching contest");
	let mut wait_retries = NOT_YET_STARTED_RETRY_LIMIT;
	sess.run(|backend, sess| {
		loop {
			match backend.contest_tasks(sess, &contest) {
				Err(unijudge::Error::NotYetStarted) if wait_retries > 0 => {
					let _status =
						crate::STATUS.push(format!("Fetching contest (waiting for time sync, {} left)", plural(wait_retries, "retry", "retries")));
					wait_retries -= 1;
					sleep(NOT_YET_STARTED_RETRY_DELAY);
				},
				tasks => break tasks,
			}
		}
	})
}

fn spawn_login_suggestion(site: &str, sess: &Arc<Session>) {
	evscode::runtime::spawn({
		let site = site.to_owned();
		let sess = sess.clone();
		move || {
			if !auth::has_any_saved(&site) {
				let dec = evscode::Message::new(format!("You are not logged in to {}, maybe do it now to save time when submitting?", site))
					.item("log-in", "Log in", false)
					.build()
					.wait();
				if let Some("log-in") = dec.as_ref().map(String::as_str) {
					let _status = crate::STATUS.push("Logging in");
					sess.force_login()?;
					evscode::Message::new("Logged in successfully").build().spawn();
				}
			}
			Ok(())
		}
	});
}

/// Contains information about the contest necessary to start waiting for it to start.
/// When created, this manifest is saved to a file in a different directory.
/// When ICIE notices this file exists, it reads it, deletes it and switches into contest mode.
#[derive(Deserialize, Serialize)]
struct Manifest {
	contest_url: String,
}
