use std::{iter::FromIterator, sync::Mutex};
use unijudge::{
	debris::{self, Context, Find}, reqwest::{
		self, header::{REFERER, USER_AGENT}, Url
	}, Error, Language, RejectionCause, Result, Submission, TaskDetails, TaskUrl, Verdict
};

pub struct Sio2;

struct Session {
	client: reqwest::Client,
	site: String,
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

impl unijudge::Backend for Sio2 {
	fn accepted_domains(&self) -> &'static [&'static str] {
		&["kiwi.ii.uni.wroc.pl", "main2.edu.pl", "sio2.mimuw.edu.pl", "sio2.staszic.waw.pl", "szkopul.edu.pl"]
	}

	fn deconstruct_segments(&self, domain: &str, segments: &[&str]) -> Result<TaskUrl> {
		let sio = TaskUrl::fix_site(format!("https://{}", domain));
		match segments {
			["c", contest, "p", task] => Ok(sio.new(*contest, *task)),
			["c", contest, "p", task, _] => Ok(sio.new(*contest, *task)),
			_ => Err(Error::WrongTaskUrl),
		}
	}

	fn connect<'s>(&'s self, site: &str, user_agent: &str) -> Result<Box<dyn unijudge::Session+'s>> {
		Ok(Box::new(Session {
			client: reqwest::Client::builder()
				.cookie_store(true)
				.default_headers(reqwest::header::HeaderMap::from_iter(vec![(
					USER_AGENT,
					reqwest::header::HeaderValue::from_str(user_agent).unwrap(),
				)]))
				.build()
				.map_err(Error::TLSFailure)?,
			site: site.to_owned(),
			username: Mutex::new(None),
		}))
	}
}

impl unijudge::Session for Session {
	fn login(&self, username: &str, password: &str) -> Result<()> {
		let url1: Url = format!("{}/login/", self.site).parse().unwrap();
		let mut resp1 = self.client.get(url1).send()?;
		let url2 = resp1.url().clone();
		let doc1 = debris::Document::new(&resp1.text()?);
		let csrf = doc1.find("input[name=\"csrfmiddlewaretoken\"]")?.attr("value")?.string();
		let mut resp2 = self
			.client
			.post(url2.clone())
			.header(REFERER, url2.as_str())
			.form(&[
				("csrfmiddlewaretoken", csrf.as_str()),
				("auth-password", password),
				("password", password),
				("auth-username", username),
				("username", username),
				("login_view-current_step", "auth"),
			])
			.send()?;
		let doc2 = debris::Document::new(&resp2.text()?);
		if doc2.find("#username").is_ok() {
			*self.username.lock().unwrap() = Some(username.to_owned());
			Ok(())
		} else if doc2.find("form")?.find("div.form-group > div > div.alert.alert-danger").is_ok() {
			Err(Error::WrongCredentials)
		} else {
			Err(Error::UnexpectedHTML(doc2.error("unrecognized login outcome")))
		}
	}

	fn restore_auth(&self, id: &str) -> Result<()> {
		let cached: CachedAuth = serde_json::from_str(id).map_err(|_| Error::WrongData)?;
		*self.username.lock().unwrap() = Some(cached.username);
		self.client.cookies().write().unwrap().0.insert(cached.sessionid, &self.site.parse().unwrap()).unwrap();
		Ok(())
	}

	fn cache_auth(&self) -> Result<Option<String>> {
		let cached = CachedAuth {
			username: match self.username.lock().unwrap().as_ref() {
				Some(username) => username.to_owned(),
				None => return Ok(None),
			},
			sessionid: match self.client.cookies().read().unwrap().0.get(Url::parse(&self.site).unwrap().domain().unwrap(), "/", "sessionid") {
				Some(c) => c.clone().into_owned(),
				None => return Ok(None),
			},
		};
		Ok(Some(serde_json::to_string(&cached).unwrap()))
	}

	fn contest<'s>(&'s self, id: &str) -> Result<Box<dyn unijudge::Contest+'s>> {
		Ok(Box::new(Contest { id: id.to_owned(), session: self }))
	}
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CachedAuth {
	username: String,
	sessionid: unijudge::reqwest::cookie_store::Cookie<'static>,
}

impl unijudge::Contest for Contest<'_> {
	fn task<'s>(&'s self, id: &str) -> Result<Box<dyn unijudge::Task+'s>> {
		Ok(Box::new(Task { id: id.to_owned(), contest: self }))
	}
}

