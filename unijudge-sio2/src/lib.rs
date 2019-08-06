#![feature(never_type, slice_patterns)]

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use unijudge::{
	debris::{self, Context, Find}, reqwest::{self, cookie_store::Cookie, header::REFERER, Url}, ContestDetails, Error, Language, RejectionCause, Resource, Result, Submission, TaskDetails, Verdict
};

pub struct Sio2;

pub struct Session {
	client: reqwest::Client,
	site: String,
	username: Mutex<Option<String>>,
}

#[derive(Debug)]
pub struct Task {
	contest: String,
	task: String,
}

#[derive(Serialize, Deserialize)]
pub struct CachedAuth {
	username: String,
	sessionid: Cookie<'static>,
}

impl unijudge::Backend for Sio2 {
	type CachedAuth = CachedAuth;
	type Contest = !;
	type Session = Session;
	type Task = Task;

	const SUPPORTS_CONTESTS: bool = false;

	fn accepted_domains(&self) -> &'static [&'static str] {
		&["kiwi.ii.uni.wroc.pl", "main2.edu.pl", "sio2.mimuw.edu.pl", "sio2.staszic.waw.pl", "szkopul.edu.pl"]
	}

	fn deconstruct_resource(&self, _domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>> {
		let (contest, task) = match segments {
			["c", contest, "p", task] => (contest, task),
			["c", contest, "p", task, ..] => (contest, task),
			_ => return Err(Error::WrongTaskUrl),
		};
		Ok(Resource::Task(Task { contest: (*contest).to_owned(), task: (*task).to_owned() }))
	}

	fn connect(&self, client: reqwest::Client, domain: &str) -> Self::Session {
		Session { client, site: format!("https://{}", domain), username: Mutex::new(None) }
	}

	fn login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let url1: Url = format!("{}/login/", session.site).parse().unwrap();
		let mut resp1 = session.client.get(url1).send()?;
		let url2 = resp1.url().clone();
		let doc1 = debris::Document::new(&resp1.text()?);
		let csrf = doc1.find("input[name=\"csrfmiddlewaretoken\"]")?.attr("value")?.string();
		let mut resp2 = session
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
			*session.username.lock().unwrap() = Some(username.to_owned());
			Ok(())
		} else if doc2.find("form")?.find("div.form-group > div > div.alert.alert-danger").is_ok() {
			Err(Error::WrongCredentials)
		} else {
			Err(Error::UnexpectedHTML(doc2.error("unrecognized login outcome")))
		}
	}

	fn restore_auth(&self, session: &Self::Session, auth: Self::CachedAuth) -> Result<()> {
		*session.username.lock().unwrap() = Some(auth.username);
		session.client.cookies().write().unwrap().0.insert(auth.sessionid, &session.site.parse().unwrap()).unwrap();
		Ok(())
	}

	fn cache_auth(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let username = match session.username.lock().unwrap().as_ref() {
			Some(username) => username.to_owned(),
			None => return Ok(None),
		};
		let sessionid = match session.client.cookies().read().unwrap().0.get(Url::parse(&session.site).unwrap().domain().unwrap(), "/", "sessionid") {
			Some(c) => c.clone().into_owned(),
			None => return Ok(None),
		};
		Ok(Some(CachedAuth { username, sessionid }))
	}

	fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		let url: Url = format!("{}/c/{}/p/", session.site, task.contest).parse().unwrap();
		let mut resp = session.client.get(url.clone()).send()?;
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
		let title = match problems.into_iter().find(|(id, _)| id == &task.task) {
			Some((_, title)) => title,
			None => return Err(Error::WrongData),
		};
		Ok(TaskDetails {
			id: task.task.clone(),
			title,
			contest_id: task.contest.clone(),
			site_short: "sio2".to_owned(),
			examples: None,
			statement: None,
			url: url.to_string(),
		})
	}

	fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		let url: Url = format!("{}/c/{}/submit/", session.site, task.contest).parse().unwrap();
		let mut resp = session.client.get(url).send()?;
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

	fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>> {
		let url: Url = format!("{}/c/{}/submissions/", session.site, task.contest).parse().unwrap();
		let mut resp = session.client.get(url).send()?;
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
				let score = tr.child(11)?.text().map(|score| score[..score.find(' ').unwrap_or_else(|| score.len())].parse::<i64>()).ok();
				Ok(Submission {
					id: tr.find("a")?.attr("href")?.map(|href| match href.split('/').filter(|seg| !seg.is_empty()).collect::<Vec<_>>().last() {
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

	fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		let url: Url = format!("{}/c/{}/submit/", session.site, task.contest).parse().unwrap();
		let mut resp = session.client.get(url.clone()).send()?;
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
			.find(|(_, symbol)| *symbol == task.task)
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
			form = form.text("user", session.username.lock().unwrap().as_ref().ok_or(Error::AccessDenied)?.to_owned()).text("kind", "IGNORED");
		}
		session.client.post(url.clone()).header(REFERER, url.to_string()).multipart(form).send()?;
		Ok(self.task_submissions(session, task)?[0].id.to_string())
	}

	fn contests(&self, _session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		Ok(Vec::new())
	}

	fn contest_tasks(&self, _session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		*contest
	}

	fn site_short(&self) -> &'static str {
		"sio2"
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		*contest
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
