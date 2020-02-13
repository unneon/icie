use crate::{auth, util::sleep};
use evscode::{error::ResultExt, E, R};
use log::debug;
use std::{fmt, future::Future, pin::Pin, time::Duration};
use unijudge::{
	boxed::{BoxedSession, BoxedURL, DynamicBackend}, http::Client, Backend, Resource, URL
};

const USER_AGENT: &str =
	concat!("ICIE/", env!("CARGO_PKG_VERSION"), " (+https://github.com/pustaczek/icie)");
const NETWORK_ERROR_RETRY_LIMIT: usize = 4;
const NETWORK_ERROR_RETRY_DELAY: Duration = Duration::from_secs(5);

pub static BACKENDS: [BackendMeta; 5] = [
	BackendMeta::new(&unijudge_atcoder::AtCoder, "C++14 (GCC 5.4.1)", "atcoder"),
	BackendMeta::new(&unijudge_codechef::CodeChef, "C++14(gcc 6.3)", "codechef"),
	BackendMeta::new(&unijudge_codeforces::Codeforces, "GNU G++17 7.3.0", "codeforces"),
	BackendMeta::new(&unijudge_sio2::Sio2, "C++", "sio2"),
	BackendMeta::new(&unijudge_spoj::SPOJ, "C++14 (clang 8.0)", "spoj"),
];

pub struct Session {
	pub backend: &'static dyn DynamicBackend,
	pub session: BoxedSession,
	site: String,
}

#[derive(Debug)]
pub struct BackendMeta {
	pub backend: &'static dyn DynamicBackend,
	pub cpp: &'static str,
	pub telemetry_id: &'static str,
}

impl BackendMeta {
	const fn new(
		backend: &'static dyn DynamicBackend,
		cpp: &'static str,
		telemetry_id: &'static str,
	) -> BackendMeta
	{
		BackendMeta { backend, cpp, telemetry_id }
	}
}

pub fn interpret_url(url: &str) -> R<(BoxedURL, &'static BackendMeta)> {
	debug!("icie.net.interpret_url, url = {:?}", url);
	let backend = BACKENDS
		.iter()
		.filter_map(|backend| match backend.backend.deconstruct_url(url) {
			Ok(Some(url)) => Some(Ok((url, backend))),
			Ok(None) => None,
			Err(e) => Some(Err(e)),
		})
		.next();
	debug!("icie.net.interpret_url, backend = {:?}", url);
	Ok(backend
		.wrap(format!("not yet supporting contests/tasks on site {}", url))?
		.map_err(from_unijudge_error)?)
}

impl Session {
	pub async fn connect(domain: &str, backend: &'static BackendMeta) -> R<Session> {
		debug!("icie.net.Session.connect, domain = {:?}, backend = {:?}", domain, backend);
		evscode::telemetry("connect", &[("backend", backend.telemetry_id)], &[]);
		let backend = backend.backend;
		let client = Client::new(USER_AGENT).map_err(from_unijudge_error)?;
		let session = backend.connect(client, domain);
		let site = format!("https://{}", domain);
		debug!("icie.net.Session.connect, connected successfully");
		if let Some(auth) = auth::get_if_cached(&site).await {
			debug!("icie.net.Session.connect, found cached auth");
			if let Ok(auth) = backend.auth_deserialize(&auth) {
				debug!("icie.net.Session.connect, cached auth has valid format");
				match backend.auth_restore(&session, &auth).await {
					Err(unijudge::Error::WrongData)
					| Err(unijudge::Error::WrongCredentials)
					| Err(unijudge::Error::AccessDenied) => {
						debug!("icie.net.Session.connect, backend refused to use cached auth");
						Ok(())
					},
					Err(e) => Err(from_unijudge_error(e)),
					Ok(()) => {
						debug!("icie.net.Session.connect, cached auth used successfully");
						Ok(())
					},
				}?;
			} else {
				debug!("icie.net.Session.connect, cached auth has invalid format");
			}
		} else {
			debug!("icie.net.Session.connect, did not find cached auth");
		}
		Ok(Session { backend, session, site })
	}

	pub async fn run<'f, Y, F: Future<Output=unijudge::Result<Y>>+'f>(
		&'f self,
		mut f: impl FnMut(&'static dyn DynamicBackend, &'f BoxedSession) -> F+'f,
	) -> R<Y>
	{
		debug!("icie.net.Session.run starting a new operation");
		let mut retries_left = NETWORK_ERROR_RETRY_LIMIT;
		loop {
			match f(self.backend, &self.session).await {
				Ok(y) => break Ok(y),
				Err(e @ unijudge::Error::WrongCredentials)
				| Err(e @ unijudge::Error::AccessDenied) => {
					debug!("icie.net.Session.run access was denied, e = {:?}", e);
					self.maybe_error_show(e);
					let (username, password) = auth::get_cached_or_ask(&self.site).await?;
					debug!("icie.net.Session.run logging in");
					self.login(&username, &password).await?
				},
				Err(unijudge::Error::NetworkFailure(e)) if retries_left > 0 => {
					self.wait_for_retry(&mut retries_left, e).await
				},
				Err(e) => break Err(from_unijudge_error(e)),
			}
		}
	}

