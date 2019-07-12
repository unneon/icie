use std::{iter::FromIterator, sync::Mutex};
use unijudge::{
	debris::{Context, Find}, reqwest::{
		self, header::{ORIGIN, REFERER, USER_AGENT}, Url
	}, Error, Result, Submission, TaskUrl
};

pub struct Codeforces;

struct Session {
	client: reqwest::Client,
	username: Mutex<Option<String>>,
}
struct Contest<'s> {
	id: String,
	session: &'s Session,
}
struct Task<'s> {
	id: String,
	contest: &'s Contest<'s>,
}

impl unijudge::Backend for Codeforces {
	fn deconstruct_url(&self, url: &str) -> Result<Option<TaskUrl>> {
		let url: Url = match url.parse() {
			Ok(url) => url,
			Err(_) => return Ok(None),
		};
		let segs: Vec<_> = url.path_segments().map_or(Vec::new(), |segs| segs.filter(|seg| !seg.is_empty()).collect());
		if url.domain() != Some("codeforces.com") {
			return Ok(None);
		}
		let (contest, task) = match segs.as_slice() {
			["problemset", "problem", contest, task] => (format!("problemset"), format!("{}/{}", contest, task)),
			// TODO if the first task is not A(e.g. A1), this won't work
			["contest", contest, "problem", "0"] => (format!("contest/{}", contest), format!("A")),
			["contest", contest, "problem", task] => (format!("contest/{}", contest), format!("{}", task)),
			["gym", contest, "problem", task] => (format!("gym/{}", contest), format!("{}", task)),
			_ => return Err(Error::WrongTaskUrl),
		};
		Ok(Some(TaskUrl {
			site: "https://codeforces.com".to_owned(),
			contest,
			task,
		}))
	}

	fn connect<'s>(&'s self, _site: &str, user_agent: &str) -> Result<Box<dyn unijudge::Session+'s>> {
		Ok(Box::new(Session {
			client: reqwest::Client::builder()
				.cookie_store(true)
				.default_headers(reqwest::header::HeaderMap::from_iter(vec![(
					USER_AGENT,
					reqwest::header::HeaderValue::from_str(user_agent).unwrap(),
				)]))
				.build()
				.map_err(Error::TLSFailure)?,
			username: Mutex::new(None),
		}))
	}
}

impl Session {
	fn fetch_csrf(&self) -> Result<String> {
		let url = self.url();
		let mut resp = self.client.get(url).send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		let csrf = doc.find(".csrf-token")?.attr("data-csrf")?.string();
		Ok(csrf)
	}

	fn url(&self) -> Url {
		"https://codeforces.com".parse().unwrap()
	}
}
impl unijudge::Session for Session {
	fn login(&self, username: &str, password: &str) -> Result<()> {
		let csrf = self.fetch_csrf()?;
		let mut resp = self
			.client
			.post("https://codeforces.com/enter")
			.header(ORIGIN, "https://codeforces.com")
			.header(REFERER, "https://codeforces.com/enter?back=/")
			.query(&[("back", "/")])
			.form(&[
				("action", "enter"),
				("csrf_token", &csrf),
				("handleOrEmail", username),
				("password", password),
				("remember", "on"),
			])
			.send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		let login_succeeded = doc
			.find_all(".lang-chooser a")
			.any(|v| v.attr("href").map(|href| href.string()).ok() == Some(format!("/profile/{}", username)));
		let wrong_password_or_handle = doc.find_all(".for__password").count() == 1;
		if login_succeeded {
			Ok(())
		} else if wrong_password_or_handle {
			Err(Error::WrongCredentials)?
		} else {
			Err(doc.error("unrecognized logic outcome"))?
		}
	}

	fn restore_auth(&self, id: &str) -> Result<()> {
		let c = serde_json::from_str(id).map_err(|_| Error::WrongData)?;
		let mut cs = self.client.cookies().write().unwrap();
		cs.0.insert(c, &"https://codeforces.com".parse().unwrap()).unwrap();
		Ok(())
	}

	fn cache_auth(&self) -> Result<Option<String>> {
		let cs = self.client.cookies().read().unwrap();
		let c = cs.0.get("codeforces.com", "/", "JSESSIONID");
		let serialized = c.map(|c| serde_json::to_string(&c).unwrap());
		Ok(serialized)
	}

	fn contest<'s>(&'s self, id: &str) -> Result<Box<dyn unijudge::Contest+'s>> {
		Ok(Box::new(Contest { id: id.to_owned(), session: self }))
	}
}

