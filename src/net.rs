use crate::{auth, util::retries::Retries};
use evscode::{
	error::{ResultExt, Severity}, E, R
};
use std::{error::Error as StdError, fmt, future::Future, pin::Pin, time::Duration};
use unijudge::{
	boxed::{BoxedSession, BoxedURL, DynamicBackend}, http::Client, Backend, ErrorCode, Resource, URL
};

const USER_AGENT: &str = concat!("ICIE/", env!("CARGO_PKG_VERSION"), " (+https://github.com/pustaczek/icie)");
const RETRY_LIMIT: usize = 4;
const RETRY_DELAY: Duration = Duration::from_secs(10);

pub static BACKENDS: [BackendMeta; 7] = [
	BackendMeta { backend: &unijudge_atcoder::AtCoder, cpp: &["C++20 (GCC 12.2)"] },
	BackendMeta { backend: &unijudge_codechef::CodeChef, cpp: &["C++17(gcc 11.1.0)"] },
	BackendMeta { backend: &unijudge_hackerearth::HackerEarth, cpp: &["C++17"] },
	BackendMeta { backend: &unijudge_newtonschool::NewtonSchool, cpp: &["C++ (GCC 9.2.0)"] },
	BackendMeta { backend: &unijudge_codeforces::Codeforces, cpp: &["GNU G++20 11.2.0 (64 bit, winlibs)"] },
	BackendMeta { backend: &unijudge_sio2::Sio2, cpp: &["C++", "C++ (g++:8.3 std=c++17)"] },
	BackendMeta { backend: &unijudge_spoj::SPOJ, cpp: &["C++14 (clang 8.0)"] },
];

pub struct Session {
	pub backend: &'static BackendMeta,
	pub session: BoxedSession,
	site: String,
}

#[derive(Debug)]
pub struct BackendMeta {
	pub backend: &'static dyn DynamicBackend,
	pub cpp: &'static [&'static str],
}

pub fn interpret_url(url: &str) -> R<(BoxedURL, &'static BackendMeta)> {
	let backend = BACKENDS
		.iter()
		.filter_map(|backend| match backend.backend.deconstruct_url(url) {
			Ok(Some(url)) => Some(Ok((url, backend))),
			Ok(None) => None,
			Err(e) => Some(Err(e)),
		})
		.next();
	backend
		.wrap(format!("not yet supporting contests/tasks on site {}", url))
		.map_err(|e| e.action("Help with URLs", help_urls()))?
		.map_err(|e| from_unijudge_error(e).context(format!("not a valid task/contest URL {}", url)))
}

impl Session {
	pub async fn connect(domain: &str, backend: &'static BackendMeta) -> R<Session> {
		let client = Client::new(USER_AGENT).map_err(from_unijudge_error)?;
		let session = backend.backend.connect(client, domain);
		let site = format!("https://{}", domain);
		if let Some(auth) = auth::get_if_cached(&site).await {
			if let Ok(auth) = backend.backend.auth_deserialize(&auth) {
				match backend.backend.auth_restore(&session, &auth).await {
					Ok(()) => Ok(()),
					Err(e) => match e.code {
						ErrorCode::MalformedData | ErrorCode::WrongCredentials | ErrorCode::AccessDenied => Ok(()),
						_ => Err(from_unijudge_error(e)),
					},
				}?;
			}
		}
		Ok(Session { backend, session, site })
	}

	pub async fn run<'f, Y, F: Future<Output=unijudge::Result<Y>>+'f>(
		&'f self,
		mut f: impl FnMut(&'static dyn DynamicBackend, &'f BoxedSession) -> F+'f,
	) -> R<Y> {
		let mut retries = Retries::new(RETRY_LIMIT, RETRY_DELAY);
		loop {
			match f(self.backend.backend, &self.session).await {
				Ok(y) => break Ok(y),
				Err(e) => match e.code {
					ErrorCode::WrongCredentials | ErrorCode::AccessDenied => {
						self.maybe_error_show(e);
						let (username, password) = auth::get_cached_or_ask(&self.site).await?;
						self.login(&username, &password).await?
					},
					ErrorCode::NetworkFailure | ErrorCode::RateLimit if retries.wait().await => (),
					_ => break Err(from_unijudge_error(e)),
				},
			}
		}
	}

	pub async fn login(&self, username: &str, password: &str) -> R<()> {
		let _status = crate::STATUS.push("Logging in");
		let mut retries = Retries::new(RETRY_LIMIT, RETRY_DELAY);
		match self.backend.backend.auth_login(&self.session, username, password).await {
			Ok(()) => {
				if let Some(cache) =
					self.backend.backend.auth_cache(&self.session).await.map_err(from_unijudge_error)?
				{
					auth::save_cache(
						&self.site,
						&self.backend.backend.auth_serialize(&cache).map_err(from_unijudge_error)?,
					)
					.await;
				}
			},
			Err(e) => match e.code {
				ErrorCode::MalformedData | ErrorCode::WrongCredentials | ErrorCode::AccessDenied => {
					self.maybe_error_show(e);
					self.force_login_boxed().await?;
				},
				ErrorCode::NetworkFailure | ErrorCode::RateLimit if retries.wait().await => (),
				_ => return Err(from_unijudge_error(e)),
			},
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
		if e.code == ErrorCode::WrongCredentials {
			evscode::spawn(async {
				evscode::Message::new::<()>("Wrong username or password").error().show().await;
				Ok(())
			});
		}
	}
}

pub fn require_task<C: fmt::Debug, T: fmt::Debug>(url: URL<C, T>) -> R<URL<!, T>> {
	match url.resource {
		Resource::Task(t) => Ok(URL { domain: url.domain, site: url.site, resource: Resource::Task(t) }),
		_ => Err(E::error(format!("expected task url, found {:?}", url.resource))),
	}
}

pub fn require_contest<C: fmt::Debug, T: fmt::Debug>(url: URL<C, T>) -> R<URL<C, !>> {
	match url.resource {
		Resource::Contest(c) => Ok(URL { domain: url.domain, site: url.site, resource: Resource::Contest(c) }),
		_ => Err(E::error(format!("expected contest url, found {:?}", url.resource))),
	}
}

fn from_unijudge_error(uj_e: unijudge::Error) -> evscode::E {
	let severity = match uj_e.code {
		ErrorCode::AccessDenied
		| ErrorCode::MalformedURL
		| ErrorCode::NetworkFailure
		| ErrorCode::RateLimit
		| ErrorCode::WrongTaskUrl => Severity::Error,
		ErrorCode::AlienInvasion | ErrorCode::MalformedData | ErrorCode::NoTLS | ErrorCode::StateCorruption => {
			Severity::Bug
		},
		ErrorCode::NotRegistered | ErrorCode::NotYetStarted | ErrorCode::WrongCredentials | ErrorCode::Ended_Already=> Severity::Workflow,
	};
	let mut e = E::from_std_ref(&uj_e);
	e.0.severity = severity;
	if let Some(cause) = uj_e.source() {
		if let Some(cause) = cause.downcast_ref::<debris::Error>() {
			e.0.extended = cause.snapshots.clone();
		}
	}
	if uj_e.code == ErrorCode::MalformedURL || uj_e.code == ErrorCode::WrongTaskUrl {
		e = e.action("Help with URLs", help_urls());
	}
	e.0.backtrace = uj_e.backtrace;
	e
}

async fn help_urls() -> R<()> {
	evscode::open_external("https://github.com/pustaczek/icie/blob/master/docs/URLS.md").await?;
	Ok(())
}
