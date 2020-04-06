#![feature(never_type, type_alias_impl_trait)]

pub extern crate chrono;
pub extern crate debris;
pub extern crate html5ever;
pub extern crate log;
pub extern crate reqwest;
pub extern crate scraper;
pub extern crate selectors;
pub extern crate serde;
pub extern crate url;

pub mod boxed;
mod error;
pub mod http;
pub mod json;
#[macro_use]
pub mod statement;

pub use error::{Error, ErrorCode, Result};

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Debug;
use url::Url;

#[derive(Clone, Debug)]
pub struct Example {
	pub input: String,
	pub output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Statement {
	HTML {
		html: String,
	},
	PDF {
		#[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
		pdf: Vec<u8>,
	},
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
	IdlenessLimitExceeded,
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
	Glitch,
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

#[async_trait(?Send)]
pub trait Backend: Debug+Send+Sync+'static {
	type CachedAuth: Debug+Send+Sync+'static;
	type Contest: Debug+Send+Sync+'static;
	type Session: Debug+Send+Sync+'static;
	type Task: Debug+Send+Sync+'static;
	fn accepted_domains(&self) -> &'static [&'static str];
	fn deconstruct_resource(&self, domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>>;
	fn deconstruct_url(&self, url: &str) -> Result<Option<URL<Self::Contest, Self::Task>>> {
		let url: Url = url.parse()?;
		let domain = url.domain().ok_or(ErrorCode::WrongTaskUrl)?;
		if self.accepted_domains().contains(&domain) {
			let segments =
				url.path_segments().ok_or(ErrorCode::WrongTaskUrl)?.filter(|s| !s.is_empty()).collect::<Vec<_>>();
			let resource = self.deconstruct_resource(domain, &segments)?;
			Ok(Some(URL { domain: domain.to_owned(), site: format!("https://{}", domain), resource }))
		} else {
			Ok(None)
		}
	}
	fn connect(&self, client: http::Client, domain: &str) -> Self::Session;
	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>>;
	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth>;
	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()>;
	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()>;
	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String>;
	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest>;
	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails>;
	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>>;
	async fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>>;
	async fn task_submit(
		&self,
		session: &Self::Session,
		task: &Self::Task,
		language: &Language,
		code: &str,
	) -> Result<String>;
	fn task_url(&self, session: &Self::Session, task: &Self::Task) -> Result<String>;
	fn submission_url(&self, session: &Self::Session, task: &Self::Task, id: &str) -> String;
	fn contest_id(&self, contest: &Self::Contest) -> String;
	fn contest_site_prefix(&self) -> &'static str;
	async fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>>;
	fn contest_url(&self, contest: &Self::Contest) -> String;
	async fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String>;
	async fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>>;
	fn name_short(&self) -> &'static str;
	fn supports_contests(&self) -> bool;
}

fn as_base64<T: AsRef<[u8]>, S: Serializer>(buffer: &T, serializer: S) -> std::result::Result<S::Ok, S::Error> {
	serializer.serialize_str(&hex::encode(buffer.as_ref()))
}
fn from_base64<'d, D: Deserializer<'d>>(deserializer: D) -> std::result::Result<Vec<u8>, D::Error> {
	<&str as Deserialize<'d>>::deserialize(deserializer)
		.and_then(|buffer| hex::decode(buffer).map_err(|e| serde::de::Error::custom(e.to_string())))
}

pub fn deserialize_auth<'d, T: Deserialize<'d>>(data: &'d str) -> Result<T> {
	Ok(serde_json::from_str(data).map_err(|_| ErrorCode::MalformedData)?)
}
pub fn serialize_auth<T: Serialize>(auth: &T) -> Result<String> {
	Ok(serde_json::to_string(auth).map_err(|_| ErrorCode::MalformedData)?)
}
