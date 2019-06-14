use unijudge::{
	debris::Find, reqwest::{
		self, header::{ORIGIN, REFERER}, multipart, Url
	}, Error, Language, RejectionCause, Result, TaskDetails, TaskUrl, Verdict
};

pub struct SPOJ;

struct Session {
	client: reqwest::Client,
}
struct Task<'s> {
	id: String,
	session: &'s Session,
}

impl unijudge::Backend for SPOJ {
	fn deconstruct_url(&self, url: &str) -> Result<Option<TaskUrl>> {
		let url: Url = match url.parse() {
			Ok(url) => url,
			Err(_) => return Ok(None),
		};
		let segs: Vec<_> = url.path_segments().map_or(Vec::new(), |segs| segs.filter(|seg| !seg.is_empty()).collect());
		if url.domain() != Some("www.spoj.com") {
			return Ok(None);
		}
		let task = match segs.as_slice() {
			["problems", task] => format!("{}", task),
			_ => return Err(Error::WrongTaskUrl),
		};
		Ok(Some(TaskUrl {
			site: "https://www.spoj.com".to_owned(),
			contest: String::new(),
			task,
		}))
	}

	fn connect<'s>(&'s self, _site: &str) -> Result<Box<dyn unijudge::Session+'s>> {
		Ok(Box::new(Session {
			client: reqwest::Client::builder().cookie_store(true).build().map_err(Error::TLSFailure)?,
		}))
	}
}

impl unijudge::Session for Session {
	fn login(&self, username: &str, password: &str) -> Result<()> {
		self.client
			.post("https://www.spoj.com/login/")
			.header(ORIGIN, "https://www.spoj.com")
			.header(REFERER, "https://www.spoj.com/")
			.form(&[("next_raw", "/"), ("autologin", "1"), ("login_user", username), ("password", password)])
			.send()?;
		Ok(())
	}

	fn restore_auth(&self, id: &str) -> Result<()> {
		let cached: CachedAuth = serde_json::from_str(id).map_err(|_| Error::WrongData)?;
		let mut cookies = self.client.cookies().write().unwrap();
		let url = "https://www.spoj.com/".parse().unwrap();
		cookies.0.insert(cached.spoj, &url).unwrap();
		cookies.0.insert(cached.login, &url).unwrap();
		cookies.0.insert(cached.hash, &url).unwrap();
		Ok(())
	}

	fn cache_auth(&self) -> Result<Option<String>> {
		let cookies = self.client.cookies().read().unwrap();
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
		let cached = CachedAuth { spoj, login, hash };
		let encoded = serde_json::to_string(&cached).unwrap();
		Ok(Some(encoded))
	}

	fn contest<'s>(&'s self, _id: &str) -> Result<Box<dyn unijudge::Contest+'s>> {
		Ok(Box::new(self))
	}
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CachedAuth {
	spoj: unijudge::reqwest::cookie_store::Cookie<'static>,
	login: unijudge::reqwest::cookie_store::Cookie<'static>,
	hash: unijudge::reqwest::cookie_store::Cookie<'static>,
}

impl unijudge::Contest for &Session {
	fn task<'s>(&'s self, id: &str) -> Result<Box<dyn unijudge::Task+'s>> {
		Ok(Box::new(Task { id: id.to_owned(), session: self }))
	}
}

impl unijudge::Task for Task<'_> {
	fn details(&self) -> Result<TaskDetails> {
		let url = Url::parse(&format!("https://www.spoj.com/problems/{}/", self.id)).unwrap();
		let mut resp = self.session.client.get(url).send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		let title = doc.find(".breadcrumb > .active")?.text().string();
		Ok(TaskDetails {
			symbol: self.id.clone(),
			title,
			examples: None,
		})
	}

	fn languages(&self) -> Result<Vec<Language>> {
		let url = Url::parse(&format!("https://www.spoj.com/submit/{}/", self.id)).unwrap();
		let mut resp = self.session.client.get(url).send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		doc.find_all("#lang > option")
			.map(|node| {
				Ok(Language {
					id: node.attr("value")?.string(),
					name: node.text().string(),
				})
			})
			.collect::<Result<_>>()
	}

	fn submissions(&self) -> Result<Vec<unijudge::Submission>> {
		let user = self
			.session
			.client
			.cookies()
			.read()
			.unwrap()
			.0
			.get("spoj.com", "/", "autologin_login")
			.ok_or(Error::AccessDenied)?
			.value()
			.to_owned();
		let url = Url::parse(&format!("https://www.spoj.com/status/{}/", user)).unwrap();
		let mut resp = self.session.client.get(url).send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		Ok(doc
			.find_all("table.newstatus > tbody > tr")
			.map(|row| {
				Ok(unijudge::Submission {
					id: row.child(1)?.text().string(),
					verdict: row.find(".statusres")?.text().map(|text| {
						let part = &text[..text.find("\n").unwrap_or(text.len())];
						match part {
							"accepted" => Ok(Verdict::Accepted),
							"wrong answer" => Ok(Verdict::Rejected {
								cause: Some(RejectionCause::WrongAnswer),
								test: None,
							}),
							"time limit exceeded" => Ok(Verdict::Rejected {
								cause: Some(RejectionCause::TimeLimitExceeded),
								test: None,
							}),
							"compilation error" => Ok(Verdict::Rejected {
								cause: Some(RejectionCause::CompilationError),
								test: None,
							}),
							"runtime error    (SIGFPE)" | "runtime error    (SIGSEGV)" | "runtime error    (SIGABRT)" | "runtime error    (NZEC)" => Ok(Verdict::Rejected {
								cause: Some(RejectionCause::RuntimeError),
								test: None,
							}),
							"internal error" => Ok(Verdict::Rejected {
								cause: Some(RejectionCause::SystemError),
								test: None,
							}),
							"waiting.." => Ok(Verdict::Pending { test: None }),
							"compiling.." => Ok(Verdict::Pending { test: None }),
							"running judge.." => Ok(Verdict::Pending { test: None }),
							"running.." => Ok(Verdict::Pending { test: None }),
							_ => {
								if let Ok(score) = part.parse::<f64>() {
									Ok(Verdict::Scored {
										score,
										max: None,
										cause: None,
										test: None,
									})
								} else {
									Err(format!("unrecognized SPOJ verdict {:?}", part))
								}
							},
						}
					})?,
				})
			})
			.collect::<Result<_>>()?)
	}

	fn submit(&self, language: &Language, code: &str) -> Result<String> {
		let mut resp = self
			.session
			.client
			.post("https://www.spoj.com/submit/complete/")
			.multipart(
				multipart::Form::new()
					.part("subm_file", multipart::Part::bytes(Vec::new()).file_name("").mime_str("application/octet-stream").unwrap())
					.text("file", code.to_owned())
					.text("lang", language.id.to_owned())
					.text("problemcode", self.id.to_owned())
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
}
