#![feature(try_blocks)]
use async_trait::async_trait;
use unijudge::{
    Problem,
	chrono::{prelude::*,Duration,FixedOffset, TimeZone}, debris::{self, Context, Document, Find}, http::{Client, Cookie}, reqwest::{
		header::{ORIGIN, REFERER}, StatusCode, Url
	}, ContestDetails, ContestTime, Error, ErrorCode, Example, Language, RejectionCause, Resource, Result, Submission, TaskDetails, Verdict,json
};
use std::{future::Future, pin::Pin, sync::Mutex};
use unijudge::serde::{Serialize,Deserialize, Deserializer};
#[derive(Debug)]
pub struct AtCoder;

#[derive(Debug)]
pub struct Task {
	contest: String,
	task: String,
}
#[derive(Debug)]
pub struct Session {
	client: Client,
	username: Mutex<Option<String>>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedAuth {
	username: String,
	c_sess: Cookie,
}
impl Session {
	fn req_user(&self) -> Result<String> {
		let username = self.username.lock()?.clone().ok_or(ErrorCode::AccessDenied)?;
		Ok(username)
	}
}
#[async_trait(?Send)]
impl unijudge::Backend for AtCoder {
	type CachedAuth = CachedAuth;
	type Contest = String;
	type Session = Session;
	type Task = Task;

	fn accepted_domains(&self) -> &'static [&'static str] {
		&["atcoder.jp"]
	}

