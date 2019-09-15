#![feature(try_blocks)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Mutex};
use unijudge::{
	backtrace::Backtrace, debris::{Context, Document, Find}, http::{Client, Cookie}, json, reqwest::{StatusCode, Url}, ContestDetails, Error, Language, RejectionCause, Resource, Result, Statement, Submission, TaskDetails, Verdict
};

pub struct CodeChef;

#[derive(Debug, Clone)]
pub enum Contest {
	Practice,
	Normal(String),
}

#[derive(Debug)]
pub struct Task {
	contest: Contest,
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

#[async_trait]
impl unijudge::Backend for CodeChef {
	type CachedAuth = CachedAuth;
	type Contest = Contest;
	type Session = Session;
	type Task = Task;

	fn accepted_domains(&self) -> &'static [&'static str] {
		&["www.codechef.com"]
	}

	fn deconstruct_resource(&self, _domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>> {
		// There is no dedicated practice contest site, so we do not need to handle ["PRACTICE"].
		// This is the only place where PRACTICE doesn't work, it's treated as a normal contest everywhere else.
		match segments {
			["problems", task] => Ok(Resource::Task(Task { contest: Contest::Practice, task: (*task).to_owned() })),
			["PRACTICE", "problems", task] => Ok(Resource::Task(Task { contest: Contest::Practice, task: (*task).to_owned() })),
			[contest, "problems", task] => Ok(Resource::Task(Task { contest: Contest::Normal((*contest).to_owned()), task: (*task).to_owned() })),
			[contest] => Ok(Resource::Contest(Contest::Normal((*contest).to_owned()))),
			_ => Err(Error::WrongTaskUrl),
		}
	}

	fn connect(&self, client: Client, _domain: &str) -> Self::Session {
		Session { client, username: Mutex::new(None) }
	}

	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let username = session.username.lock().map_err(|_| Error::StateCorruption)?.clone();
		let c_sess = session.client.cookie_get_if(|c| c.starts_with("SESS"))?;
		Ok(try { CachedAuth { username: username?, c_sess: c_sess? } })
	}

	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		unijudge::deserialize_auth(data)
	}

	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let resp1 = session.client.get("https://www.codechef.com/".parse()?).send().await?;
		let form_build_id = Document::new(&resp1.text().await?).find("#new-login-form [name=form_build_id]")?.attr("value")?.string();
		let resp2 = session
			.client
			.post("https://www.codechef.com/".parse()?)
			.form(&[("name", username), ("pass", password), ("form_build_id", &form_build_id), ("form_id", "new_login_form"), ("op", "Login")])
			.send()
			.await?;
		let resp2_url = resp2.url().clone();
		let other_sessions = {
			let doc = Document::new(&resp2.text().await?);
			if doc.find("a[title=\"Edit Your Account\"]").is_ok() {
				if resp2_url.as_str() == "https://www.codechef.com/session/limit" {
					// CodeChef does not allow to have more than one session active at once.
					// When this happens, disconnect all the other sessions so that ICIE's one can proceed.
					// This can be irritating, but there is no other sensible way of doing this.
					Some(self.select_other_sessions(&doc)?)
				} else {
					None
				}
			} else if doc.html().contains("Sorry, unrecognized username or password.") {
				return Err(Error::WrongCredentials);
			} else {
				return Err(Error::UnexpectedHTML(doc.error("unrecognized login outcome")));
			}
		};
		*session.username.lock().map_err(|_| Error::StateCorruption)? = Some(username.to_owned());
		if let Some(other_sessions) = other_sessions {
			self.disconnect_other_sessions(session, other_sessions).await?;
		}
		Ok(())
	}

	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		*session.username.lock().map_err(|_| Error::StateCorruption)? = Some(auth.username.clone());
		session.client.cookie_set(auth.c_sess.clone(), "https://www.codechef.com")?;
		Ok(())
	}

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		unijudge::serialize_auth(auth)
	}

	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest> {
		Some(task.contest.clone())
	}

	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		let url: Url = format!("https://www.codechef.com/api/contests/{}/problems/{}", task.contest.as_virt_symbol(), task.task).parse()?;
		let resp = json::from_resp::<api::Task>(session.client.get(url.clone()).send().await?, "/api/contests/{}/problems/{}").await?;
		let statement = Some(self.prepare_statement(&resp.problem_name, resp.body));
		Ok(TaskDetails {
			id: task.task.clone(),
			title: resp.problem_name,
			contest_id: task.contest.as_virt_symbol().to_owned(),
			site_short: "codechef".to_owned(),
			examples: None,
			statement,
			url: url.to_string(),
		})
	}

	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		// Querying languages doesn't require login, in contrast to most other sites.
		let resp = json::from_resp::<api::Languages>(
			session
				.client
				.get(format!("https://www.codechef.com/api/ide/{}/languages/{}", task.contest.as_virt_symbol(), task.task).parse()?)
				.send()
				.await?,
			"/api/ide/{}/languages/{}",
		)
		.await?;
		Ok(resp
			.languages
			.into_iter()
			.map(|language| Language { id: language.id, name: format!("{}({})", language.full_name, language.version) })
			.collect())
	}

	async fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>> {
		// There is also an API to query a specific submission, but it is not available in other sites and would require refactoring unijudge.
		// However, using it would possible make things faster and also get rid of the insanity that is querying all these submission lists.
		let doc = Document::new(
			&session
				.client
				.get(
					format!(
						"https://www.codechef.com/{}status/{},{}",
						match &task.contest {
							Contest::Practice => String::new(),
							Contest::Normal(contest) => format!("{}/", contest),
						},
						task.task,
						session.req_user()?
					)
					.parse()?,
				)
				.send()
				.await?
				.text()
				.await?,
		);
		if doc.find("#recaptcha-content").is_ok() {
			// This could possibly also happen in the other endpoints.
			// But CodeChef is nice and liberal with the number of requests, so even this is unnecessary.
			// If I'll ever add a config option for network delays at least the most common case will be caught.
			// I don't think I'll bother for other sites, since I only discovered this due to an error on my side.
			return Err(Error::RateLimit);
		}
		// If the code was submitted as a team, but tracking is done after logout, this will return an empty list every time.
		// But I don't think this is a common situation so let's just ignore it, until the huge tracking refactor fixes that.
		doc.find(".dataTable")?
			.find_all("tbody > tr")
			.map(|row| {
				let id = row.find_nth("td", 0)?.text().string();
				let verdict = row.find_nth("td", 3)?.find("span")?.attr("title")?.map(|verdict| match verdict {
					"accepted" => Ok(Verdict::Accepted),
					"wrong answer" => Ok(Verdict::Rejected { cause: Some(RejectionCause::WrongAnswer), test: None }),
					"waiting.." => Ok(Verdict::Pending { test: None }),
					"compilation error" => Ok(Verdict::Rejected { cause: Some(RejectionCause::CompilationError), test: None }),
					"compiling.." => Ok(Verdict::Pending { test: None }),
					"running.." => Ok(Verdict::Pending { test: None }),
					"running judge.." => Ok(Verdict::Pending { test: None }),
					"time limit exceeded" => Ok(Verdict::Rejected { cause: Some(RejectionCause::TimeLimitExceeded), test: None }),
					re if re.starts_with("runtime error") => Ok(Verdict::Rejected { cause: Some(RejectionCause::RuntimeError), test: None }),
					_ => Err(format!("unrecognized verdict {:?}", verdict)),
				})?;
				Ok(Submission { id, verdict })
			})
			.collect()
	}

	async fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		// This seems to work even if submitting as a team, although there are some mild problems with tracking.
		let resp = session
			.client
			.post("https://www.codechef.com/api/ide/submit".parse()?)
			.form(&[("sourceCode", code), ("language", &language.id), ("problemCode", &task.task), ("contestCode", task.contest.as_virt_symbol())])
			.send()
			.await?;
		if resp.status() == StatusCode::FORBIDDEN {
			return Err(Error::AccessDenied);
		}
		let endpoint = "/api/ide/submit";
		let resp_raw = resp.text().await?;
		let resp = json::from_str::<api::Submit>(&resp_raw, endpoint)?;
		if resp.status == "OK" {
			Ok(resp.upid.ok_or_else(|| Error::UnexpectedJSON { endpoint, backtrace: Backtrace::new(), resp_raw, inner: None })?)
		} else {
			Err(Error::RateLimit)
		}
	}

	fn task_url(&self, _session: &Self::Session, task: &Self::Task) -> Result<String> {
		Ok(format!("https://www.codechef.com/{}/problems/{}", task.contest.as_virt_symbol(), task.task))
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		contest.as_virt_symbol().to_owned()
	}

	fn contest_site_prefix(&self) -> &'static str {
		"CodeChef"
	}

	async fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		Ok(self.contest_details_ex(session, contest).await?.tasks)
	}

	fn contest_url(&self, contest: &Self::Contest) -> String {
		format!("https://www.codechef.com/{}", contest.as_virt_symbol())
	}

	async fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		Ok(self.contest_details_ex(session, contest).await?.title)
	}

	async fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		let doc = Document::new(&session.client.get("https://www.codechef.com/contests".parse()?).send().await?.text().await?);
		doc.find("#primary-content > .content-wrapper")?
			// CodeChef does not separate ongoing contests and permanent contests, so we only select the upcoming ones.
			// This is irritating, but I would like to add some general heuristics for all sites later.
			// Doing this only for CodeChef wouldn't make sense because it's better to also handle SPOJ and sio2 at the same time.
			.find_nth("table", 1)?
			.find_all("tbody > tr")
			.map(|row| {
				let id = Contest::Normal(row.find_nth("td", 0)?.text().string());
				let title = row.find_nth("td", 1)?.text().string();
				let start =
					row.find_nth("td", 2)?.attr("data-starttime")?.map(|start_time| unijudge::chrono::DateTime::parse_from_rfc3339(start_time))?;
				Ok(ContestDetails { id, title, start })
			})
			.collect()
	}

	fn name_short(&self) -> &'static str {
		"codechef"
	}

	fn supports_contests(&self) -> bool {
		true
	}
}

