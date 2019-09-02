#![feature(never_type)]

use unijudge::{
	debris::{self, Context, Find}, reqwest::{
		self, cookie_store::Cookie, header::{ORIGIN, REFERER}, multipart, Url
	}, ContestDetails, Error, Language, RejectionCause, Resource, Result, Submission, TaskDetails, Verdict
};

pub struct SPOJ;

impl unijudge::Backend for SPOJ {
	type CachedAuth = [Cookie<'static>; 3];
	type Contest = !;
	type Session = reqwest::Client;
	type Task = String;

	fn accepted_domains(&self) -> &'static [&'static str] {
		&["www.spoj.com"]
	}

	fn deconstruct_resource(&self, _domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>> {
		match segments {
			["problems", task] => Ok(Resource::Task((*task).to_owned())),
			_ => Err(Error::WrongTaskUrl),
		}
	}

	fn connect(&self, client: reqwest::Client, _domain: &str) -> Self::Session {
		client
	}

	fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let cookies = session.cookies().read().map_err(|_| Error::StateCorruption)?;
		let spoj = match cookies.0.get("spoj.com", "/", "SPOJ") {
			Some(c) => c.clone().into_owned(),
			None => return Ok(None),
		};
		let login = match cookies.0.get("spoj.com", "/", "autologin_login") {
			Some(c) => c.clone().into_owned(),
			None => return Ok(None),
		};
		let hash = match cookies.0.get("spoj.com", "/", "autologin_hash") {
			Some(c) => c.clone().into_owned(),
			None => return Ok(None),
		};
		Ok(Some([spoj, login, hash]))
	}

	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		unijudge::deserialize_auth(data)
	}

	fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let mut resp = session
			.post("https://www.spoj.com/login/")
			.header(ORIGIN, "https://www.spoj.com")
			.header(REFERER, "https://www.spoj.com/")
			.form(&[("next_raw", "/"), ("autologin", "1"), ("login_user", username), ("password", password)])
			.send()?;
		let doc = debris::Document::new(&resp.text()?);
		if resp.url().as_str() == "https://www.spoj.com/login/" {
			Err(Error::WrongCredentials)
		} else if resp.url().as_str() == "https://www.spoj.com/" {
			Ok(())
		} else {
			Err(Error::UnexpectedHTML(doc.error("unrecognized login outcome")))
		}
	}

	fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		let url = "https://www.spoj.com/".parse()?;
		let [c1, c2, c3] = auth;
		let mut cookies = session.cookies().write().map_err(|_| Error::StateCorruption)?;
		cookies.0.insert(c2.clone(), &url).map_err(|_| Error::WrongData)?;
		cookies.0.insert(c3.clone(), &url).map_err(|_| Error::WrongData)?;
		cookies.0.insert(c1.clone(), &url).map_err(|_| Error::WrongData)?;
		Ok(())
	}

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		unijudge::serialize_auth(auth)
	}

	fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		let url: Url = format!("https://www.spoj.com/problems/{}/", task).parse()?;
		let mut resp = session.get(url.clone()).send()?;
		let doc = debris::Document::new(&resp.text()?);
		let title = doc.find(".breadcrumb > .active")?.text().string();
		let mut statement = unijudge::statement::Rewrite::start(doc);
		statement.fix_hide(|v| {
			if let unijudge::scraper::Node::Element(v) = v.value() {
				v.id().map_or(false, |id| ["problem-name", "problem-tags", "problem-body"].contains(&id))
			} else {
				false
			}
		});
		statement.fix_override_csp();
		statement.fix_traverse(|mut v| {
			if let unijudge::scraper::Node::Element(v) = v.value() {
				unijudge::statement::fix_url(v, unijudge::qn!("href"), "//", "https:");
				unijudge::statement::fix_url(v, unijudge::qn!("href"), "/", "https://www.spoj.com");
				if v.name() == "body" {
					unijudge::statement::add_style(v, "background: #fff;");
				}
				if v.id() == Some("content") {
					unijudge::statement::add_style(v, "border: none;");
				}
			}
		});
		Ok(TaskDetails {
			id: task.to_owned(),
			title,
			contest_id: "problems".to_owned(),
			site_short: "spoj".to_owned(),
			examples: None,
			statement: Some(statement.export()),
			url: url.to_string(),
		})
	}

	fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		let url: Url = format!("https://www.spoj.com/submit/{}/", task).parse()?;
		let mut resp = session.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		doc.find_all("#lang > option")
			.map(|node| Ok(Language { id: node.attr("value")?.string(), name: node.text().string() }))
			.collect::<Result<_>>()
	}

	fn task_submissions(&self, session: &Self::Session, _task: &Self::Task) -> Result<Vec<Submission>> {
		let user = req_user(session)?;
		let url: Url = format!("https://www.spoj.com/status/{}/", user).parse()?;
		let mut resp = session.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		Ok(doc
			.find_all("table.newstatus > tbody > tr")
			.map(|row| {
				Ok(unijudge::Submission {
					id: row.child(1)?.text().string(),
					verdict: row.find(".statusres")?.text().map(|text| {
						let part = &text[..text.find('\n').unwrap_or_else(|| text.len())];
						match part {
							"accepted" => Ok(Verdict::Accepted),
							"wrong answer" => Ok(Verdict::Rejected { cause: Some(RejectionCause::WrongAnswer), test: None }),
							"time limit exceeded" => Ok(Verdict::Rejected { cause: Some(RejectionCause::TimeLimitExceeded), test: None }),
							"compilation error" => Ok(Verdict::Rejected { cause: Some(RejectionCause::CompilationError), test: None }),
							"runtime error    (SIGFPE)" | "runtime error    (SIGSEGV)" | "runtime error    (SIGABRT)" | "runtime error    (NZEC)" => {
								Ok(Verdict::Rejected { cause: Some(RejectionCause::RuntimeError), test: None })
							},
							"internal error" => Ok(Verdict::Rejected { cause: Some(RejectionCause::SystemError), test: None }),
							"waiting.." => Ok(Verdict::Pending { test: None }),
							"compiling.." => Ok(Verdict::Pending { test: None }),
							"running judge.." => Ok(Verdict::Pending { test: None }),
							"running.." => Ok(Verdict::Pending { test: None }),
							_ => part
								.parse::<f64>()
								.map(|score| Verdict::Scored { score, max: None, cause: None, test: None })
								.map_err(|_| Err::<Verdict, String>(format!("unrecognized SPOJ verdict {:?}", part))),
						}
					})?,
				})
			})
			.collect::<Result<_>>()?)
	}

	fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		let mut resp = session
			.post("https://www.spoj.com/submit/complete/")
			.multipart(
				multipart::Form::new()
					.part(
						"subm_file",
						multipart::Part::bytes(Vec::new()).file_name("").mime_str("application/octet-stream").map_err(|_| Error::WrongData)?,
					)
					.text("file", code.to_owned())
					.text("lang", language.id.to_owned())
					.text("problemcode", task.to_owned())
					.text("submit", "Submit!"),
			)
			.header(ORIGIN, "https://www.spoj.com")
			.header(REFERER, "https://www.spoj.com/submit/TEST/")
			.send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		if doc.find("title")?.text().string().contains("Authorisation required") {
			return Err(Error::AccessDenied);
		}
		Ok(doc.find("#content > input")?.attr("value")?.string())
	}

	fn task_url(&self, _sess: &Self::Session, task: &Self::Task) -> Result<String> {
		Ok(format!("https://www.spoj.com/problems/{}/", task))
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		*contest
	}

	fn contest_site_prefix(&self) -> &'static str {
		unimplemented!()
	}

	fn contest_tasks(&self, _session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		*contest
	}

	fn contest_url(&self, contest: &Self::Contest) -> String {
		*contest
	}

	fn contests(&self, _session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		Ok(Vec::new())
	}

	fn name_short(&self) -> &'static str {
		"spoj"
	}

	fn supports_contests(&self) -> bool {
		false
	}
}

fn req_user(session: &reqwest::Client) -> Result<String> {
	Ok(session
		.cookies()
		.read()
		.map_err(|_| Error::StateCorruption)?
		.0
		.get("spoj.com", "/", "autologin_login")
		.ok_or(Error::AccessDenied)?
		.value()
		.to_owned())
}
