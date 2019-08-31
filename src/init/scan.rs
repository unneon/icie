use crate::net::{self, BackendMeta, BACKENDS};
use evscode::R;
use std::{
	sync::Arc, thread::{self, JoinHandle}
};
use unijudge::{boxed::BoxedContestDetails, Backend, URL};

pub fn fetch_contests() -> Vec<(Arc<net::Session>, BoxedContestDetails)> {
	let _status = crate::STATUS.push("Fetching contests");
	let domains: Vec<(&'static str, &'static BackendMeta)> = BACKENDS
		.iter()
		.filter(|backend| backend.backend.supports_contests())
		.flat_map(|backend| backend.backend.accepted_domains().iter().map(move |domain| (*domain, backend)))
		.collect();
	let _status = crate::STATUS.push_silence();
	let tasks: Vec<_> = domains
		.into_iter()
		.map(|(domain, backend)| {
			(
				domain,
				thread::spawn(move || {
					let url = URL::dummy_domain(domain);
					let sess = {
						let _status = crate::STATUS.push(format!("Connecting {}", domain));
						Arc::new(net::Session::connect(&url.domain, backend.backend)?)
					};
					let _status = crate::STATUS.push(format!("Fetching {}", domain));
					let contests = sess.run(|backend, sess| backend.contests(sess))?;
					Ok((sess, contests))
				}),
			)
		})
		.collect();
	tasks
		.into_iter()
		.flat_map(|(domain, handle): (_, JoinHandle<R<_>>)| -> Vec<(Arc<net::Session>, BoxedContestDetails)> {
			handle
				.join()
				.unwrap()
				.map(|(sess, contests)| contests.into_iter().map(|contest| (sess.clone(), contest)).collect::<Vec<_>>())
				.unwrap_or_else(|e| {
					e.context(format!("failed to fetch {} contests", domain)).warning().emit();
					Vec::new()
				})
		})
		.collect()
}