struct ContestDetailsEx {
	tasks: Vec<Task>,
	title: String,
}

struct OtherSessions {
	others: Vec<(String, String)>,
	form_build_id: String,
	form_token: String,
}

impl CodeChef {
	fn select_other_sessions(&self, doc: &Document) -> Result<OtherSessions> {
		let form = doc.find("#session-limit-page")?;
		let form_build_id = form.find("[name=form_build_id]")?.attr("value")?.string();
		let form_token = form.find("[name=form_token]")?.attr("value")?.string();
		let others = form
			.find_all(".form-item > .form-checkboxes > .form-item")
			.filter(|fi| fi.find("b").map(|b| b.text().as_str().is_empty()).unwrap_or(true))
			.map(|fi| {
				let name = fi.find("input")?.attr("name")?.string();
				let value = fi.find("input")?.attr("value")?.string();
				Ok((name, value))
			})
			.collect::<Result<_>>()?;
		Ok(OtherSessions { others, form_build_id, form_token })
	}

	async fn disconnect_other_sessions(&self, session: &Session, other: OtherSessions) -> Result<()> {
		let payload = other
			.others
			.iter()
			.map(|(k, v)| (k.as_str(), v.as_str()))
			.chain(
				[
					("op", "Disconnect session"),
					("form_build_id", &other.form_build_id),
					("form_token", &other.form_token),
					("form_id", "session_limit_page"),
				]
				.iter()
				.cloned(),
			)
			.collect::<Vec<_>>();
		session.client.post("https://www.codechef.com/session/limit".parse()?).form(&payload).send().await?;
		Ok(())
	}

