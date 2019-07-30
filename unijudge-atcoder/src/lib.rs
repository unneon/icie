use unijudge::{
	debris::{self, Context, Find}, reqwest::{
		self, cookie_store::Cookie, header::{ORIGIN, REFERER}, Url
	}, Error, Example, Language, RejectionCause, Result, Submission, TaskDetails, Verdict
};

pub struct Atcoder;

#[derive(Debug)]
pub struct Task {
	contest: String,
	task: String,
}

impl unijudge::Backend for Atcoder {
	type CachedAuth = Cookie<'static>;
	type Session = reqwest::Client;
	type Task = Task;

	fn accepted_domains(&self) -> &[&str] {
		&["atcoder.jp"]
	}

	fn deconstruct_task(&self, _domain: &str, segments: &[&str]) -> Result<Self::Task> {
		match segments {
			["contests", contest, "tasks", task] => Ok(Task { contest: (*contest).to_owned(), task: (*task).to_owned() }),
			_ => return Err(Error::WrongTaskUrl),
		}
	}

	fn connect(&self, client: reqwest::Client, _domain: &str) -> Self::Session {
		client
	}

	fn login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let csrf = self.fetch_login_csrf(session)?;
		let url: Url = "https://atcoder.jp/login".parse().unwrap();
		let mut resp = match session
			.post(url)
			.header(ORIGIN, "https://atcoder.jp")
			.header(REFERER, "https://atcoder.jp/login")
			.form(&[("username", username), ("password", password), ("csrf_token", &csrf)])
			.send()
		{
			Ok(resp) => resp,
			// this is the worst way to indicate wrong password I have heard of
			Err(ref e) if e.to_string().contains("Infinite redirect loop") => return Err(Error::WrongCredentials),
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

	fn restore_auth(&self, session: &Self::Session, auth: Self::CachedAuth) -> Result<()> {
		let mut cookies = session.cookies().write().unwrap();
		cookies.0.insert(auth, &"https://atcoder.jp".parse().unwrap()).map_err(|_| Error::WrongData)?;
		Ok(())
	}

	fn cache_auth(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let cookies = session.cookies().read().unwrap();
		Ok(cookies.0.get("atcoder.jp", "/", "REVEL_SESSION").map(|c| c.clone().into_owned()))
	}

	fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		let url: Url = format!("https://atcoder.jp/contests/{}/tasks/{}", task.contest, task.task).parse().unwrap();
		let mut resp = session.get(url).send()?;
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
		Ok(TaskDetails { symbol, title, contest_id: task.contest.clone(), site_short: "atc".to_owned(), examples })
	}

	fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		let url: Url = format!("https://atcoder.jp/contests/{}/submit", task.contest).parse().unwrap();
		let mut resp = session.get(url).send()?;
		if resp.url().path() == "/login" {
			return Err(Error::AccessDenied);
		}
		let doc = debris::Document::new(&resp.text()?);
		let selection_id = format!("select-lang-{}", task.task);
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

	fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>> {
		let url: Url = format!("https://atcoder.jp/contests/{}/submissions/me", task.contest).parse().unwrap();
		let mut resp = session.get(url).send()?;
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

	fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		let csrf = self.fetch_login_csrf(session)?;
		let url: Url = format!("https://atcoder.jp/contests/{}/submit", task.contest).parse().unwrap();
		session
			.post(url)
			.form(&[
				("data.TaskScreenName", &task.task),
				("data.LanguageId", &language.id),
				("sourceCode", &String::from(code)),
				("csrf_token", &csrf),
			])
			.send()?;
		Ok(self.task_submissions(session, task)?[0].id.to_string())
	}
}

impl Atcoder {
	fn fetch_login_csrf(&self, session: &reqwest::Client) -> Result<String> {
		let url: Url = "https://atcoder.jp/login".parse().unwrap();
		let mut resp = session.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		Ok(doc.find_first("[name=\"csrf_token\"]")?.attr("value")?.string())
	}
}
