use crate::{
	auth, util::{self, from_unijudge_error}
};
use evscode::{E, R};
use std::{thread::sleep, time::Duration};
use unijudge::{boxed::BoxedURL, URL};

pub const USER_AGENT: &str = concat!("ICIE/", env!("CARGO_PKG_VERSION"), " (+https://github.com/pustaczek/icie)");
const NETWORK_ERROR_RETRY_LIMIT: usize = 2;
const NETWORK_ERROR_RETRY_DELAY: Duration = Duration::from_secs(2);

pub fn interpret_url(url: &str) -> R<(BoxedURL, &'static Backend)> {
	Ok(BACKENDS
		.iter()
		.filter_map(|backend| match backend.network.deconstruct_url(url) {
			Ok(Some(url)) => Some(Ok((url, backend))),
			Ok(None) => None,
			Err(e) => Some(Err(e)),
		})
		.next()
		.ok_or_else(|| E::error(format!("not yet supporting contests/tasks on site {}", url)))?
		.map_err(from_unijudge_error)?)
}

pub struct Session {
	pub site: String,
	raw: unijudge::boxed::Session,
}
impl Session {
	pub fn connect<C, T>(url: &URL<C, T>, backend: &'static Backend) -> R<Session> {
		let raw = backend.network.connect(&url.domain, USER_AGENT).map_err(from_unijudge_error)?;
		if let Some(cached_session) = auth::get_if_cached(&url.site) {
			match raw.restore_auth(&cached_session) {
				Err(unijudge::Error::WrongData) | Err(unijudge::Error::WrongCredentials) | Err(unijudge::Error::AccessDenied) => Ok(()),
				Err(e) => Err(util::from_unijudge_error(e)),
				Ok(()) => Ok(()),
			}?;
		}
		Ok(Session { site: url.site.clone(), raw })
	}

	pub fn run<T>(&self, mut f: impl FnMut(&unijudge::boxed::Session) -> unijudge::Result<T>) -> R<T> {
		let mut retries_left = NETWORK_ERROR_RETRY_LIMIT;
		loop {
			match f(&self.raw) {
				Ok(y) => break Ok(y),
				Err(e @ unijudge::Error::WrongCredentials) | Err(e @ unijudge::Error::AccessDenied) => {
					self.maybe_error_show(e);
					let (username, password) = auth::get_cached_or_ask(&self.site)?;
					self.login(&username, &password)?
				},
				Err(unijudge::Error::NetworkFailure(e)) if retries_left > 0 => self.wait_for_retry(&mut retries_left, e),
				Err(e) => break Err(util::from_unijudge_error(e)),
			}
		}
	}

	pub fn login(&self, username: &str, password: &str) -> R<()> {
		let mut retries_left = NETWORK_ERROR_RETRY_LIMIT;
		match self.raw.login(&username, &password) {
			Ok(()) => {
				if let Some(cache) = self.raw.cache_auth().map_err(util::from_unijudge_error)? {
					auth::save_cache(&self.site, &cache);
				}
			},
			Err(e @ unijudge::Error::WrongData) | Err(e @ unijudge::Error::WrongCredentials) | Err(e @ unijudge::Error::AccessDenied) => {
				self.maybe_error_show(e);
				self.force_login()?;
			},
			Err(unijudge::Error::NetworkFailure(e)) if retries_left > 0 => self.wait_for_retry(&mut retries_left, e),
			Err(e) => return Err(util::from_unijudge_error(e)),
		}
		Ok(())
	}

	pub fn force_login(&self) -> R<()> {
		let (username, password) = auth::get_force_ask(&self.site)?;
		self.login(&username, &password)
	}

	fn maybe_error_show(&self, e: unijudge::Error) {
		if let unijudge::Error::WrongCredentials = e {
			evscode::Message::new("Wrong username or password").error().build().spawn();
		}
	}

	fn wait_for_retry(&self, retries_left: &mut usize, e: reqwest::Error) {
		assert!(*retries_left > 0);
		let _status = crate::STATUS.push("Waiting to retry");
		if *retries_left == NETWORK_ERROR_RETRY_LIMIT {
			util::from_unijudge_error(unijudge::Error::NetworkFailure(e))
				.context(format!("retrying in {} seconds", NETWORK_ERROR_RETRY_DELAY.as_secs_f64()))
				.warning()
				.emit();
		}
		*retries_left -= 1;
		sleep(NETWORK_ERROR_RETRY_DELAY);
	}
}

pub struct Backend {
	pub network: &'static dyn unijudge::boxed::Backend,
	pub cpp: &'static str,
}

pub const BACKENDS: &[Backend] = &[
	Backend { network: &unijudge_codeforces::Codeforces, cpp: "GNU G++17 7.3.0" },
	Backend { network: &unijudge_atcoder::Atcoder, cpp: "C++14 (GCC 5.4.1)" },
	Backend { network: &unijudge_spoj::SPOJ, cpp: "C++14 (clang 8.0)" },
	Backend { network: &unijudge_sio2::Sio2, cpp: "C++" },
];
