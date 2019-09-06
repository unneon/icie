use crate::{
	auth, init::{
		init_task, names::{design_contest_name, design_task_name}
	}, launch, net::{interpret_url, Session}, util::{fmt_time_left, fs_read_to_string, fs_write, plural, TransactionDir}
};
use evscode::{error::ResultExt, E, R};
use serde::{Deserialize, Serialize};
use std::{
	fs, path::{Path, PathBuf}, sync::Arc, thread::sleep, time::{Duration, SystemTime}
};
use unijudge::{
	boxed::{BoxedContest, BoxedContestURL, BoxedTask, BoxedURL}, Backend, Resource, TaskDetails, URL
};

/// Set up an external directory with a contest manifest, and switch VS Code context to it.
/// This will likely kill the extension process, so do not do anything important after calling this function.
/// Post-setup steps will be called from plugin activation function.  
pub fn setup_sprint(sess: &Session, contest: &BoxedContest, contest_title: Option<&str>) -> R<()> {
	let root_dir = design_contest_name(
		&sess.backend.contest_id(contest),
		&match contest_title {
			Some(title) => title.to_owned(),
			None => sess.run(|backend, sess| backend.contest_title(sess, contest))?,
		},
		sess.backend.name_short(),
	)?;
	let root_dir = TransactionDir::new(&root_dir)?;
	let task0_dir = root_dir.path().join("icie-task0");
	let task0_dir = TransactionDir::new(&task0_dir)?;
	let manifest = Manifest { contest_url: sess.backend.contest_url(contest) };
	fs_write(task0_dir.path().join(".icie-contest"), serde_json::to_string(&manifest).wrap("serialization of contest manifest failed")?)?;
	let task0_dir = task0_dir.commit();
	root_dir.commit();
	evscode::open_folder(task0_dir, false);
	Ok(())
}

/// Check if a contest manifest exists, and if it does, start the contest setup.
pub fn check_for_manifest() -> R<()> {
	if let Ok(workspace) = evscode::workspace_root() {
		let manifest = workspace.join(".icie-contest");
		if manifest.exists() {
			sprint(&manifest)?;
		}
	}
	Ok(())
}

/// Do the actual contest setup and preparation.
fn sprint(manifest: &Path) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let manifest = pop_manifest(manifest)?;
	let (url, backend) = interpret_url(&manifest.contest_url)?;
	let url = extract_contest_url(url)?;
	let sess = Arc::new(Session::connect(&url.domain, backend.backend)?);
	wait_for_contest(&manifest, &url.site, &sess)?;
	let Resource::Contest(contest) = url.resource;
	let tasks = fetch_tasks(&sess, &contest)?;
	for (i, task) in tasks.iter().enumerate() {
		let details = fetch_task(task, &format!("{}/{}", i + 1, tasks.len()), &sess)?;
		let root = sprint_task_path(i == 0, &details)?;
		init_task(&root, Some(sess.run(|_, _| sess.backend.task_url(&sess.session, &task))?), Some(details))?;
		if i == 0 {
			launch::layout_setup()?;
		}
	}
	Ok(())
}

fn fetch_task(task: &BoxedTask, name: &str, sess: &Session) -> R<TaskDetails> {
	let _status = crate::STATUS.push(format!("Fetching task {}", name));
	sess.run(|backend, sess| backend.task_details(sess, task))
}

fn sprint_task_path(is_zero: bool, details: &TaskDetails) -> R<PathBuf> {
	let task0_root = evscode::workspace_root()?;
	if is_zero {
		Ok(task0_root)
	} else {
		let contest_root = task0_root.parent().unwrap();
		design_task_name(&contest_root, Some(&details))
	}
}

fn wait_for_contest(manifest: &Manifest, site: &str, sess: &Arc<Session>) -> R<()> {
	let details = match sess
		.run(|backend, sess| Ok(backend.contests(sess)?.into_iter().find(|details| backend.contest_url(&details.id) == manifest.contest_url)))?
	{
		Some(details) => details,
		None => return Ok(()),
	};
	let deadline = SystemTime::from(details.start);
	let total = match deadline.duration_since(SystemTime::now()) {
		Ok(total) => total,
		Err(_) => return Ok(()),
	};
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
	Ok(())
}

/// Parse the manifest and removes it.
fn pop_manifest(path: &Path) -> R<Manifest> {
	let manifest = serde_json::from_str(&fs_read_to_string(path)?).wrap("malformed contest manifest")?;
	fs::remove_file(path).wrap("could not delete contest manifest after use")?;
	Ok(manifest)
}

/// Get metadata from contest url.
fn extract_contest_url(url: BoxedURL) -> R<BoxedContestURL> {
	Ok(URL {
		domain: url.domain,
		site: url.site,
		resource: match url.resource {
			Resource::Contest(c) => Resource::Contest(c),
			Resource::Task(_) => return Err(E::error("expected a contest url in a contest manifest")),
		},
	})
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
