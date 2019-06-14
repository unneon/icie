use crate::{auth, util};
use evscode::{E, R};

pub fn connect(url: &str) -> R<(Session, unijudge::TaskUrl)> {
	let (url, backend) = find_backend(url)?.ok_or_else(|| E::error("this site is not supported yet"))?;
	let raw = backend.connect(&url.site).map_err(util::from_unijudge_error)?;
	if let Some(cached_session) = auth::cached(&url.site) {
		raw.restore_auth(&cached_session).map_err(util::from_unijudge_error)?;
	}
	Ok((Session { site: url.site.clone(), raw }, url))
}

pub struct Session {
	site: String,
	raw: Box<dyn unijudge::Session>,
}
impl Session {
	pub fn run<T>(&self, f: impl Fn(&dyn unijudge::Session) -> unijudge::Result<T>) -> R<T> {
		loop {
			match f(&*self.raw) {
				Ok(y) => break Ok(y),
				Err(unijudge::Error::AccessDenied) => self.login()?,
				Err(e) => break Err(util::from_unijudge_error(e)),
			}
		}
	}

	fn force_new_login(&self) -> R<()> {
		let (username, password) = auth::ask(&self.site)?;
		match self.raw.login(&username, &password) {
			Ok(()) => {
				if let Some(cache) = self.raw.cache_auth().map_err(util::from_unijudge_error)? {
					auth::save_session(&self.site, &cache);
				}
				Ok(())
			},
			Err(unijudge::Error::WrongCredentials) => self.force_new_login(),
			Err(e) => Err(util::from_unijudge_error(e)),
		}
	}

	fn login(&self) -> R<()> {
		let (username, password) = auth::query(&self.site)?;
		match self.raw.login(&username, &password) {
			Ok(()) => {
				if let Some(cache) = self.raw.cache_auth().map_err(util::from_unijudge_error)? {
					auth::save_session(&self.site, &cache);
				}
				Ok(())
			},
			Err(unijudge::Error::WrongCredentials) => self.force_new_login(),
			Err(e) => Err(util::from_unijudge_error(e)),
		}
	}
}

pub fn find_backend(url: &str) -> R<Option<(unijudge::TaskUrl, &'static dyn unijudge::Backend)>> {
	for backend in BACKENDS {
		if let Some(url) = backend.deconstruct_url(url).map_err(util::from_unijudge_error)? {
			return Ok(Some((url, *backend)));
		}
	}
	return Ok(None);
}

const BACKENDS: &[&dyn unijudge::Backend] = &[&unijudge_codeforces::Codeforces, &unijudge_spoj::SPOJ];
