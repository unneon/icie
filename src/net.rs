use crate::{
	auth, util::{self, from_unijudge_error}
};
use evscode::{E, R};
use unijudge::{boxed::BoxedURL, URL};

pub const USER_AGENT: &str = concat!("ICIE/", env!("CARGO_PKG_VERSION"), " (+https://github.com/pustaczek/icie)");

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
	site: String,
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

	pub fn run<T>(&self, f: impl Fn(&unijudge::boxed::Session) -> unijudge::Result<T>) -> R<T> {
		loop {
			match f(&self.raw) {
				Ok(y) => break Ok(y),
				Err(unijudge::Error::AccessDenied) => {
					let (username, password) = auth::get_cached_or_ask(&self.site)?;
					self.login(&username, &password)?
				},
				Err(e) => break Err(util::from_unijudge_error(e)),
			}
		}
	}

	fn login(&self, username: &str, password: &str) -> Result<(), E> {
		match self.raw.login(&username, &password) {
			Ok(()) => {
				if let Some(cache) = self.raw.cache_auth().map_err(util::from_unijudge_error)? {
					auth::save_cache(&self.site, &cache);
				}
				Ok(())
			},
			Err(unijudge::Error::WrongData) | Err(unijudge::Error::WrongCredentials) | Err(unijudge::Error::AccessDenied) => {
				let (username, password) = auth::get_force_ask(&self.site)?;
				self.login(&username, &password)
			},
			Err(e) => Err(util::from_unijudge_error(e)),
		}
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