	async fn contest_details_ex(&self, session: &Session, contest: &Contest) -> Result<ContestDetailsEx> {
		let endpoint = "https://www.codechef.com/api/contests/{}";
		let resp_raw =
			session.client.get(format!("https://www.codechef.com/api/contests/{}", contest.as_virt_symbol()).parse()?).send().await?.text().await?;
		let resp = json::from_str::<api::ContestTasks>(&resp_raw, endpoint)?;
		if let Some(tasks) = resp.problems {
			let mut tasks: Vec<_> =
				tasks.into_iter().map(|kv| (Task { contest: contest.clone(), task: kv.1.code }, kv.1.successful_submissions)).collect();
			// CodeChef does not sort problems by estimated difficulty, contrary to Codeforces/AtCoder.
			// Instead, it sorts them by submission count. This is problematic when contest begin, as all problems have a submit count of 0.
			// But since this naive sort is as good as what you get with a browser, let's just ignore this.
			tasks.sort_unstable_by_key(|task| u64::max_value() - task.1);
			Ok(ContestDetailsEx { title: resp.name, tasks: tasks.into_iter().map(|kv| kv.0).collect() })
		} else if resp.time.current <= resp.time.start {
			Err(Error::NotYetStarted)
		} else if !resp.user.username.is_empty() {
			// If no tasks are present, that means CodeChef would present us with a "choose your division" screen.
			// Fortunately, it also checks which division are you so we can just choose that one.
			let tasks: Option<_> = try {
				let div = resp.user_rating_div?.div.code;
				let child = &resp.child_contests.as_ref()?.get(&div).as_ref()?.contest_code;
				let contest = Contest::Normal(child.clone());
				self.contest_details_ex_boxed(session, &contest).await
			};
			tasks.ok_or_else(|| Error::UnexpectedJSON { endpoint, backtrace: Backtrace::new(), resp_raw, inner: None })?
		} else {
			// If no username is present in the previous case, codechef assumes you're div2.
			// This behaviour is unsatisfactory, so we require a login from the user.
			Err(Error::AccessDenied)
		}
	}

