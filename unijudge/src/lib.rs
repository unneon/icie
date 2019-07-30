pub extern crate debris;
pub extern crate log;
pub extern crate reqwest;
pub extern crate serde;

use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

#[derive(Debug)]
pub enum Error {
	WrongCredentials,
	WrongData,
	WrongTaskUrl,
	AccessDenied,
	NetworkFailure(reqwest::Error),
	TLSFailure(reqwest::Error),
	URLParseFailure(reqwest::UrlError),
	UnexpectedHTML(debris::Error),
}
impl From<reqwest::Error> for Error {
	fn from(e: reqwest::Error) -> Self {
		Error::NetworkFailure(e)
	}
}
impl From<reqwest::UrlError> for Error {
	fn from(e: reqwest::UrlError) -> Self {
		Error::URLParseFailure(e)
	}
}
impl From<debris::Error> for Error {
	fn from(e: debris::Error) -> Self {
		Error::UnexpectedHTML(e)
	}
}
impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::WrongCredentials => f.write_str("wrong username or password"),
			Error::WrongData => f.write_str("wrong data passed to site API"),
			Error::WrongTaskUrl => f.write_str("wrong task URL format"),
			Error::AccessDenied => f.write_str("access denied"),
			Error::NetworkFailure(_) => f.write_str("network failure"),
			Error::TLSFailure(_) => f.write_str("TLS encryption failure"),
			Error::URLParseFailure(_) => f.write_str("URL parse failure"),
			Error::UnexpectedHTML(_) => f.write_str("error when scrapping site API response"),
		}
	}
}
impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error+'static)> {
		match self {
			Error::WrongCredentials => None,
			Error::WrongData => None,
			Error::WrongTaskUrl => None,
			Error::AccessDenied => None,
			Error::NetworkFailure(e) => Some(e),
			Error::TLSFailure(e) => Some(e),
			Error::URLParseFailure(e) => Some(e),
			Error::UnexpectedHTML(e) => Some(e),
		}
	}
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct Example {
	pub input: String,
	pub output: String,
}

#[derive(Clone, Debug)]
pub struct TaskDetails {
	pub symbol: String,
	pub title: String,
	pub contest_id: String,
	pub site_short: String,
	pub examples: Option<Vec<Example>>,
}

#[derive(Clone, Debug)]
pub struct Language {
	pub id: String,
	pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RejectionCause {
	WrongAnswer,
	RuntimeError,
	TimeLimitExceeded,
	MemoryLimitExceeded,
	RuleViolation,
	SystemError,
	CompilationError,
}

#[derive(Clone, Debug)]
pub struct Submission {
	pub id: String,
	pub verdict: Verdict,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
	Scored { score: f64, max: Option<f64>, cause: Option<RejectionCause>, test: Option<String> },
	Accepted,
	Rejected { cause: Option<RejectionCause>, test: Option<String> },
	Pending { test: Option<String> },
	Skipped,
}

pub trait Backend {
	type Session: 'static;
	type Task: Debug+'static;
	type CachedAuth: Serialize+for<'a> Deserialize<'a>;
	fn accepted_domains(&self) -> &[&str];
	fn deconstruct_task(&self, domain: &str, segments: &[&str]) -> Result<Self::Task>;
	fn connect(&self, client: reqwest::Client, domain: &str) -> Self::Session;
	fn login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()>;
	fn restore_auth(&self, session: &Self::Session, auth: Self::CachedAuth) -> Result<()>;
	fn cache_auth(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>>;
	fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails>;
	fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>>;
	fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>>;
	fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String>;
}

pub mod boxed {
	use crate::{Error, Language, Result, Submission, TaskDetails};
	use reqwest::{header::USER_AGENT, Url};
	use std::{any::Any, ops::Deref};