impl Contest<'_> {
	fn url(&self) -> Url {
		self.session.url().join(&format!("{}/", self.id)).unwrap()
	}
}
impl unijudge::Contest for Contest<'_> {
	fn task<'s>(&'s self, id: &str) -> Result<Box<dyn unijudge::Task+'s>> {
		Ok(Box::new(Task { id: id.to_owned(), contest: self }))
	}
}

impl Task<'_> {
	fn url(&self) -> Url {
		self.contest.url().join(&format!("problem/{}/", self.id)).unwrap()
	}
}
impl unijudge::Task for Task<'_> {
	fn details(&self) -> Result<unijudge::TaskDetails> {
		let url = self.url();
		let mut resp = self.contest.session.client.get(url).send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		let (symbol, title) = doc.find(".problem-statement > .header > .title")?.text().map(|full| {
			let i = match full.find('.') {
				Some(i) => i,
				None => return Err("full problem title does not have a symbol prefix"),
			};
			Ok((full[..i].trim().to_owned(), full[i + 1..].trim().to_owned()))
		})?;
		let examples = doc
			.find_all(".sample-test .input")
			.zip(doc.find_all(".sample-test .output"))
			.map(|(input, output)| {
				Ok(unijudge::Example {
					input: input.child(1)?.text_br().string(),
					output: output.child(1)?.text_br().string(),
				})
			})
			.collect::<Result<_>>()?;
		Ok(unijudge::TaskDetails {
			symbol,
			title,
			contest_id: if self.contest.id.starts_with("contest/") {
				self.contest.id[8..].to_owned()
			} else {
				self.contest.id.clone()
			},
			site_short: "cf".to_owned(),
			examples: Some(examples),
		})
	}

	fn languages(&self) -> Result<Vec<unijudge::Language>> {
		let url = self.contest.url().join("submit").unwrap();
		let mut resp = self.contest.session.client.get(url).send()?;
		if resp.url().as_str() == "https://codeforces.com/" {
			return Err(Error::AccessDenied);
		}
		let doc = unijudge::debris::Document::new(&resp.text()?);
		let languages = doc
			.find_all("[name=\"programTypeId\"] option")
			.map(|opt| {
				Ok(unijudge::Language {
					id: opt.attr("value")?.as_str().trim().to_owned(),
					name: opt.text().string(),
				})
			})
			.collect::<Result<_>>()?;
		Ok(languages)
	}

	fn submissions(&self) -> Result<Vec<Submission>> {
		let mut resp = if self.id != "problemset" {
			let url = self.contest.url().join("my").unwrap();
			let resp = self.contest.session.client.get(url).send()?;
			resp
		} else {
			let url = self
				.contest
				.session
				.url()
				.join("submissions/")
				.unwrap()
				.join(&self.contest.session.username.lock().unwrap().as_ref().ok_or(Error::AccessDenied)?)
				.unwrap();
			let resp = self.contest.session.client.get(url).send()?;
			resp
		};
		let doc = unijudge::debris::Document::new(&resp.text()?);
		Ok(doc
			.find_all("[data-submission-id]")
			.map(|node| {
				let kids = node.find_all("td").collect::<Vec<_>>();
				let id = kids[0].child(1)?.text().string();
				let verdict = if kids[5].text() == "In queue" {
					Verdict::InQueue
				} else if kids[5].text() == "Running" {
					Verdict::TestingStart
				} else {
					let verdict_span = kids[5].find_first("span")?;
					let verdict_tag = verdict_span.attr("submissionverdict")?;
					match verdict_tag.as_str() {
						"OK" => Verdict::Accepted,
						"WRONG_ANSWER" => Verdict::WrongAnswer(TestIndex::scrap(verdict_span)?),
						"COMPILATION_ERROR" => Verdict::CompilationError,
						"TESTING" => Verdict::Testing(TestIndex::scrap(verdict_span)?),
						"RUNTIME_ERROR" => Verdict::RuntimeError(TestIndex::scrap(verdict_span)?),
						"TIME_LIMIT_EXCEEDED" => Verdict::TimeLimitExceeded(TestIndex::scrap(verdict_span)?),
						"MEMORY_LIMIT_EXCEEDED" => Verdict::MemoryLimitExceeded(TestIndex::scrap(verdict_span)?),
						"PARTIAL" => Verdict::Partial(verdict_span.find(".verdict-format-points")?.text().parse()?),
						"SKIPPED" => Verdict::Skipped,
						"CHALLENGED" => Verdict::Hacked,
						_ => Err(verdict_span.error("unrecognized verdict tag"))?,
					}
				}
				.to_unijudge();
				Ok(Submission { id, verdict })
			})
			.collect::<Result<Vec<_>>>()?)
	}

	fn submit(&self, language: &unijudge::Language, code: &str) -> Result<String> {
		let url = self.contest.url().join("submit").unwrap();
		let mut resp1 = self.contest.session.client.get(url.clone()).send()?;

		let referer = resp1.url().clone();
		let csrf = {
			let doc = unijudge::debris::Document::new(&resp1.text()?);
			doc.find_first("[name=\"csrf_token\"]")?.attr("value")?.string()
		};

		let (contest_id, problem_index) = if self.contest.id == "problemset" {
			(self.id[..self.id.find('/').unwrap()].to_string(), self.id[self.id.find('/').unwrap() + 1..].to_string())
		} else {
			(self.contest.id.clone(), self.id.clone())
		};
		let form = reqwest::multipart::Form::new()
			.text("csrf_token", csrf.clone())
			.text("ftaa", "")
			.text("bfaa", "")
			.text("action", "submitSolutionFormSubmitted")
			.text("contestId", contest_id)
			.text("submittedProblemIndex", problem_index)
			.text("programTypeId", language.id.clone())
			.text("source", code.to_string())
			.text("tabSize", "4");

		self.contest
			.session
			.client
			.post(url.clone())
			.header(ORIGIN, "https://codeforces.com")
			.header(REFERER, referer.as_str())
			.query(&[("csrf_token", &csrf)])
			.multipart(form)
			.send()?;

		Ok(self.submissions()?[0].id.to_string())
	}
}