	fn contest_details_ex_boxed<'a>(
		&'a self,
		session: &'a Session,
		contest: &'a Contest,
	) -> Pin<Box<dyn Future<Output=Result<ContestDetailsEx>>+Send+'a>> {
		Box::pin(self.contest_details_ex(session, contest))
	}

	fn prepare_statement(&self, title: &str, text: String) -> Statement {
		let mut html = String::new();
		// CodeChef statements are pretty wild. They seem to follow some structure and use Markdown, but it's not true.
		// They mix Markdown and HTML very liberally, and their Markdown implementation is not standard-compliant.
		// So e.g. you can have sections with "###Example input", which CommonMark parsers ignore.
		// Fortunately, we can ignore the HTML because Markdown permits it.
		// Also, we add a title so that the statement looks better.
		pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(&format!("# {}\n\n{}", title, text.replace("###", "### "))));
		Statement::HTML {
			html: format!(
				r#"
<html>
	<head>
		<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/3.0.1/github-markdown.min.css">
		<script type="text/x-mathjax-config">
			MathJax.Hub.Config({{
				tex2jax: {{inlineMath: [['$','$']]}}
			}});
		</script>
		<script src='https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.5/MathJax.js?config=TeX-MML-AM_CHTML' async></script>
		<style>
			.markdown-body {{
				background-color: white;
				padding-bottom: 20px;
			}}
			.markdown-body code {{
				color: #24292e;
			}}
			.solution-visible-txt {{
				display: none;
			}}
		</style>
	</head>
	<body class="markdown-body">
		{}
	<body>
</html>"#,
				html
			),
		}
	}
}
impl Session {
	fn req_user(&self) -> Result<String> {
		self.username.lock().map_err(|_| Error::StateCorruption)?.clone().ok_or(Error::AccessDenied)
	}
}
impl Contest {
	fn as_virt_symbol(&self) -> &str {
		match self {
			Contest::Normal(name) => name.as_str(),
			Contest::Practice => "PRACTICE",
		}
	}
}

mod api {

	use serde::{
		de::{self, MapAccess, SeqAccess, Unexpected}, export::PhantomData, Deserialize, Deserializer
	};
	use std::{collections::HashMap, fmt, hash::Hash};

	#[derive(Debug, Deserialize)]
	pub struct Task {
		pub problem_name: String,
		/// Task statement in Markdown with HTML tags and MathJax $ tags.
		/// Contains example tests.
		pub body: String,
	}

	#[derive(Debug, Deserialize)]
	pub struct Languages {
		pub languages: Vec<Language>,
	}

	#[derive(Debug, Deserialize)]
	pub struct Language {
		pub id: String,
		pub full_name: String,
		pub version: String,
	}