	pub async fn login(&self, username: &str, password: &str) -> R<()> {
		debug!("icie.net.Session.login logging in");
		let _status = crate::STATUS.push("Logging in");
		let mut retries_left = NETWORK_ERROR_RETRY_LIMIT;
		match self.backend.auth_login(&self.session, &username, &password).await {
			Ok(()) => {
				debug!("icie.net.Session.login logged in successfully");
				if let Some(cache) =
					self.backend.auth_cache(&self.session).await.map_err(from_unijudge_error)?
				{
					debug!("icie.net.Session.login caching auth data");
					auth::save_cache(
						&self.site,
						&self.backend.auth_serialize(&cache).map_err(from_unijudge_error)?,
					)
					.await;
				} else {
					debug!("icie.net.Session.login failed to prepare auth cache data");
				}
			},
			Err(e @ unijudge::Error::WrongData)
			| Err(e @ unijudge::Error::WrongCredentials)
			| Err(e @ unijudge::Error::AccessDenied) => {
				debug!("icie.net.Session.login login was not successful");
				self.maybe_error_show(e);
				self.force_login_boxed().await?;
			},
			Err(unijudge::Error::NetworkFailure(e)) if retries_left > 0 => {
				self.wait_for_retry(&mut retries_left, e).await
			},
			Err(e) => return Err(from_unijudge_error(e)),
		}
		Ok(())
	}

	pub async fn force_login(&self) -> R<()> {
		let (username, password) = auth::get_force_ask(&self.site).await?;
		self.login(&username, &password).await
	}

	fn force_login_boxed<'a>(&'a self) -> Pin<Box<dyn Future<Output=R<()>>+'a>> {
		Box::pin(self.force_login())
	}

	fn maybe_error_show(&self, e: unijudge::Error) {
		if let unijudge::Error::WrongCredentials = e {
			evscode::spawn(async {
				evscode::Message::new::<()>("Wrong username or password").error().show().await;
				Ok(())
			});
		}
	}

	async fn wait_for_retry(&self, retries_left: &mut usize, e: unijudge::reqwest::Error) {
		debug!(
			"icie.net.Session.wait_for_retry network failure, retries_left = {:?}",
			retries_left
		);
		assert!(*retries_left > 0);
		let _status = crate::STATUS.push("Waiting to retry");
		if *retries_left == NETWORK_ERROR_RETRY_LIMIT {
			from_unijudge_error(unijudge::Error::NetworkFailure(e))
				.context(format!("retrying in {} seconds", NETWORK_ERROR_RETRY_DELAY.as_secs_f64()))
				.warning()
				.emit();
		}
		*retries_left -= 1;
		sleep(NETWORK_ERROR_RETRY_DELAY).await;
	}
}

pub fn require_task<C: fmt::Debug, T: fmt::Debug>(url: URL<C, T>) -> R<URL<!, T>> {
	match url.resource {
		Resource::Task(t) => {
			Ok(URL { domain: url.domain, site: url.site, resource: Resource::Task(t) })
		},
		_ => Err(E::error(format!("expected task url, found {:?}", url.resource))),
	}
}
pub fn require_contest<C: fmt::Debug, T: fmt::Debug>(url: URL<C, T>) -> R<URL<C, !>> {
	match url.resource {
		Resource::Contest(c) => {
			Ok(URL { domain: url.domain, site: url.site, resource: Resource::Contest(c) })
		},
		_ => Err(E::error(format!("expected contest url, found {:?}", url.resource))),
	}
}

fn from_unijudge_error(e: unijudge::Error) -> evscode::E {
	match e {
		unijudge::Error::WrongCredentials => E::from_std(e).reform("wrong username or password"),
		unijudge::Error::WrongData => E::from_std(e).reform("wrong data passed to API"),
		unijudge::Error::WrongTaskUrl => E::from_std(e).reform("wrong task URL format"),
		unijudge::Error::AccessDenied => E::from_std(e).reform("access denied"),
		unijudge::Error::NotYetStarted => E::from_std(e).reform("contest not yet started"),
		unijudge::Error::RateLimit => E::from_std(e).reform("too frequent requests to site"),
		unijudge::Error::NetworkFailure(e) => E::from_std(e).context("network error"),
		unijudge::Error::NoTLS(e) => E::from_std(e).context("TLS initialization error"),
		unijudge::Error::URLParseFailure(e) => E::from_std(e).context("URL parse error"),
		unijudge::Error::StateCorruption => E::from_std(e).context("broken state"),
		unijudge::Error::UnexpectedHTML(e) => {
			let mut extended = Vec::new();
			if !e.snapshots.is_empty() {
				extended.push(e.snapshots.last().unwrap().clone());
			}
			evscode::E {
				severity: evscode::error::Severity::Error,
				reasons: vec![format!(
					"[PLEASE REPORT THIS](https://github.com/pustaczek/icie/issues), unexpected \
					 HTML structure ({:?} at {:?})",
					e.reason, e.operations
				)],
				details: Vec::new(),
				actions: Vec::new(),
				backtrace: evscode::error::Backtrace::new(),
				extended,
			}
		},
		unijudge::Error::UnexpectedJSON { endpoint, resp_raw, inner } => {
			let message = format!("unexpected JSON response at {}", endpoint);
			let mut e = match inner {
				Some(inner) => E::from_std_ref(inner.as_ref()).context(message),
				None => E::error(message),
			};
			e.extended.push(resp_raw);
			e
		},
		unijudge::Error::UnexpectedResponse { endpoint, message, resp_raw, inner } => {
			let mut e = match inner {
				Some(inner) => E::from_std_ref(inner.as_ref()).context(message),
				None => E::error(message),
			}
			.context(format!("unexpected site response at {}", endpoint));
			e.extended.push(resp_raw);
			e
		},
	}
}
