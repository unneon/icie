#[derive(Debug)]
pub enum Error {
	WrongCredentials,
	WrongData,
	WrongTaskUrl,
	AccessDenied,
	NetworkFailure(reqwest::Error),
	TLSFailure(reqwest::Error),
	UnexpectedHTML(debris::Error),
}
impl From<reqwest::Error> for Error {
	fn from(e: reqwest::Error) -> Self {
		Error::NetworkFailure(e)
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
			Error::UnexpectedHTML(e) => Some(e),
		}
	}
}

pub extern crate debris;
pub extern crate reqwest;

use std::fmt;

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

#[derive(Clone, Debug)]
pub struct TaskUrl {
	pub site: String,
	pub contest: String,
	pub task: String,
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
	fn accepted_domains(&self) -> &'static [&'static str];
	fn deconstruct_segments(&self, domain: &str, segments: &[&str]) -> Result<TaskUrl>;
	fn deconstruct_url(&self, url: &str) -> Result<Option<TaskUrl>> {
		let url: reqwest::Url = match url.parse() {
			Ok(url) => url,
			Err(_) => return Ok(None),
		};
		let segments: Vec<_> = url.path_segments().map_or(Vec::new(), |segs| segs.filter(|seg| !seg.is_empty()).collect());
		let domain = match url.domain() {
			Some(domain) => domain,
			None => return Ok(None),
		};
		if !self.accepted_domains().contains(&domain) {
			return Ok(None);
		}
		self.deconstruct_segments(domain, &segments).map(Some)
	}
	fn connect<'s>(&'s self, site: &str, user_agent: &str) -> Result<Box<dyn Session+'s>>;
}

pub trait Session {
	fn login(&self, username: &str, password: &str) -> Result<()>;
	fn restore_auth(&self, id: &str) -> Result<()>;
	fn cache_auth(&self) -> Result<Option<String>>;
	fn contest<'s>(&'s self, id: &str) -> Result<Box<dyn Contest+'s>>;
}

pub trait Contest {
	fn task<'s>(&'s self, id: &str) -> Result<Box<dyn Task+'s>>;
}

pub trait Task {
	fn details(&self) -> Result<TaskDetails>;
	fn languages(&self) -> Result<Vec<Language>>;
	fn submissions(&self) -> Result<Vec<Submission>>;
	fn submit(&self, language: &Language, code: &str) -> Result<String>;
}

pub struct FixedSiteTaskUrl {
	site: String,
}

impl TaskUrl {
	pub fn new(site: impl Into<String>, contest: impl Into<String>, task: impl Into<String>) -> TaskUrl {
		TaskUrl { site: site.into(), contest: contest.into(), task: task.into() }
	}

	pub fn fix_site(site: impl Into<String>) -> FixedSiteTaskUrl {
		FixedSiteTaskUrl { site: site.into() }
	}
}

impl FixedSiteTaskUrl {
	pub fn new(self, contest: impl Into<String>, task: impl Into<String>) -> TaskUrl {
		TaskUrl::new(self.site, contest, task)
	}
}