	#[derive(Debug, Deserialize)]
	pub struct Submit {
		pub status: String,
		#[serde(default)]
		pub upid: Option<String>,
		#[serde(default)]
		pub errors: Option<Vec<String>>,
	}

	#[derive(Debug, Deserialize)]
	pub struct ContestTasksTask {
		pub code: String,
		// This field is sometimes returned as an integer, and sometimes as a string.
		// The pattern seems to be that zeroes are returned as integers, and anything else as strings.
		// I don't even want to know why on earth does the backend do that.
		#[serde(deserialize_with = "de_u64_or_u64str")]
		pub successful_submissions: u64,
	}
	#[derive(Debug, Deserialize)]
	pub struct ContestTasksTime {
		pub start: i64,
		pub current: i64,
	}
	#[derive(Debug, Deserialize)]
	pub struct ContestTasksDivision {
		pub code: String,
	}
	#[derive(Debug, Deserialize)]
	pub struct ContestTasksUserRatingDiv {
		pub div: ContestTasksDivision,
	}
	#[derive(Debug, Deserialize)]
	pub struct ContestTasksChildContest {
		pub contest_code: String,
	}
	#[derive(Debug, Deserialize)]
	pub struct ContestTasksUser {
		pub username: String,
	}
	#[derive(Debug, Deserialize)]
	pub struct ContestTasks {
		pub user: ContestTasksUser,
		pub name: String,
		// When this fields is an object, it contains a task symbol => task details sorted in no particular order.
		// However, it can also be an empty array - which means the contest has not started or is a parent contest.
		#[serde(deserialize_with = "de_hash_map_or_empty_vec")]
		pub problems: Option<HashMap<String, ContestTasksTask>>,
		pub time: ContestTasksTime,
		#[serde(default)]
		pub child_contests: Option<HashMap<String, ContestTasksChildContest>>,
		#[serde(default)]
		pub user_rating_div: Option<ContestTasksUserRatingDiv>,
	}

	fn de_hash_map_or_empty_vec<'d, D: Deserializer<'d>>(d: D) -> Result<Option<HashMap<String, ContestTasksTask>>, D::Error> {
		d.deserialize_any(HashMapOrEmptyVec(PhantomData))
	}
	struct HashMapOrEmptyVec<'d, K: Eq+Hash+Deserialize<'d>, V: Deserialize<'d>>(PhantomData<&'d (K, V)>);
	impl<'d, K: Eq+Hash+Deserialize<'d>, V: Deserialize<'d>> serde::de::Visitor<'d> for HashMapOrEmptyVec<'d, K, V> {
		type Value = Option<HashMap<K, V>>;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			write!(formatter, "a hash map or an empty vector")
		}

		fn visit_seq<A: SeqAccess<'d>>(self, mut seq: A) -> Result<Self::Value, <A as SeqAccess<'d>>::Error> {
			match seq.next_element::<()>() {
				Ok(None) => Ok(None),
				Ok(Some(_)) => Err(de::Error::invalid_value(Unexpected::Seq, &self)),
				Err(e) => Err(e),
			}
		}

		fn visit_map<A: MapAccess<'d>>(self, mut map: A) -> Result<Self::Value, <A as MapAccess<'d>>::Error> {
			let mut acc = HashMap::new();
			while let Some(kv) = map.next_entry::<K, V>()? {
				acc.insert(kv.0, kv.1);
			}
			Ok(Some(acc))
		}
	}
	fn de_u64_or_u64str<'d, D: Deserializer<'d>>(d: D) -> Result<u64, D::Error> {
		d.deserialize_any(U64OrU64Str)
	}
	struct U64OrU64Str;
	impl<'d> serde::de::Visitor<'d> for U64OrU64Str {
		type Value = u64;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			write!(formatter, "{}", Self::EXPECTING)
		}

		fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
			Ok(v)
		}

		fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
			v.parse().map_err(|_| E::invalid_type(Unexpected::Str(v), &Self::EXPECTING))
		}
	}
	impl U64OrU64Str {
		const EXPECTING: &'static str = "an u64 or an u64 string";
	}
}