impl unijudge::Task for Task<'_> {
	fn details(&self) -> Result<TaskDetails> {
		let url: Url = format!("{}/c/{}/p/", self.contest.session.site, self.contest.id).parse().unwrap();
		let mut resp = self.contest.session.client.get(url.clone()).send()?;
		if resp.url() != &url {
			return Err(Error::AccessDenied);
		}
		let doc = debris::Document::new(&resp.text()?);
		let problems = doc
			.find(".main-content > div > table > tbody")?
			.find_all("tr")
			.filter(|tr| tr.child(3).is_ok())
			.map(|tr| Ok((tr.child(1)?.text().string(), tr.find("a")?.text().string())))
			.collect::<Result<Vec<_>>>()?;
		let title = match problems.into_iter().find(|(id, _)| id == &self.id) {
			Some((_, title)) => title,
			None => return Err(Error::WrongData),
		};
		Ok(TaskDetails { symbol: self.id.to_string(), title, contest_id: self.contest.id.clone(), site_short: "sio2".to_owned(), examples: None })
	}

	fn languages(&self) -> Result<Vec<Language>> {
		let url: Url = format!("{}/c/{}/submit/", self.contest.session.site, self.contest.id).parse().unwrap();
		let mut resp = self.contest.session.client.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		if doc.find("#id_password").is_ok() {
			return Err(Error::AccessDenied);
		}
		Ok(doc
			.find_all("#id_prog_lang > option")
			.filter(|opt| opt.attr("selected").is_err())
			.map(|opt| Ok(Language { id: opt.attr("value")?.string(), name: opt.text().string() }))
			.collect::<Result<_>>()?)
	}

	fn submissions(&self) -> Result<Vec<Submission>> {
		let url: Url = format!("{}/c/{}/submissions/", self.contest.session.site, self.contest.id).parse().unwrap();
		let mut resp = self.contest.session.client.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		Ok(doc
			.find_all("section.main-content > div > table > tbody > tr")
			.filter(|tr| tr.child(3).is_ok())
			.map(|tr| {
				let status = tr.child(9)?.text().map(|status| match status {
					"OK" => Ok(Some(Status::Accepted)),
					"Wrong answer" => Ok(Some(Status::WrongAnswer)),
					"Time limit exceeded" => Ok(Some(Status::TimeLimitExceeded)),
					"Memory limit exceeded" => Ok(Some(Status::MemoryLimitExceeded)),
					"Runtime error" => Ok(Some(Status::RuntimeError)),
					"Compilation failed" | "CE" => Ok(Some(Status::CompilationFailed)),
					"Initial tests: OK" | "INI_OK" => Ok(None),
					"Initial tests: failed" | "INI_ERR" => Ok(None),
					"Pending" | "Oczekuje" => Ok(Some(Status::Pending)),
					_ => Err(format!("unrecognized submission status {:?}", status)),
				})?;
				let score = tr
					.child(11)?
					.text()
					.map(|score| {
						let score = &score[..score.find(" ").unwrap_or(score.len())];
						score.parse::<i64>()
					})
					.ok();
				Ok(Submission {
					id: tr.find("a")?.attr("href")?.map(|href| match href.split("/").filter(|seg| !seg.is_empty()).collect::<Vec<_>>().last() {
						Some(id) => Ok(String::from(*id)),
						None => Err("empty submission href"),
					})?,
					verdict: match (status, score) {
						(Some(Status::CompilationFailed), _) => Verdict::Rejected { cause: Some(RejectionCause::CompilationError), test: None },
						(Some(Status::Pending), _) => Verdict::Pending { test: None },
						(status, Some(score)) => Verdict::Scored {
							score: score as f64,
							max: None,
							cause: match status {
								Some(Status::WrongAnswer) => Some(RejectionCause::WrongAnswer),
								Some(Status::TimeLimitExceeded) => Some(RejectionCause::TimeLimitExceeded),
								Some(Status::MemoryLimitExceeded) => Some(RejectionCause::MemoryLimitExceeded),
								Some(Status::RuntimeError) => Some(RejectionCause::RuntimeError),
								_ => None,
							},
							test: None,
						},
						(_, None) => Verdict::Pending { test: None },
					},
				})
			})
			.collect::<Result<_>>()?)
	}

	fn submit(&self, language: &Language, code: &str) -> Result<String> {
		let url: Url = format!("{}/c/{}/submit/", self.contest.session.site, self.contest.id).parse().unwrap();
		let mut resp = self.contest.session.client.get(url.clone()).send()?;
		let doc = debris::Document::new(&resp.text()?);
		let problem_instance_id = doc
			.find("#id_problem_instance_id")?
			.find_all("option")
			.filter(|opt| opt.attr("selected").is_err())
			.map(|opt| {
				Ok((
					opt.attr("value")?.string(),
					opt.text().map(|joint| {
						let i1 = joint.rfind('(').ok_or("'(' not found in submittable title")?;
						let i2 = joint.rfind(')').ok_or("')' not found in submittable title")?;
						std::result::Result::<_, &'static str>::Ok(joint[i1 + 1..i2].to_owned())
					})?,
				))
			})
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.find(|(_, symbol)| *symbol == self.id)
			.ok_or(Error::WrongData)?
			.0;
		let csrf = doc.find_first("input[name=\"csrfmiddlewaretoken\"]")?.attr("value")?.string();
		let is_admin = doc.find("#id_kind").is_ok();
		let mut form = reqwest::multipart::Form::new()
			.text("csrfmiddlewaretoken", csrf)
			.text("problem_instance_id", problem_instance_id)
			.text("code", code.to_owned())
			.text("prog_lang", language.id.to_owned());
		if is_admin {
			form = form
				.text("user", self.contest.session.username.lock().unwrap().as_ref().ok_or(Error::AccessDenied)?.to_owned())
				.text("kind", "IGNORED");
		}
		self.contest.session.client.post(url.clone()).header(REFERER, url.to_string()).multipart(form).send()?;
		Ok(self.submissions()?[0].id.to_string())
	}
}

enum Status {
	Accepted,
	WrongAnswer,
	TimeLimitExceeded,
	MemoryLimitExceeded,
	RuntimeError,
	CompilationFailed,
	Pending,
}
