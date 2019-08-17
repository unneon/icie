#![feature(never_type)]

pub extern crate chrono;
pub extern crate debris;
pub extern crate html5ever;
pub extern crate log;
pub extern crate reqwest;
pub extern crate scraper;
pub extern crate selectors;
pub extern crate serde;

#[macro_use]
pub mod statement;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

#[derive(Debug)]
pub enum Error {
	WrongCredentials,
	WrongData,
	WrongTaskUrl,
	AccessDenied,
	NotYetStarted,
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
			Error::NotYetStarted => f.write_str("contest not yet started"),
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
			Error::NotYetStarted => None,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Statement {
	HTML { html: String },
}

#[derive(Clone, Debug)]
pub struct TaskDetails {
	pub id: String,
	pub title: String,
	pub contest_id: String,
	pub site_short: String,
	pub examples: Option<Vec<Example>>,
	pub statement: Option<Statement>,
	pub url: String,
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

#[derive(Clone, Debug)]
pub struct ContestDetails<I> {
	pub id: I,
	pub title: String,
	pub start: DateTime<FixedOffset>,
}

#[derive(Clone, Debug)]
pub enum Resource<C, T> {
	Contest(C),
	Task(T),
}

#[derive(Clone, Debug)]
pub struct URL<C, T> {
	pub domain: String,
	pub site: String,
	pub resource: Resource<C, T>,
}

impl URL<(), ()> {
	pub fn dummy_domain(domain: &str) -> URL<(), ()> {
		URL { domain: domain.to_owned(), site: format!("https://{}", domain), resource: Resource::Task(()) }
	}
}

pub trait Backend: Send+Sync {
	type Session: Send+Sync+'static;
	type Contest: Debug+Send+Sync+'static;
	type Task: Debug+Send+Sync+'static;
	type CachedAuth: Serialize+for<'a> Deserialize<'a>;
	fn accepted_domains(&self) -> &'static [&'static str];
	fn deconstruct_resource(&self, domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>>;
	fn connect(&self, client: reqwest::Client, domain: &str) -> Self::Session;
	fn login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()>;
	fn restore_auth(&self, session: &Self::Session, auth: Self::CachedAuth) -> Result<()>;
	fn cache_auth(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>>;
	fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails>;
	fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>>;
	fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>>;
	fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String>;
	fn task_url(&self, session: &Self::Session, task: &Self::Task) -> String;
	fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>>;
	fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>>;
	fn contest_id(&self, contest: &Self::Contest) -> String;
	fn contest_url(&self, contest: &Self::Contest) -> String;
	fn contest_site_prefix(&self) -> &'static str;
	fn site_short(&self) -> &'static str;
	const SUPPORTS_CONTESTS: bool;
}

pub mod boxed {
	use crate::{ContestDetails, Error, Language, Resource, Result, Submission, TaskDetails, URL};
	use reqwest::{header::USER_AGENT, Url};
	use std::{
		any::Any, fmt::{self, Debug}, ops::Deref
	};

	pub trait Backend: Send+Sync {
		fn accepted_domains(&self) -> &'static [&'static str];
		fn deconstruct_url(&self, url: &str) -> Result<Option<BoxedURL>>;
		fn connect(&'static self, domain: &str, user_agent: &str) -> Result<Session>;
		fn login(&self, session: &dyn Any, username: &str, password: &str) -> Result<()>;
		fn restore_auth(&self, session: &dyn Any, auth: &str) -> Result<()>;
		fn cache_auth(&self, session: &dyn Any) -> Result<Option<String>>;
		fn task_details(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<TaskDetails>;
		fn task_languages(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<Vec<Language>>;
		fn task_submissions(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<Vec<Submission>>;
		fn task_submit(&self, session: &dyn Any, task: &dyn AnyDebug, language: &Language, code: &str) -> Result<String>;
		fn task_url(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<String>;
		fn contests(&self, session: &dyn Any) -> Result<Vec<BoxedContestDetails>>;
		fn contest_tasks(&self, session: &dyn Any, contest: &dyn Any) -> Result<Vec<BoxedTask>>;
		fn contest_id(&self, contest: &dyn Any) -> Result<String>;
		fn contest_url(&self, contest: &dyn Any) -> Result<String>;
		fn contest_site_prefix(&self) -> &'static str;
		fn site_short(&self) -> &'static str;
		fn supports_contests(&self) -> bool;
	}
	pub struct Session {
		backend: &'static dyn Backend,
		raw: Box<dyn Any+Send+Sync+'static>,
	}
	pub type BoxedContestDetails = ContestDetails<BoxedContest>;
	pub struct BoxedContest {
		raw: Box<dyn AnyDebug+Send+Sync+'static>,
	}
	pub struct BoxedTask {
		raw: Box<dyn AnyDebug+Send+Sync+'static>,
	}
	pub type BoxedURL = URL<BoxedContest, BoxedTask>;
	pub type BoxedTaskURL = URL<!, BoxedTask>;
	pub type BoxedContestURL = URL<BoxedContest, !>;
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

		pub fn task_details(&self, task: &BoxedTask) -> Result<TaskDetails> {
			self.backend.task_details(self.raw.deref(), task.raw.deref())
		}

		pub fn task_languages(&self, task: &BoxedTask) -> Result<Vec<Language>> {
			self.backend.task_languages(self.raw.deref(), task.raw.deref())
		}

		pub fn task_submissions(&self, task: &BoxedTask) -> Result<Vec<Submission>> {
			self.backend.task_submissions(self.raw.deref(), task.raw.deref())
		}

		pub fn task_submit(&self, task: &BoxedTask, language: &Language, code: &str) -> Result<String> {
			self.backend.task_submit(self.raw.deref(), task.raw.deref(), language, code)
		}

		pub fn task_url(&self, task: &BoxedTask) -> Result<String> {
			self.backend.task_url(self.raw.deref(), task.raw.deref())
		}

		pub fn contests(&self) -> Result<Vec<BoxedContestDetails>> {
			self.backend.contests(self.raw.deref())
		}

		pub fn contest_tasks(&self, contest: &BoxedContest) -> Result<Vec<BoxedTask>> {
			self.backend.contest_tasks(self.raw.deref(), contest.raw.deref().as_any())
		}

		pub fn contest_id(&self, contest: &BoxedContest) -> Result<String> {
			self.backend.contest_id(contest.raw.deref().as_any())
		}

		pub fn contest_url(&self, contest: &BoxedContest) -> Result<String> {
			self.backend.contest_url(contest.raw.deref().as_any())
		}

		pub fn contest_site_prefix(&self) -> &'static str {
			self.backend.contest_site_prefix()
		}

		pub fn site_short(&self) -> &'static str {
			self.backend.site_short()
		}
	}

	impl<T: crate::Backend> Backend for T {
		fn accepted_domains(&self) -> &'static [&'static str] {
			T::accepted_domains(self)
		}

		fn deconstruct_url(&self, url: &str) -> Result<Option<BoxedURL>> {
			let url: Url = url.parse()?;
			let domain = url.domain().ok_or(Error::WrongTaskUrl)?;
			if self.accepted_domains().contains(&domain) {
				let segments = url.path_segments().ok_or(Error::WrongTaskUrl)?.filter(|s| !s.is_empty()).collect::<Vec<_>>();
				let resource = <T as crate::Backend>::deconstruct_resource(self, domain, &segments)?;
				Ok(Some(URL {
					domain: domain.to_owned(),
					site: format!("https://{}", domain),
					resource: match resource {
						Resource::Contest(c) => Resource::Contest(BoxedContest { raw: Box::new(c) }),
						Resource::Task(c) => Resource::Task(BoxedTask { raw: Box::new(c) }),
					},
				}))
			} else {
				Ok(None)
			}
		}

		fn connect(&'static self, domain: &str, user_agent: &str) -> Result<Session> {
			let client = reqwest::ClientBuilder::new()
				.cookie_store(true)
				.default_headers(vec![(USER_AGENT, reqwest::header::HeaderValue::from_str(user_agent).unwrap())].into_iter().collect())
				.build()
				.map_err(Error::TLSFailure)?;
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

		fn task_details(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<TaskDetails> {
			<T as crate::Backend>::task_details(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.as_any().downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
			)
		}

		fn task_languages(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<Vec<Language>> {
			<T as crate::Backend>::task_languages(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.as_any().downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
			)
		}

		fn task_submissions(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<Vec<Submission>> {
			<T as crate::Backend>::task_submissions(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.as_any().downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
			)
		}

		fn task_submit(&self, session: &dyn Any, task: &dyn AnyDebug, language: &Language, code: &str) -> Result<String> {
			<T as crate::Backend>::task_submit(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.as_any().downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
				language,
				code,
			)
		}

		fn task_url(&self, session: &dyn Any, task: &dyn AnyDebug) -> Result<String> {
			Ok(<T as crate::Backend>::task_url(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				task.as_any().downcast_ref::<T::Task>().ok_or(Error::WrongData)?,
			))
		}

		fn contests(&self, session: &dyn Any) -> Result<Vec<BoxedContestDetails>> {
			Ok(<T as crate::Backend>::contests(self, session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?)?
				.into_iter()
				.map(|ContestDetails { id, title, start }| ContestDetails { id: BoxedContest { raw: Box::new(id) }, title, start })
				.collect())
		}

		fn contest_tasks(&self, session: &dyn Any, contest: &dyn Any) -> Result<Vec<BoxedTask>> {
			Ok(<T as crate::Backend>::contest_tasks(
				self,
				session.downcast_ref::<T::Session>().ok_or(Error::WrongData)?,
				contest.downcast_ref::<T::Contest>().ok_or(Error::WrongData)?,
			)?
			.into_iter()
			.map(|task| BoxedTask { raw: Box::new(task) })
			.collect())
		}

		fn contest_id(&self, contest: &dyn Any) -> Result<String> {
			Ok(<T as crate::Backend>::contest_id(self, contest.downcast_ref::<T::Contest>().ok_or(Error::WrongData)?))
		}

		fn contest_url(&self, contest: &dyn Any) -> Result<String> {
			Ok(<T as crate::Backend>::contest_url(self, contest.downcast_ref::<T::Contest>().ok_or(Error::WrongData)?))
		}

		fn contest_site_prefix(&self) -> &'static str {
			<T as crate::Backend>::contest_site_prefix(self)
		}

		fn site_short(&self) -> &'static str {
			<T as crate::Backend>::site_short(self)
		}

		fn supports_contests(&self) -> bool {
			T::SUPPORTS_CONTESTS
		}
	}

	pub trait AnyDebug: Any+Debug {
		fn as_any(&self) -> &dyn Any;
		fn as_debug(&self) -> &dyn Debug;
	}
	impl<T: Any+Debug> AnyDebug for T {
		fn as_any(&self) -> &dyn Any {
			self
		}

		fn as_debug(&self) -> &dyn Debug {
			self
		}
	}

	impl Debug for BoxedContest {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			Debug::fmt(self.raw.deref(), f)
		}
	}
	impl Debug for BoxedTask {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			Debug::fmt(self.raw.deref(), f)
		}
	}
}