	pub trait Backend {
		fn deconstruct_task(&self, url: &str) -> Result<Option<(String, Task)>>;
		fn connect(&'static self, url: &str, user_agent: &str) -> Result<Session>;
		fn login(&self, session: &dyn Any, username: &str, password: &str) -> Result<()>;
		fn restore_auth(&self, session: &dyn Any, auth: &str) -> Result<()>;
		fn cache_auth(&self, session: &dyn Any) -> Result<Option<String>>;
		fn task_details(&self, session: &dyn Any, task: &dyn Any) -> Result<TaskDetails>;
		fn task_languages(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Language>>;
		fn task_submissions(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Submission>>;
		fn task_submit(&self, session: &dyn Any, task: &dyn Any, language: &Language, code: &str) -> Result<String>;
	}
	pub struct Session {
		backend: &'static dyn Backend,
		raw: Box<dyn Any+'static>,
	}
	pub struct Task {
		raw: Box<dyn Any+'static>,
	}
	impl Session {
		pub fn login(&self, username: &str, password: &str) -> Result<()> {
			self.backend.login(self.raw.deref(), username, password)
		}

		pub fn restore_auth(&self, auth: &str) -> Result<()> {
			self.backend.restore_auth(self.raw.deref(), auth)
		}

		pub fn cache_auth(&self) -> Result<Option<String>> {
			self.backend.cache_auth(self.raw.deref())
		}

		pub fn task_details(&self, task: &Task) -> Result<TaskDetails> {
			self.backend.task_details(self.raw.deref(), task.raw.deref())
		}

		pub fn task_languages(&self, task: &Task) -> Result<Vec<Language>> {
			self.backend.task_languages(self.raw.deref(), task.raw.deref())
		}

		pub fn task_submissions(&self, task: &Task) -> Result<Vec<Submission>> {
			self.backend.task_submissions(self.raw.deref(), task.raw.deref())
		}

		pub fn task_submit(&self, task: &Task, language: &Language, code: &str) -> Result<String> {
			self.backend.task_submit(self.raw.deref(), task.raw.deref(), language, code)
		}
	}

	impl<T: crate::Backend> Backend for T {
		fn deconstruct_task(&self, url: &str) -> Result<Option<(String, Task)>> {
			let url: Url = url.parse()?;
			let domain = url.domain().ok_or(Error::WrongTaskUrl)?;
			if self.accepted_domains().contains(&domain) {
				let segments = url.path_segments().ok_or(Error::WrongTaskUrl)?.filter(|s| !s.is_empty()).collect::<Vec<_>>();
				let task = <T as crate::Backend>::deconstruct_task(self, domain, &segments)?;
				Ok(Some((domain.to_owned(), Task { raw: Box::new(task) })))
			} else {
				Ok(None)
			}
		}

		fn connect(&'static self, url: &str, user_agent: &str) -> Result<Session> {
			let client = reqwest::ClientBuilder::new()
				.cookie_store(true)
				.default_headers(vec![(USER_AGENT, reqwest::header::HeaderValue::from_str(user_agent).unwrap())].into_iter().collect())
				.build()
				.map_err(Error::TLSFailure)?;
			let url = url.parse::<Url>()?;
			let domain = url.domain().ok_or(Error::WrongTaskUrl)?;
			let session = <T as crate::Backend>::connect(self, client, domain);
			Ok(Session { backend: self, raw: Box::new(session) })
		}

		fn login(&self, session: &dyn Any, username: &str, password: &str) -> Result<()> {
			<T as crate::Backend>::login(self, session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?, username, password)
		}

		fn restore_auth(&self, session: &dyn Any, auth: &str) -> Result<()> {
			<T as crate::Backend>::restore_auth(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				serde_json::from_str(auth).map_err(|_| Error::WrongData)?,
			)
		}

		fn cache_auth(&self, session: &dyn Any) -> Result<Option<String>> {
			Ok(<T as crate::Backend>::cache_auth(self, session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?)?
				.map(|c| serde_json::to_string(&c).unwrap()))
		}

		fn task_details(&self, session: &dyn Any, task: &dyn Any) -> Result<TaskDetails> {
			<T as crate::Backend>::task_details(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
			)
		}

		fn task_languages(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Language>> {
			<T as crate::Backend>::task_languages(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
			)
		}

		fn task_submissions(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Submission>> {
			<T as crate::Backend>::task_submissions(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
			)
		}

		fn task_submit(&self, session: &dyn Any, task: &dyn Any, language: &Language, code: &str) -> Result<String> {
			<T as crate::Backend>::task_submit(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
				language,
				code,
			)
		}
	}
}
