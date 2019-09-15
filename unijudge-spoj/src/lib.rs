#![feature(never_type, try_blocks)]

use async_trait::async_trait;
use unijudge::{
	self, debris::{self, Context, Find}, http::{Client, Cookie}, reqwest::{
		header::{ORIGIN, REFERER}, multipart
	}, url::Url, ContestDetails, Error, Language, RejectionCause, Resource, Result, Submission, TaskDetails, Verdict
};

pub struct SPOJ;

#[async_trait]
impl unijudge::Backend for SPOJ {
	type CachedAuth = [Cookie; 3];
	type Contest = !;
	type Session = Client;
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

	fn connect(&self, client: Client, _domain: &str) -> Self::Session {
		client
	}

	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let spoj = session.cookie_get("SPOJ")?;
		let autologin_login = session.cookie_get("autologin_login")?;
		let autologin_hash = session.cookie_get("autologin_hash")?;
		Ok(try { [spoj?, autologin_login?, autologin_hash?] })
	}

	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		unijudge::deserialize_auth(data)
	}

	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let resp = session
			.post("https://www.spoj.com/login/".parse()?)
			.header(ORIGIN, "https://www.spoj.com")
			.header(REFERER, "https://www.spoj.com/")
			.form(&[("next_raw", "/"), ("autologin", "1"), ("login_user", username), ("password", password)])
			.send()
			.await?;
		let url = resp.url().clone();
		let doc = debris::Document::new(&resp.text().await?);
		if url.as_str() == "https://www.spoj.com/login/" {
			Err(Error::WrongCredentials)
		} else if url.as_str() == "https://www.spoj.com/" {
			Ok(())
		} else {
			Err(Error::UnexpectedHTML(doc.error("unrecognized login outcome")))
		}
	}

	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		let [c1, c2, c3] = auth;
		session.cookie_set(c1.clone(), "https://www.spoj.com/")?;
		session.cookie_set(c2.clone(), "https://www.spoj.com/")?;
		session.cookie_set(c3.clone(), "https://www.spoj.com/")?;
		Ok(())
	}

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		unijudge::serialize_auth(auth)
	}

	fn task_contest(&self, _: &Self::Task) -> Option<Self::Contest> {
		None
	}

	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		let url: Url = format!("https://www.spoj.com/problems/{}/", task).parse()?;
		let resp = session.get(url.clone()).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
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

	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		let url: Url = format!("https://www.spoj.com/submit/{}/", task).parse()?;
		let resp = session.get(url).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
		doc.find_all("#lang > option")
			.map(|node| Ok(Language { id: node.attr("value")?.string(), name: node.text().string() }))
			.collect::<Result<_>>()
	}

	async fn task_submissions(&self, session: &Self::Session, _task: &Self::Task) -> Result<Vec<Submission>> {
		let user = req_user(session)?;
		let url: Url = format!("https://www.spoj.com/status/{}/", user).parse()?;
		let resp = session.get(url).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
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

	async fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		let resp = session
			.post("https://www.spoj.com/submit/complete/".parse()?)
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
			.send()
			.await?;
		let doc = unijudge::debris::Document::new(&resp.text().await?);
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

	async fn contest_tasks(&self, _session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		*contest
	}

	fn contest_url(&self, contest: &Self::Contest) -> String {
		*contest
	}

	async fn contest_title(&self, _session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		*contest
	}

	async fn contests(&self, _session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		Ok(Vec::new())
	}

	fn name_short(&self) -> &'static str {
		"spoj"
	}

	fn supports_contests(&self) -> bool {
		false
	}
}

fn req_user(session: &Client) -> Result<String> {
	Ok(session.cookie_get("autologin_login")?.ok_or(Error::AccessDenied)?.value().to_owned())
}
