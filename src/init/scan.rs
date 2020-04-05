use crate::{
	net::{BackendMeta, Session, BACKENDS}, util::join_all_with_progress
};
use evscode::{error::Severity, R};
use std::sync::Arc;
use unijudge::{boxed::BoxedContestDetails, Backend};

pub struct ContestMeta {
	pub sess: Arc<Session>,
	pub details: BoxedContestDetails,
	pub backend: &'static BackendMeta,
}

type ContestList = (Arc<Session>, Vec<BoxedContestDetails>, &'static BackendMeta);

pub async fn fetch_contests() -> Vec<ContestMeta> {
	let domains = collect_contest_domains();
	let contest_lists = fetch_contest_lists(&domains).await;
	collect_contests(contest_lists)
}

fn collect_contest_domains() -> Vec<(&'static str, &'static BackendMeta)> {
	BACKENDS
		.iter()
		.filter(|backend| backend.backend.supports_contests())
		.flat_map(|backend| {
			backend.backend.accepted_domains().iter().map(move |domain| (*domain, backend))
		})
		.collect()
}

async fn fetch_contest_lists(domains: &[(&str, &'static BackendMeta)]) -> Vec<R<ContestList>> {
	join_all_with_progress(
		"ICIE Scan",
		domains.iter().copied().map(|(domain, backend)| async move {
			let sess = connect_to(domain, backend).await?;
			let contests = fetch_domain_contests(domain, &sess).await?;
			Ok((sess, contests, backend))
		}),
	)
	.await
}

async fn connect_to(domain: &str, backend: &'static BackendMeta) -> R<Arc<Session>> {
	let _status = crate::STATUS.push(format!("Connecting {}", domain));
	let session = Session::connect(domain, backend).await?;
	Ok(Arc::new(session))
}

async fn fetch_domain_contests(domain: &str, sess: &Session) -> R<Vec<BoxedContestDetails>> {
	let _status = crate::STATUS.push(format!("Fetching {}", domain));
	let contests = sess.run(|backend, sess| backend.contests(sess)).await?;
	Ok(contests)
}

fn collect_contests(contest_lists: Vec<R<ContestList>>) -> Vec<ContestMeta> {
	contest_lists
		.into_iter()
		.flat_map(|resp| match resp {
			Ok((sess, contests, backend)) => contests
				.into_iter()
				.map(move |details| ContestMeta { sess: sess.clone(), details, backend })
				.collect(),
			Err(e) => {
				e.severity(Severity::Warning).emit();
				Vec::new()
			},
		})
		.collect()
}
