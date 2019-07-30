use crate::{auth, util};
use evscode::{E, R};

pub fn connect(url: &str) -> R<(Session, unijudge::boxed::Task, &'static Backend)> {
	let raw_url = url;
	let task = find_backend(url)?.ok_or_else(|| E::error("this site is not supported yet"))?;
	let user_agent = format!("ICIE/{} (+https://github.com/pustaczek/icie)", env!("CARGO_PKG_VERSION"));
	let raw = task.backend.network.connect(raw_url, &user_agent).map_err(util::from_unijudge_error)?;
	if let Some(cached_session) = auth::get_if_cached(&task.site) {
		match raw.restore_auth(&cached_session) {
			Err(unijudge::Error::WrongData) | Err(unijudge::Error::WrongCredentials) | Err(unijudge::Error::AccessDenied) => Ok(()),
			Err(e) => Err(util::from_unijudge_error(e)),
			Ok(()) => Ok(()),
		}?;
	}
	Ok((Session { site: task.site, raw }, task.task, task.backend))
}

pub struct Session {
	site: String,
	raw: unijudge::boxed::Session,
}
impl Session {
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

pub struct ProcessedTask {
	pub site: String,
	pub task: unijudge::boxed::Task,
	pub backend: &'static Backend,
}

pub fn find_backend(url: &str) -> R<Option<ProcessedTask>> {
	for backend in BACKENDS {
		if let Some((domain, task)) = backend.network.deconstruct_task(url).map_err(util::from_unijudge_error)? {
			let site = format!("https://{}", domain);
			return Ok(Some(ProcessedTask { site, task, backend }));
		}
	}
	return Ok(None);
}

pub struct Backend {
	pub network: &'static dyn unijudge::boxed::Backend,
	pub cpp: &'static str,
}

const BACKENDS: &[Backend] = &[
	Backend { network: &unijudge_atcoder::Atcoder, cpp: "C++14 (GCC 5.4.1)" },
	Backend { network: &unijudge_codeforces::Codeforces, cpp: "GNU G++17 7.3.0" },
	Backend { network: &unijudge_sio2::Sio2, cpp: "C++" },
	Backend { network: &unijudge_spoj::SPOJ, cpp: "C++14 (clang 8.0)" },
];