	fn deconstruct_resource(&self, _domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>> {
		match segments {
			["contests", contest] => Ok(Resource::Contest((*contest).to_owned())),
			["contests", contest, "tasks"] => Ok(Resource::Contest((*contest).to_owned())),
			["contests", contest, "tasks", task] => {
				Ok(Resource::Task(Task { contest: (*contest).to_owned(), task: (*task).to_owned() }))
			},
			_ => Err(ErrorCode::WrongTaskUrl.into()),
		}
	}

	fn connect(&self, client: Client, _domain: &str) -> Self::Session {
		Session { client, username: Mutex::new(None) }
	}

	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let username = session.username.lock()?.clone();
		let c_sess = session.client.cookie_get_if(|c| c.starts_with("REVEL_SESSION"))?;
		Ok(try { CachedAuth { username: username?, c_sess: c_sess? } })
	}

	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		unijudge::deserialize_auth(data)
	}

	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let csrf = self.fetch_login_csrf(session).await?;
		let url: Url = "https://atcoder.jp/login".parse()?;
		let resp = match session.client
			.post(url)
			.header(ORIGIN, "https://atcoder.jp")
			.header(REFERER, "https://atcoder.jp/login")
			.form(&[("username", username), ("password", password), ("csrf_token", &csrf)])
			.send()
			.await
		{
			Ok(resp) => resp,
			// this is the worst way to indicate wrong password I have heard of
			Err(ref e) if e.to_string().contains("redirect loop") => {
				return Err(ErrorCode::WrongCredentials.into());
			},
			Err(e) => return Err(e.into()),
		};
		let doc = debris::Document::new(&resp.text().await?);
		if doc.find("#main-container > div.row > div.alert.alert-success").is_ok() {
			*session.username.lock()? = Some(username.to_owned());
			Ok(())
		} else if doc.find("#main-container > div.row > div.alert.alert-danger").is_ok() {
			Err(ErrorCode::WrongCredentials.into())
		} else {
			Err(doc.error("unrecognized login outcome").into())
		}
	}

	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		session.client.cookie_set(auth.c_sess.clone(), "https://atcoder.jp/")?;
		*session.username.lock()? = Some(auth.username.clone());
		Ok(())
	}

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		unijudge::serialize_auth(auth)
	}

	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest> {
		Some(task.contest.clone())
	}

	async fn remain_time(&self, session: &Self::Session, task: &Self::Task) -> Result<i64>{
		let resp = session.client.get(format!("https://atcoder.jp/contests/{}",task.contest).parse()?).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
		
		let end_time = doc.find(".contest-duration")?.find_nth("a", 1)?.attr("href")?.map(|href| {
			let japan_standard_time = FixedOffset::east(9 * 3600);
			japan_standard_time.datetime_from_str(
				href,
				"http://www.timeanddate.com/worldclock/fixedtime.html?iso=%Y%m%dT%H%M&p1=248",
			)
		})?;
		let start = doc.find(".contest-duration")?.find_nth("a", 0)?.attr("href")?.map(|href| {
			let japan_standard_time = FixedOffset::east(9 * 3600);
			japan_standard_time.datetime_from_str(
				href,
				"http://www.timeanddate.com/worldclock/fixedtime.html?iso=%Y%m%dT%H%M&p1=248",
			)
		})?;
		let today: DateTime<Utc> = Utc::now();
		let diff = end_time.signed_duration_since(today);
		if diff.num_seconds()<0 {
			return Err(ErrorCode::Ended_Already.into());
		}
		if start.signed_duration_since(today).num_seconds()>0{
			return Err(ErrorCode::NotYetStarted.into());
		}
		let secs = diff.num_seconds();
		return Ok(secs);
	}
	
	async fn rank_list(&self, session: &Self::Session, task: &Self::Task) -> Result<String>{
		session.req_user()?;
		let url: Url = format!("https://atcoder.jp/contests/{}/standings/json", task.contest).parse()?;
		let resp = session.client.get(url.clone()).send().await?.text().await?;
		let resp_raw = json::from_str::<Standings>(&resp)?;
		let result:Vec<_>=resp_raw.StandingsData.iter().filter(|user|{
			user.UserName==session.req_user().unwrap()
		}).map(|user|{
			"Rank: ".to_owned()+&user.Rank.to_string()+" , Solved: "+&user.TotalResult.Accepted.to_string()+" , Score: "+&(user.TotalResult.Score/100).to_string()
		}).collect();
		if result.len()==0 {
			return Ok("User not found".to_string());
		}
		return Ok(result[0].to_string())
	}

    async fn problems_list(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Problem>>{
		return Ok(Vec::new());
	}
    
	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		session.req_user()?;
		let url: Url = format!("https://atcoder.jp/contests/{}/tasks/{}", task.contest, task.task).parse()?;
		let resp = session.client.get(url.clone()).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
		let (symbol, title) = doc.find("#main-container > .row > div > span.h2")?.text().map(|text| {
			let mark = text.find('-').ok_or("no dash(-) found in task title")?;
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
		let mut statement = unijudge::statement::Rewrite::start(doc);
		statement.fix_hide(|v| {
			if let unijudge::scraper::Node::Element(v) = v.value() {
				if v.name() == "form" || v.attr("id") == Some("task-statement") {
					return false;
				}
				if v.has_class("lang-en", unijudge::selectors::attr::CaseSensitivity::CaseSensitive) {
					return true;
				}
			}
			unijudge::statement::any_sibling(v, |u| {
				if let unijudge::scraper::Node::Element(u) = u.value() {
					u.attr("id") == Some("task-statement")
				} else {
					false
				}
			})
		});
		statement.fix_override_csp();
		statement.fix_traverse(|mut v| {
			if let unijudge::scraper::Node::Element(v) = v.value() {
				let is_css = v.name() == "link";
				//		&& v.attr("href");
				//		.map_or(false, |href| href.contains("contests.css") || href.contains("bootstrap.min.css") || href.contains("katex.min.css"));
				if is_css {
					unijudge::statement::fix_url(v, unijudge::qn!("href"), "//", "https:");
					unijudge::statement::fix_url(v, unijudge::qn!("href"), "/", "https://atcoder.jp");
				}
				if v.name()=="img" ||  v.name()=="a" {
					unijudge::statement::fix_url(v, unijudge::qn!("src"), "//", "https:");
					unijudge::statement::fix_url(v, unijudge::qn!("src"), "/", "https://atcoder.jp");
				}
				//if v.name() == "script" && v.attr("src").map_or(false, |src| src.contains("katex.min.js") || src.contains("auto-render.min.js")) {
				if v.name() == "script" {
					unijudge::statement::fix_url(v, unijudge::qn!("src"), "//", "https:");
				}
			}
			let is_tex = if let unijudge::scraper::Node::Element(v) = v.value() { v.name() == "var" } else { false };
			if is_tex {
				if let Some(mut u) = v.first_child() {
					if let unijudge::scraper::Node::Text(text) = u.value() {
						text.text = format!("{}", text.text).into();
					}
				}
			}
		});
		Ok(TaskDetails {
			id: symbol.clone(),
			title: symbol.clone()+"_"+&title,
			contest_id: task.contest.clone(),
			site_short: "atcoder".to_owned(),
			examples,
			statement: Some(statement.export()),
			url: url.to_string(),
		})
	}

	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		let url: Url = format!("https://atcoder.jp/contests/{}/submit", task.contest).parse()?;
		let resp = session.client.get(url).send().await?;
		if resp.url().path() == "/login" {
			return Err(ErrorCode::AccessDenied.into());
		}
		let doc = debris::Document::new(&resp.text().await?);
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
			.filter(|option| !option.text().as_str().is_empty())
			.map(|opt| Ok(Language { id: opt.attr("value")?.string(), name: opt.text().string() }))
			.collect::<Result<_>>()?)
	}

	async fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>> {
		session.req_user()?;
		let url: Url = format!("https://atcoder.jp/contests/{}/submissions/me", task.contest).parse()?;
		let resp = session.client.get(url).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
		Ok(doc
			.find_all(".panel-submission tbody > tr")
			.map(|row| {
				let id = row.find(".submission-score")?.attr("data-id")?.string();
				let status = row.find("td > span")?;
				let status = status.text();
				let (test_index, verdict) = match status.as_str().find(' ') {
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
							"IE" => Some(RejectionCause::SystemError),
							_ => {
								return Err(status.error(format!(
									"unrecognized AtCoder verdict {:?} [{:?} {:?}]",
									status.as_str(),
									verdict,
									test_index
								)));
							},
						},
						test: None,
					},
					(None, None) => {
						return Err(status.error(format!(
							"unrecognized AtCoder verdict {:?} [{:?} {:?}]",
							status.as_str(),
							verdict,
							test_index
						)));
					},
				};
				Ok(Submission { id, verdict })
			})
			.collect::<debris::Result<_>>()?)
	}

	async fn task_submit(
		&self,
		session: &Self::Session,
		task: &Self::Task,
		language: &Language,
		code: &str,
	) -> Result<String> {
		session.req_user()?;
		let csrf = self.fetch_login_csrf(session).await?;
		let url: Url = format!("https://atcoder.jp/contests/{}/submit", task.contest).parse()?;
		session.client
			.post(url)
			.form(&[
				("data.TaskScreenName", &task.task),
				("data.LanguageId", &language.id),
				("sourceCode", &String::from(code)),
				("csrf_token", &csrf),
			])
			.send()
			.await?;
		Ok(self.task_submissions(session, task).await?[0].id.to_string())
	}

	fn task_url(&self, _sess: &Self::Session, task: &Self::Task) -> Result<String> {
		Ok(format!("https://atcoder.jp/contests/{}/tasks/{}", task.contest, task.task))
	}

	fn submission_url(&self, _sess: &Self::Session, task: &Self::Task, id: &str) -> String {
		format!("{}/submissions/{}", self.contest_url(&task.contest), id)
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		contest.clone()
	}

	fn contest_site_prefix(&self) -> &'static str {
		"AtCoder"
	}

	async fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		session.req_user()?;
		let resp = session.client.get(format!("https://atcoder.jp/contests/{}/tasks", contest).parse()?).send().await?;
		let status = resp.status();
		let doc = debris::Document::new(&resp.text().await?);
		if status == StatusCode::NOT_FOUND {
			let alert = doc.find(".alert.alert-danger")?.text().string();
			if alert.ends_with("Contest not found.") {
				return Err(ErrorCode::MalformedData.into());
			} else if alert.ends_with("Permission denied.") {
                return Err(ErrorCode::AccessDenied.into());
				//return Err(ErrorCode::NotYetStarted.into());
			} else {
				return Err(doc.error("unrecognized alert message").into());
			}
		}
		doc.find("table")?
			.find_all("tbody > tr")
			.map(|row| {
				Ok(row.find_nth("td", 1)?.find("a")?.attr("href")?.map(|href| {
					match href.split('/').collect::<Vec<_>>().as_slice() {
						["", "contests", contest, "tasks", task] => {
							Ok(Task { contest: (*contest).to_owned(), task: (*task).to_owned() })
						},
						_ => Err(format!("invalid task url {:?}", href)),
					}
				})?)
			})
			.collect()
	}

	fn contest_url(&self, contest: &Self::Contest) -> String {
		format!("https://atcoder.jp/contests/{}", contest)
	}

	async fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		let url: Url = self.contest_url(contest).parse()?;
		let doc = Document::new(&session.client.get(url).send().await?.text().await?);
		Ok(doc.find("#main-container > .row > div > div > h1")?.text().string())
	}

	async fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		let resp = session.client.get("https://atcoder.jp/contests/".parse()?).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
		let container = doc.find("#main-container > .row > div.col-lg-9.col-md-8")?;
		let headers = container.find_all("h3").map(|h3| h3.text().string()).collect::<Vec<_>>();
		let table_indices: &[usize] = match headers.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
			["Active Contests", "Permanent Contests", "Upcoming Contests", "Recent Contests"] => &[0, 2],
			["Active Contests", "Permanent Contests", "Recent Contests"] => &[0],
			["Permanent Contests", "Upcoming Contests", "Recent Contests"] => &[1],
			["Permanent Contests", "Recent Contests"] => &[],
			_ => {
				return Err(Error::from(container.error(format!("unrecognized header layout {:?}", headers))));
			},
		};
		let tables = table_indices
			.iter()
			.map(|index| container.find_nth("table", *index))
			.collect::<debris::Result<Vec<_>>>()?;
		tables
			.iter()
			.flat_map(|table| {
				table.find_all("tbody > tr").map(|row| {
					let id = row.find_nth("td", 1)?.find("a")?.attr("href")?.map(|href| {
						Ok::<_, &'static str>(href[href.rfind('/').ok_or("no '/' in /contests/{}")? + 1..].to_owned())
					})?;
					let title = row.find_nth("td", 1)?.find("a")?.text().string();
					let start = row.find_nth("td", 0)?.find("a")?.attr("href")?.map(|href| {
						let japan_standard_time = FixedOffset::east(9 * 3600);
						japan_standard_time.datetime_from_str(
							href,
							"http://www.timeanddate.com/worldclock/fixedtime.html?iso=%Y%m%dT%H%M&p1=248",
						)
					})?;
					let time = ContestTime::Upcoming { start };
					Ok(ContestDetails { id, title, time })
				})
			})
			.collect()
	}

	fn name_short(&self) -> &'static str {
		"atcoder"
	}

	fn supports_contests(&self) -> bool {
		true
	}
}

impl AtCoder {
	async fn fetch_login_csrf(&self, session: &Session) -> Result<String> {
		let url: Url = "https://atcoder.jp/login".parse()?;
		let resp = session.client.get(url).send().await?;
		let doc = debris::Document::new(&resp.text().await?);
		Ok(doc.find_first("[name=\"csrf_token\"]")?.attr("value")?.string())
	}
}
#[derive(Debug, Deserialize)]
pub struct Standings {
	pub StandingsData: Vec<User>,
}
#[derive(Debug, Deserialize)]
pub struct User {
	pub Rank: i64,
	pub TotalResult:TResult,
	pub UserName:String
}

#[derive(Debug, Deserialize)]
pub struct TResult {
	pub Accepted: i64,
	pub Score:i64,
	pub Count:i64,

}