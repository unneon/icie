#![feature(never_type)]

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
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Debug};
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
		// Use base64 when (de)serializing to reduce file sizes.
		#[serde(with = "hex::serde")]
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
pub enum ContestTime {
	Upcoming { start: DateTime<FixedOffset> },
	Ongoing { finish: DateTime<FixedOffset> },
}

#[derive(Clone, Debug)]
pub struct ContestDetails<I> {
	pub id: I,
	pub title: String,
	pub time: ContestTime,
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
	async fn rank_list(&self, session: &Self::Session, task: &Self::Task) -> Result<String>;
	async fn remain_time(&self, session: &Self::Session, task: &Self::Task) -> Result<i64>;
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

pub fn deserialize_auth<'d, T: Deserialize<'d>>(data: &'d str) -> Result<T> {
	Ok(serde_json::from_str(data).map_err(|_| ErrorCode::MalformedData)?)
}
pub fn serialize_auth<T: Serialize>(auth: &T) -> Result<String> {
	Ok(serde_json::to_string(auth).map_err(|_| ErrorCode::MalformedData)?)
}

impl fmt::Display for RejectionCause {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match self {
			RejectionCause::WrongAnswer => "Wrong Answer",
			RejectionCause::RuntimeError => "Runtime Error",
			RejectionCause::TimeLimitExceeded => "Time Limit Exceeded",
			RejectionCause::MemoryLimitExceeded => "Memory Limit Exceeded",
			RejectionCause::RuleViolation => "Rule Violation",
			RejectionCause::SystemError => "System Error",
			RejectionCause::CompilationError => "Compilation Error",
			RejectionCause::IdlenessLimitExceeded => "Idleness Limit Exceeded",
		})
	}
}

impl fmt::Display for Verdict {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Verdict::Scored { score, max, cause, test } => {
				let out_of = max.map(|max| format!(" out of {}", max)).unwrap_or_default();
				write!(f, "Scored {}{}{}{}", score, out_of, fmt_verdict_cause(cause, test), fmt_verdict_test(test))
			},
			Verdict::Accepted => write!(f, "Accepted"),
			Verdict::Rejected { cause, test } => {
				write!(f, "Rejected{}{}", fmt_verdict_cause(cause, test), fmt_verdict_test(test))
			},
			Verdict::Pending { test } => write!(f, "Pending{}", fmt_verdict_test(test)),
			Verdict::Skipped => write!(f, "Skipped"),
			Verdict::Glitch => write!(f, "Glitched"),
		}
	}
}

fn fmt_verdict_cause(cause: &Option<RejectionCause>, test: &Option<String>) -> String {
	match (cause, test) {
		(Some(cause), _) => format!(" due to {}", cause),
		(None, Some(_)) => " failing".to_owned(),
		(None, None) => "".to_owned(),
	}
}

fn fmt_verdict_test(test: &Option<String>) -> String {
	test.as_ref().map(|test| format!(" on {}", test)).unwrap_or_default()
}
pub fn fmt_title(val:i64)-> String{
	let mut num=val;
	if num==-1{
		return "unscored_".to_owned();
	}
	let mut to_str:String="_".to_string();
	let mut base=26;
	while(num!=0){
		let mut intVar:u8  = (97+num%base).try_into().unwrap();
		let mut charVar:char;
		
		//println!("{}-{}",num,intVar);
		
		if to_str.len()>=1 && num<base {intVar-=1;}
		num/=base;
		if to_str.len()==0 { base+=1;}
		charVar=intVar as char;
		to_str.push(charVar);
		
	}
	to_str.chars().rev().collect()
}