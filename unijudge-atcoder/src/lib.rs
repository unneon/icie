use std::iter::FromIterator;
use unijudge::{
	debris::{self, Context, Find}, reqwest::{
		self, header::{ORIGIN, REFERER, USER_AGENT}, Url
	}, Error, Example, Language, RejectionCause, Result, Submission, TaskDetails, TaskUrl, Verdict
};

pub struct Atcoder;

struct Session {
	client: reqwest::Client,
}
struct Contest<'s> {
	id: String,
	session: &'s Session,
}
struct Task<'s> {
	id: String,
	contest: &'s Contest<'s>,
}

impl unijudge::Backend for Atcoder {
	fn deconstruct_url(&self, url: &str) -> Result<Option<TaskUrl>> {
		let url: Url = match url.parse() {
			Ok(url) => url,
			Err(_) => return Ok(None),
		};
		let segs: Vec<_> = url.path_segments().map_or(Vec::new(), |segs| segs.filter(|seg| !seg.is_empty()).collect());
		if url.domain() != Some("atcoder.jp") {
			return Ok(None);
		}
		let (contest, task) = match segs.as_slice() {
			["contests", contest, "tasks", task] => (String::from(*contest), String::from(*task)),
			_ => return Err(Error::WrongTaskUrl),
		};
		Ok(Some(TaskUrl { site: "https://atcoder.jp".to_owned(), contest, task }))
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
		}))
	}
}

impl Session {
	fn fetch_login_csrf(&self) -> Result<String> {
		let url: Url = "https://atcoder.jp/login".parse().unwrap();
		let mut resp = self.client.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		Ok(doc.find_first("[name=\"csrf_token\"]")?.attr("value")?.string())
	}
}
impl unijudge::Session for Session {
	fn login(&self, username: &str, password: &str) -> Result<()> {
		let csrf = self.fetch_login_csrf()?;
		let url: Url = "https://atcoder.jp/login".parse().unwrap();
		let mut resp = match self
			.client
			.post(url)
			.header(ORIGIN, "https://atcoder.jp")
			.header(REFERER, "https://atcoder.jp/login")
			.form(&[("username", username), ("password", password), ("csrf_token", &csrf)])
			.send()
		{
			Ok(resp) => resp,
			// this is the worst way to indicate wrong password I have heard of
			Err(ref e) if format!("{}", e).contains("Infinite redirect loop") => return Err(Error::WrongCredentials),
			Err(e) => return Err(Error::NetworkFailure(e)),
		};
		let doc = debris::Document::new(&resp.text()?);
		if doc.find("#main-container > div.row > div.alert.alert-success").is_ok() {
			Ok(())
		} else if doc.find("#main-container > div.row > div.alert.alert-danger").is_ok() {
			Err(Error::WrongCredentials)
		} else {
			Err(Error::UnexpectedHTML(doc.error("unrecognized login outcome")))
		}
	}

	fn restore_auth(&self, id: &str) -> Result<()> {
		self.client
			.cookies()
			.write()
			.unwrap()
			.0
			.insert(serde_json::from_str(id).map_err(|_| Error::WrongData)?, &"https://atcoder.jp".parse().unwrap())
			.map_err(|_| Error::WrongData)?;
		Ok(())
	}

	fn cache_auth(&self) -> Result<Option<String>> {
		let cookies = self.client.cookies().read().unwrap();
		match cookies.0.get("atcoder.jp", "/", "REVEL_SESSION") {
			Some(c) => Ok(Some(serde_json::to_string(c).unwrap())),
			None => Ok(None),
		}
	}

	fn contest<'s>(&'s self, id: &str) -> Result<Box<dyn unijudge::Contest+'s>> {
		Ok(Box::new(Contest { id: id.to_owned(), session: self }))
	}
}

impl unijudge::Contest for Contest<'_> {
	fn task<'s>(&'s self, id: &str) -> Result<Box<dyn unijudge::Task+'s>> {
		Ok(Box::new(Task { id: id.to_owned(), contest: self }))
	}
}