#[derive(Clone, Debug)]
enum TestIndex {
	Test(i64),
	Pretest(i64),
	Hack(i64),
}
#[derive(Clone, Debug)]
enum Verdict {
	Accepted,
	MemoryLimitExceeded(TestIndex),
	WrongAnswer(TestIndex),
	TimeLimitExceeded(TestIndex),
	RuntimeError(TestIndex),
	Partial(i64),
	Testing(TestIndex),
	Hacked,
	CompilationError,
	InQueue,
	TestingStart,
	Skipped,
}

impl TestIndex {
	fn scrap(span: unijudge::debris::Node) -> unijudge::debris::Result<TestIndex> {
		let txt = span.child(0)?;
		let txt = txt.text_child(0)?;
		let num = span.find(".verdict-format-judged")?.text().parse()?;
		Ok(if txt.as_str().contains("hack") {
			TestIndex::Hack(num)
		} else if txt.as_str().contains("pretest") {
			TestIndex::Pretest(num)
		} else if txt.as_str().contains("test") {
			TestIndex::Test(num)
		} else {
			Err(txt.error("unrecognized test index"))?
		})
	}

	fn desc(&self) -> String {
		match self {
			TestIndex::Test(i) => format!("test {}", i),
			TestIndex::Hack(i) => format!("hack {}", i),
			TestIndex::Pretest(i) => format!("pretest {}", i),
		}
	}
}

impl Verdict {
	fn to_unijudge(&self) -> unijudge::Verdict {
		use unijudge::{RejectionCause as UR, Verdict as UV};
		use Verdict as CV;
		match self {
			CV::Accepted => UV::Accepted,
			CV::MemoryLimitExceeded(ti) => UV::Rejected {
				cause: Some(UR::MemoryLimitExceeded),
				test: Some(ti.desc()),
			},
			CV::WrongAnswer(ti) => UV::Rejected {
				cause: Some(UR::WrongAnswer),
				test: Some(ti.desc()),
			},
			CV::TimeLimitExceeded(ti) => UV::Rejected {
				cause: Some(UR::TimeLimitExceeded),
				test: Some(ti.desc()),
			},
			CV::RuntimeError(ti) => UV::Rejected {
				cause: Some(UR::RuntimeError),
				test: Some(ti.desc()),
			},
			CV::Testing(ti) => UV::Pending { test: Some(ti.desc()) },
			CV::Partial(score) => UV::Scored {
				score: *score as f64,
				max: None,
				cause: None,
				test: None,
			},
			CV::Hacked => UV::Rejected {
				cause: None,
				test: Some(String::from("a hack")),
			},
			CV::CompilationError => UV::Rejected {
				cause: Some(UR::CompilationError),
				test: None,
			},
			CV::InQueue => UV::Pending { test: None },
			CV::TestingStart => UV::Pending { test: None },
			CV::Skipped => UV::Skipped,
		}
	}
}
