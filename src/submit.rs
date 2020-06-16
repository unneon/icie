use crate::{
	dir, manifest::Manifest, net::{self, require_task, Session}, test, test::TestRun, util::{fs, retries::Retries, sleep, SourceTarget}
};
use evscode::{error::Severity, E, R};
use log::debug;
use std::time::Duration;
use unijudge::{
	boxed::{BoxedContest, BoxedTask}, Backend, Language, Resource, Submission
};

const TRACK_DELAY: Duration = Duration::from_secs(5);
const TRACK_NOT_SEEN_RETRY_LIMIT: usize = 4;
const TRACK_NOT_SEEN_RETRY_DELAY: Duration = Duration::from_secs(5);

#[evscode::command(title = "ICIE Submit", key = "alt+f12")]
async fn send() -> R<()> {
	debug!("requesting submit");
	let _status = crate::STATUS.push("Submitting");
	let report = crate::test::view::manage::COLLECTION.get_force(SourceTarget::Main).await?.1;
	check_tests_passed(&report)?;
	check_any_tests_ran(&report)?;
	drop(_status);
	send_after_tests_passed().await
}

fn check_tests_passed(report: &[TestRun]) -> R<()> {
	if report.iter().any(|test| !test.success()) {
		debug!("submit aborted because of failing tests");
		return Err(E::error("some tests failed, submit aborted").severity(Severity::Workflow));
	}
	Ok(())
}

fn check_any_tests_ran(report: &[TestRun]) -> R<()> {
	if report.is_empty() {
		debug!("submit aborted because of missing tests");
		return Err(E::error("no tests available, add some to check if your solution is correct")
			.action("Add test (Alt+-)", test::input())
			.action("Submit anyway", send_after_tests_passed())
			.severity(Severity::Workflow));
	}
	Ok(())
}

async fn send_after_tests_passed() -> R<()> {
	let _status = crate::STATUS.push("Submitting");
	let code = fs::read_to_string(&dir::solution()?).await?;
	let (sess, task) = connect_to_workspace_task().await?;
	let language = fetch_cpp_language(&task, &sess).await?;
	let submit_id = sess.run(|backend, sess| backend.task_submit(sess, &task, &language, &code)).await?;
	drop(_status);
	track(&sess, &task, &submit_id).await?;
	Ok(())
}

async fn connect_to_workspace_task() -> R<(Session, BoxedTask)> {
	let manifest = Manifest::load().await?;
	let url = manifest.req_task_url()?;
	let (url, backend) = net::interpret_url(url)?;
	let url = require_task::<BoxedContest, BoxedTask>(url)?;
	let Resource::Task(task) = url.resource;
	let sess = net::Session::connect(&url.domain, backend).await?;
	Ok((sess, task))
}

async fn fetch_cpp_language(task: &BoxedTask, sess: &Session) -> R<Language> {
	let languages = fetch_languages(task, sess).await?;
	debug!("found {} supported languages", languages.len());
	let language = languages.iter().find(|lang| sess.backend.cpp.contains(&&*lang.name)).ok_or_else(|| {
		E::error(format!("not found language {:?}", sess.backend.cpp))
			.context("this task does not seem to allow C++ solutions")
			.extended(format!("{:#?}", languages))
	})?;
	Ok(language.clone())
}

async fn fetch_languages(task: &BoxedTask, sess: &Session) -> R<Vec<Language>> {
	let _status = crate::STATUS.push("Querying languages");
	sess.run(|backend, sess| backend.task_languages(sess, &task)).await
}

async fn track(sess: &Session, task: &BoxedTask, id: &str) -> R<()> {
	let _status = crate::STATUS.push("Tracking");
	let submission_url = sess.run(|backend, sess| async move { Ok(backend.submission_url(sess, &task, &id)) }).await?;
	let progress = evscode::Progress::new().title(format!("Tracking submit [#{}]({})", id, submission_url)).show().0;
	let mut last_verdict = None;
	let mut not_seen_retries = Retries::new(TRACK_NOT_SEEN_RETRY_LIMIT, TRACK_NOT_SEEN_RETRY_DELAY);
	let verdict = loop {
		let submissions = fetch_submissions(task, &sess).await?;
		let submission = match submissions.into_iter().find(|subm| subm.id == id) {
			Some(submission) => submission,
			None if not_seen_retries.wait().await => continue,
			None => return Err(E::error(format!("submission {} not found on status page", id))),
		};
		let is_pending = matches!(&submission.verdict, unijudge::Verdict::Pending { .. });
		if !is_pending {
			break submission.verdict;
		} else if Some(&submission.verdict) != last_verdict.as_ref() {
			progress.message(submission.verdict.to_string());
			last_verdict = Some(submission.verdict);
		}
		sleep(TRACK_DELAY).await;
	};
	progress.end();
	evscode::Message::new::<()>(&verdict.to_string()).show().await;
	Ok(())
}

async fn fetch_submissions(task: &BoxedTask, sess: &Session) -> R<Vec<Submission>> {
	let _status = crate::STATUS.push("Refreshing...");
	sess.run(|backend, sess| backend.task_submissions(sess, &task)).await
}
