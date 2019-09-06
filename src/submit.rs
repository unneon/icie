use crate::{
	dir, net::{self, require_task}, util::{self, plural}
};
use evscode::{E, R};
use std::time::Duration;
use unijudge::{
	boxed::{BoxedContest, BoxedTask}, Backend, RejectionCause, Resource
};

#[evscode::command(title = "ICIE Submit", key = "alt+f12")]
fn send() -> R<()> {
	let _status = crate::STATUS.push("Submitting");
	let (_, report) = crate::test::view::manage::COLLECTION.get_force(None)?;
	if report.runs.iter().any(|test| !test.success()) {
		return Err(E::error("some tests failed, submit aborted").workflow_error());
	}
	if report.runs.is_empty() {
		return Err(E::error("no tests available; add some using Alt+- keyboard shortcut!").action("Submit anyway", send_passed).workflow_error());
	}
	send_passed()
}

fn send_passed() -> R<()> {
	let _status = crate::STATUS.push("Submitting");
	let code = util::fs_read_to_string(dir::solution()?)?;
	let manifest = crate::manifest::Manifest::load()?;
	let url = manifest.req_task_url().map_err(|e| e.context("submit aborted"))?;
	let (url, backend) = net::interpret_url(url)?;
	let url = require_task::<BoxedContest, BoxedTask>(url)?;
	let Resource::Task(task) = url.resource;
	let sess = net::Session::connect(&url.domain, backend.backend)?;
	let langs = {
		let _status = crate::STATUS.push("Querying languages");
		sess.run(|backend, sess| backend.task_languages(sess, &task))?
	};
	let lang = langs.iter().find(|lang| lang.name == backend.cpp).ok_or_else(|| {
		E::error(format!("not found language {:?}", backend.cpp))
			.reform("this task does not seem to allow C++ solutions")
			.extended(format!("{:#?}", langs))
	})?;
	let submit_id = sess.run(|backend, sess| backend.task_submit(sess, &task, lang, &code))?;
	track(sess, &task, submit_id)?;
	Ok(())
}

const TRACK_NOT_SEEN_RETRY_LIMIT: usize = 8;
const TRACK_NOT_SEEN_RETRY_DELAY: Duration = Duration::from_secs(1);

fn track(sess: crate::net::Session, url: &unijudge::boxed::BoxedTask, id: String) -> R<()> {
	let _status = crate::STATUS.push("Tracking");
	let sleep_duration = Duration::from_millis(2000);
	let progress = evscode::Progress::new().title(format!("Tracking submit #{}", id)).show();
	let mut last_verdict = None;
	let mut not_seen_retry_limit = TRACK_NOT_SEEN_RETRY_LIMIT;
	let verdict = loop {
		let submissions = {
			let _status = crate::STATUS.push("Tracking...");
			sess.run(|backend, sess| backend.task_submissions(sess, &url))?
		};
		let submission = match submissions.into_iter().find(|subm| subm.id == id) {
			Some(submission) => submission,
			None if not_seen_retry_limit > 0 => {
				log::debug!("submission {} not found on status page, {} left", id, plural(not_seen_retry_limit, "retry", "retries"));
				let _status = crate::STATUS.push("Tracking (retrying...)");
				not_seen_retry_limit -= 1;
				std::thread::sleep(TRACK_NOT_SEEN_RETRY_DELAY);
				continue;
			},
			None => return Err(E::error(format!("submission {} not found on status page", id))),
		};
		let should_send = match &submission.verdict {
			unijudge::Verdict::Pending { .. } => false,
			_ => true,
		};
		if should_send {
			break submission.verdict;
		} else if Some(&submission.verdict) != last_verdict.as_ref() {
			progress.message(fmt_verdict(&submission.verdict));
			last_verdict = Some(submission.verdict);
		}
		std::thread::sleep(sleep_duration);
	};
	progress.end();
	evscode::Message::new(fmt_verdict(&verdict)).build().spawn();
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
		(None, Some(_)) => " failing",
		(None, None) => "",
	}
}

fn fmt_testid(test: &Option<String>) -> String {
	test.as_ref().map(|test| format!(" on {}", test)).unwrap_or_default()
}
