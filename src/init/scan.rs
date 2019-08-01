use crate::net::{self, Backend, BACKENDS};
use evscode::{E, R};
use std::{sync::Arc, thread};
use unijudge::{boxed::BoxedContestDetails, URL};

pub fn fetch_contests() -> R<Vec<(Arc<net::Session>, BoxedContestDetails)>> {
	let domains: Vec<(&'static str, &'static Backend)> = BACKENDS
		.iter()
		.filter(|backend| backend.network.supports_contests())
		.flat_map(|backend| backend.network.accepted_domains().iter().map(move |domain| (*domain, backend)))
		.collect();
	let _status = crate::STATUS.push_silence();
	let tasks: Vec<thread::JoinHandle<R<(net::Session, Vec<BoxedContestDetails>)>>> = domains
		.into_iter()
		.map(|(domain, backend)| {
			thread::spawn(move || {
				let url = URL::dummy_domain(domain);
				#[evscode::status("Connecting {}", domain)]
				let sess = net::Session::connect(&url, backend)?;
				#[evscode::status("Fetching {}", domain)]
				let contests = sess.run(|sess| sess.contests())?;
				Ok((sess, contests))
			})
		})
		.collect();
	tasks
		.into_iter()
		.flat_map(|handle| {
			match {
				try {
					let (sess, contests) = handle.join().map_err(|p| -> E { panic!(p) })??;
					let sess = Arc::new(sess);
					contests.into_iter().map(|contest| Ok((sess.clone(), contest))).collect::<Vec<_>>()
				}
			} {
				Ok(contests) => contests,
				Err(e) => vec![Err(e)],
			}
		})
		.collect()
}
