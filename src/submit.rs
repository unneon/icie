use crate::{
	dir, manifest::Manifest, net::{self, require_task}, telemetry::TELEMETRY, test, util::{fs, sleep, SourceTarget}
};
use evscode::{error::Severity, E, R};
use log::debug;
use std::time::Duration;
use unijudge::{
	boxed::{BoxedContest, BoxedTask}, Backend, RejectionCause, Resource
};

#[evscode::command(title = "ICIE Submit", key = "alt+f12")]
async fn send() -> R<()> {
	debug!("requesting submit");
	let _status = crate::STATUS.push("Submitting");
	TELEMETRY.submit_f12.spark();
	let (_, report) = crate::test::view::manage::COLLECTION.get_force(SourceTarget::Main).await?;
	if report.iter().any(|test| !test.success()) {
		debug!("submit aborted because of failing tests");
		TELEMETRY.submit_failtest.spark();
		return Err(E::error("some tests failed, submit aborted").severity(Severity::Workflow));
	}
	if report.is_empty() {
		debug!("submit aborted because of missing tests");
		TELEMETRY.submit_notest.spark();
		return Err(E::error("no tests available, add some to check if your solution is correct")
			.action("Add test (Alt+-)", test::input())
			.action("Submit anyway", send_passed())
			.severity(Severity::Workflow));
	}
	drop(_status);
	send_passed().await
}

async fn send_passed() -> R<()> {
	let _status = crate::STATUS.push("Submitting");
	TELEMETRY.submit_send.spark();
	let code = dir::solution()?;
	let code = fs::read_to_string(&code).await?;
	let manifest = Manifest::load().await?;
	let url = manifest.req_task_url()?;
	let (url, backend) = net::interpret_url(url)?;
	let url = require_task::<BoxedContest, BoxedTask>(url)?;
	let Resource::Task(task) = url.resource;
	let sess = net::Session::connect(&url.domain, backend).await?;
	let langs = {
		let _status = crate::STATUS.push("Querying languages");
		sess.run(|backend, sess| backend.task_languages(sess, &task)).await?
	};
	debug!("found {} supported languages", langs.len());
	let lang = langs.iter().find(|lang| lang.name == backend.cpp).ok_or_else(|| {
		E::error(format!("not found language {:?}", backend.cpp))
			.context("this task does not seem to allow C++ solutions")
			.extended(format!("{:#?}", langs))
	})?;
	let submit_id = sess.run(|backend, sess| backend.task_submit(sess, &task, lang, &code)).await?;
	drop(_status);
	track(sess, &task, submit_id).await?;
	Ok(())
}

const TRACK_DELAY: Duration = Duration::from_secs(5);
const TRACK_NOT_SEEN_RETRY_LIMIT: usize = 4;
const TRACK_NOT_SEEN_RETRY_DELAY: Duration = Duration::from_secs(5);

async fn track(sess: crate::net::Session, url: &unijudge::boxed::BoxedTask, id: String) -> R<()> {
	let _status = crate::STATUS.push("Tracking");
	let submission_url = sess.run(|backend, sess| futures::future::ok(backend.submission_url(sess, &url, &id))).await?;
	let progress = evscode::Progress::new().title(format!("Tracking submit [#{}]({})", id, submission_url)).show().0;
	let mut last_verdict = None;
	let mut not_seen_retry_limit = TRACK_NOT_SEEN_RETRY_LIMIT;
	let verdict = loop {
		let submissions = {
			let _status = crate::STATUS.push("Refreshing...");
			sess.run(|backend, sess| backend.task_submissions(sess, &url)).await?
		};
		let submission = match submissions.into_iter().find(|subm| subm.id == id) {
			Some(submission) => submission,
			None if not_seen_retry_limit > 0 => {
				let _status = crate::STATUS.push("Retrying...");
				not_seen_retry_limit -= 1;
				sleep(TRACK_NOT_SEEN_RETRY_DELAY).await;
				continue;
			},
			None => {
				return Err(E::error(format!("submission {} not found on status page", id)));
			},
		};
		let should_send = !matches!(&submission.verdict, unijudge::Verdict::Pending { .. });
		if should_send {
			break submission.verdict;
		} else if Some(&submission.verdict) != last_verdict.as_ref() {
			progress.message(fmt_verdict(&submission.verdict));
			last_verdict = Some(submission.verdict);
		}
		sleep(TRACK_DELAY).await;
	};
	progress.end();
	let message = fmt_verdict(&verdict);
	evscode::Message::new::<()>(&message).show().await;
	Ok(())
}

fn fmt_verdict(verdict: &unijudge::Verdict) -> String {
	let mut message = String::new();
	match verdict {
		unijudge::Verdict::Scored { score, max, cause, test } => {
			message += &format!("Scored {}", score);
			message += &max.map(|max| format!(" out of {}", max)).unwrap_or_default();
			message += fmt_cause_withtest(&cause, &test);
			message += &fmt_testid(&test);
		},
		unijudge::Verdict::Accepted => {
			message += "Accepted";
		},
		unijudge::Verdict::Rejected { cause, test } => {
			message += "Rejected";
			message += fmt_cause_withtest(&cause, &test);
			message += &fmt_testid(&test);
		},
		unijudge::Verdict::Pending { test } => {
			message += "Pending";
			message += &fmt_testid(&test);
		},
		unijudge::Verdict::Skipped => {
			message += "Skipped";
		},
		unijudge::Verdict::Glitch => {
			message += "Glitched";
		},
	};
	message
}

fn fmt_cause_withtest(cause: &Option<RejectionCause>, test: &Option<String>) -> &'static str {
	match (cause, test) {
		(Some(RejectionCause::WrongAnswer), _) => " due to a Wrong Answer",
		(Some(RejectionCause::RuntimeError), _) => " due to a Runtime Error",
		(Some(RejectionCause::TimeLimitExceeded), _) => " due to a Time Limit Exceeded",
		(Some(RejectionCause::MemoryLimitExceeded), _) => " due to a Memory Limit Exceeded",
		(Some(RejectionCause::RuleViolation), _) => " due to a Rule Violation",
		(Some(RejectionCause::SystemError), _) => " due to a System Error",
		(Some(RejectionCause::CompilationError), _) => " due to a Compilation Error",
		(Some(RejectionCause::IdlenessLimitExceeded), _) => " due to an Idleness Limit Exceeded",
		(None, Some(_)) => " failing",
		(None, None) => "",
	}
}

fn fmt_testid(test: &Option<String>) -> String {
	test.as_ref().map(|test| format!(" on {}", test)).unwrap_or_default()
}