impl unijudge::Task for Task<'_> {
	fn details(&self) -> Result<TaskDetails> {
		let url: Url = format!("https://atcoder.jp/contests/{}/tasks/{}", self.contest.id, self.id).parse().unwrap();
		let mut resp = self.contest.session.client.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		let (symbol, title) = doc.find("#main-container > .row > div > span.h2")?.text().map(|text| {
			let mark = text.find("-").ok_or("no dash(-) found in task title")?;
			std::result::Result::<_, &'static str>::Ok((text[..mark - 1].to_owned(), text[mark + 2..].to_owned()))
		})?;
		let parts = doc
			.find_all("#task-statement > .lang > .lang-en > .part")
			.filter(|node| {
				let title = match node.find("h3") {
					Ok(h3) => h3,
					Err(_) => return false,
				}
				.text()
				.string();
				title.starts_with("Sample Input") || title.starts_with("Sample Output")
			})
			.map(|node| Ok(node.find_first("pre")?.text().string()))
			.collect::<Result<Vec<_>>>()?;
		let examples = Some(
			parts
				.chunks(2)
				.map(|pres| match pres {
					[input, output] => Ok(Example { input: input.to_string(), output: output.to_string() }),
					_ => Err(doc.error("sample input with no matching output")),
				})
				.collect::<debris::Result<_>>()?,
		);
		Ok(TaskDetails { symbol, title, contest_id: self.contest.id.clone(), site_short: "atc".to_owned(), examples })
	}

	fn languages(&self) -> Result<Vec<Language>> {
		let url: Url = format!("https://atcoder.jp/contests/{}/submit", self.contest.id).parse().unwrap();
		let mut resp = self.contest.session.client.get(url).send()?;
		if resp.url().path() == "/login" {
			return Err(Error::AccessDenied);
		}
		let doc = debris::Document::new(&resp.text()?);
		let selection_id = format!("select-lang-{}", self.id);
		Ok(doc
			.find_all("#select-lang > div")
			.find(|pll| {
				match pll.attr("id") {
					Ok(id) => id,
					Err(_) => return false,
				}
				.string() == selection_id
			})
			.ok_or_else(|| doc.error(format!("no lang list with id equal to {}", selection_id)))?
			.find_all("option")
			.map(|opt| Ok(Language { id: opt.attr("value")?.string(), name: opt.text().string() }))
			.collect::<Result<_>>()?)
	}

	fn submissions(&self) -> Result<Vec<Submission>> {
		let url: Url = format!("https://atcoder.jp/contests/{}/submissions/me", self.contest.id).parse().unwrap();
		let mut resp = self.contest.session.client.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		Ok(doc
			.find_all(".panel-submission tbody > tr")
			.map(|row| {
				let id = row.find(".submission-score")?.attr("data-id")?.string();
				let status = row.find("td > span")?;
				let status = status.text();
				let (test_index, verdict) = match status.as_str().find(" ") {
					Some(i) => (Some(&status.as_str()[..i]), Some(&status.as_str()[i + 1..])),
					None if status.as_str().starts_with(char::is_numeric) => (Some(status.as_str()), None),
					None => (None, Some(status.as_str())),
				};
				let verdict = match (verdict, test_index) {
					(None, Some(index)) => Verdict::Pending { test: Some(index.to_owned()) },
					(Some("WJ"), None) => Verdict::Pending { test: None },
					(Some(verdict), _) => Verdict::Scored {
						score: row.find(".submission-score")?.text().parse::<f64>()?,
						max: None,
						cause: match verdict {
							"AC" => None,
							"WA" => Some(RejectionCause::WrongAnswer),
							"RE" => Some(RejectionCause::RuntimeError),
							"TLE" => Some(RejectionCause::TimeLimitExceeded),
							"CE" => Some(RejectionCause::CompilationError),
							_ => {
								return Err(status.error(format!(
									"unrecognized Atcoder verdict {:?} [{:?} {:?}]",
									status.as_str(),
									verdict,
									test_index
								)));
							},
						},
						test: None,
					},
					(None, None) => {
						return Err(status.error(format!("unrecognized Atcoder verdict {:?} [{:?} {:?}]", status.as_str(), verdict, test_index)));
					},
				};
				Ok(Submission { id, verdict })
			})
			.collect::<debris::Result<_>>()?)
	}

	fn submit(&self, language: &Language, code: &str) -> Result<String> {
		let csrf = self.contest.session.fetch_login_csrf()?;
		let url: Url = format!("https://atcoder.jp/contests/{}/submit", self.contest.id).parse().unwrap();
		self.contest
			.session
			.client
			.post(url)
			.form(&[("data.TaskScreenName", &self.id), ("data.LanguageId", &language.id), ("sourceCode", &String::from(code)), ("csrf_token", &csrf)])
			.send()?;
		Ok(self.submissions()?[0].id.to_string())
	}
}
