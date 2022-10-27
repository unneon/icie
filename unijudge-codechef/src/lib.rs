#![feature(try_blocks)]
use markdown;
use html_escape;
use std::convert::TryInto;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Mutex};
use unijudge::{
	chrono::{prelude::*,Duration},
	debris::{ Document, Find}, http::{Client, Cookie}, json, log::{debug, error}, reqwest::{ Url}, ContestDetails, ContestTime, ErrorCode, Language, RejectionCause, Resource, Result, Statement, Submission, TaskDetails, Verdict
};
use urlencoding::decode;
use node_sys::console;
use std::collections::HashMap;
#[derive(Debug)]
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
	prefix: i64,
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

#[async_trait(?Send)]
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
		// This is the only place where PRACTICE doesn't work, it's treated as a normal contest
		// everywhere else.
		match segments {
			["problems-old", task] => Ok(Resource::Task(Task { contest: Contest::Practice, task: (*task).to_owned() , prefix:0})),
			["problems", task] => Ok(Resource::Task(Task { contest: Contest::Practice, task: (*task).to_owned(), prefix:0 })),
			["submit", task] => Ok(Resource::Task(Task { contest: Contest::Practice, task: (*task).to_owned() , prefix:0})),
			["PRACTICE", "problems", task] => {
				Ok(Resource::Task(Task { contest: Contest::Practice, task: (*task).to_owned() , prefix:0}))
			},
			[contest, "problems", task] => {
				Ok(Resource::Task(Task { contest: Contest::Normal((*contest).to_owned()), task: (*task).to_owned() , prefix:0}))
			},
			[contest] => Ok(Resource::Contest(Contest::Normal((*contest).to_owned()))),
			_ => Err(ErrorCode::WrongTaskUrl.into()),
		}
	}

	fn connect(&self, client: Client, _domain: &str) -> Self::Session {
		Session { client, username: Mutex::new(None) }
	}

	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let username = session.username.lock()?.clone();
		let c_sess = session.client.cookie_get_if(|c| c.starts_with("SESS"))?;
		Ok(try { CachedAuth { username: username?, c_sess: c_sess? } })
	}

	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		unijudge::deserialize_auth(data)
	}

	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		/*debug!("starting login");
		session.client.cookies_clear()?;
		let resp1 = session.client.get("https://www.codechef.com".parse()?).send().await?;
		let doc = Document::new(&resp1.text().await?);
		debug!("received the login form");
		let form = doc.find("#new-login-form")?;
		let form_build_id = form.find("[name=form_build_id]")?.attr("value")?.string();*/
		//let csrf = form.find("[name=csrfToken]")?.attr("value")?.string();
		//session.client.cookies_clear()?;
        let resp = session
                       .client
                       .get(format!("https://www.codechef.com/api/codechef/login").parse()?)
                       .send()
                       .await?
                       .text()
                       .await?;
        let re= regex::Regex::new("id=\"(form-[_0-9A-Za-z-]+)\"").unwrap();
        let resp_raw = json::from_str::<api::Login>(&resp)?;
        let formdata=resp_raw.form;
        if ! re.is_match(&formdata) {
            return Err(ErrorCode::AccessDenied.into());
        }
        let cap =re.captures(&formdata).unwrap();
        let form_build_id = cap.get(1).unwrap().as_str();

        let resp2 = session
			.client
            .post(format!("https://www.codechef.com/api/codechef/login").parse()?)
			.form(&[
				("name", username),
				("pass", password),
				("form_build_id", &form_build_id),
				("form_id", "ajax_login_form")
			])
			.send()
			.await?
            .text()
            .await?;
        let resp = json::from_str::<api::SuccessorError>(&resp2)?;
		debug!("sent the login form");
        if resp.status== "success" {
            debug!("OK logged in");
        } else {
            return Err(ErrorCode::WrongCredentials.into());
        }
        *session.username.lock()? = Some(username.to_owned());
        debug!("seemingly logged in");
        Ok(())
	}

	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		debug!("restoring an old session");
		*session.username.lock()? = Some(auth.username.clone());
		session.client.cookie_set(auth.c_sess.clone(), "https://www.codechef.com")?;
		Ok(())
	}

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		unijudge::serialize_auth(auth)
	}

	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest> {
		Some(task.contest.clone())
	}

	async fn remain_time(&self, session: &Self::Session, task: &Self::Task) -> Result<i64>{
		//session.req_user()?;
		//console::debug(&format!("Reached Here remain time"));
		let resp_raw = session
			.client
			.get(format!("https://www.codechef.com/api/contests/{}", task.contest.as_virt_symbol()).parse()?)
			.send()
			.await?
			.text()
			.await?;
		let resp = json::from_str::<api::ContestTasks>(&resp_raw)?;
		if resp.time.current <= resp.time.start {
			return Err(ErrorCode::NotYetStarted.into());
		}else if resp.time.current > resp.time.end {
			return Err(ErrorCode::Ended_Already.into());
		}
		let naive_end = NaiveDateTime::from_timestamp(resp.time.end, 0);
		let end_time: DateTime<Utc> = DateTime::from_utc(naive_end, Utc);
		let today: DateTime<Utc> = Utc::now();
		let diff = end_time.signed_duration_since(today);
		let secs = diff.num_seconds();
		return Ok(secs);
		/*
		console::debug(&format!("time {}",result));
		return Ok(result);*/
	}
	
	async fn rank_list(&self, session: &Self::Session, task: &Self::Task) -> Result<String>{
		session.req_user()?;
		match task.contest {
			Contest::Normal(_) => {
				let submiturl= format!("https://www.codechef.com/rankings/{}?itemsPerPage=100&order=asc&page=1&sortBy=rank",task.contest.as_virt_symbol()).parse()?;
				let doc = session.client.get(submiturl).send().await?.text().await?;
				let re= regex::Regex::new("window.csrfToken = \"([_0-9A-Za-z-]+)\"").unwrap();
				let cap =re.captures(&doc).unwrap();
				let csrf_tok=cap.get(1).unwrap().as_str();
				let list_rank=self.get_next_page_list(session,task,1,csrf_tok.to_string()).await?;
				return Ok("Rank: ".to_owned()+&list_rank.rank_and_score.rank+" , Score: "+&list_rank.rank_and_score.score);
					
				/*let mut i_rank=1;
				for p in 1..=list_rank.availablePages{
					let page =self.get_next_page_list(session,task,p,csrf_tok.to_string()).await?;
					for user in page.list.iter(){
						if user.user_handle==session.req_user()?{
							return Ok("Rank: ".to_owned()+&user.rank.to_string()+" ("+&i_rank.to_string()+") , Score: "+&user.score.to_string());
						}
						if user.country=="India"
							{i_rank+=1;}
					}
				}*/
				//Ok("user not found".to_string())
			},
			Contest::Practice => Ok("Practice contest has no ranklist".to_string()),
		}
		
	}
	
	
	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		session.req_user()?;
		debug!("querying task details of {:?}", task);
		let resp = self.api_task(task, session).await?;
		let cases = Some(resp.problemComponents.sampleTestCases.iter().map(|tc|
                                                                            Ok(unijudge::Example {
                                                                                input: tc.input.clone(),
                                                                                output: tc.output.clone(),
                                                                            })
                                                                            ).collect::<Result<_>>()?
                         );

        let statement = Some(self.prepare_statement(&resp.problem_name, resp.problemComponents));
		Ok(TaskDetails {
			id: task.task.clone(),
			title: unijudge::fmt_title(task.prefix)+&resp.problem_name,
			contest_id: task.contest.as_virt_symbol().to_owned(),
			site_short: "codechef".to_owned(),
			examples: cases,
			statement,
			url: self.task_url(session, task)?,
		})
	}

	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		debug!("querying languages of {:?}", task);
        //session.req_user();
        let submiturl= self.active_submit_url(task, session).await?;
        let doc = session.client.get(submiturl.clone()).send().await?.text().await?;
        let re= regex::Regex::new("window.csrfToken = \"([_0-9A-Za-z-]+)\"").unwrap();
        let cap =re.captures(&doc).unwrap();
        let csrf_tok=cap.get(1).unwrap().as_str();
        let url = self.active_languages_url(task, session).await?;
        let resp = session.client.get(url)
            .header("x-csrf-token",csrf_tok)
            .send().await?.text().await?;
        let langs = json::from_str::<api::LanguageList>(&resp)?;
        langs.languages.iter().map(|lang|
                                   Ok(Language { id: lang.id.clone(), name: lang.full_name.clone()+"(" + &lang.version.clone()+")"}))
            .collect()

	}

	async fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>> {
		session.req_user()?;
		// There is also an API to query a specific submission, but it is not available in other
		// sites and would require refactoring unijudge. However, using it would possible make
		// things faster and also get rid of the insanity that is querying all these submission
		// lists.
		let url = self.active_submission_url(task, session).await?;
		let doc = Document::new(&session.client.get(url).send().await?.text().await?);
		if doc.find("#recaptcha-content").is_ok() {
			// This could possibly also happen in the other endpoints.
			// But CodeChef is nice and liberal with the number of requests, so even this is
			// unnecessary. If I'll ever add a config option for network delays at least the most
			// common case will be caught. I don't think I'll bother for other sites, since I only
			// discovered this due to an error on my side.
			return Err(ErrorCode::RateLimit.into());
		}
		// If the code was submitted as a team, but tracking is done after logout, this will return
		// an empty list every time. But I don't think this is a common situation so let's just
		// ignore it, until the huge tracking refactor fixes that.
		let mut output:String="".to_string();
		if doc.find(".dataTable")?.find_nth("tbody > tr",0).is_ok(){
			let first_id=doc.find(".dataTable")?.find_nth("tbody > tr",0)?.find_nth("td", 0)?.text().string();
			let ver_txt=doc.find(".dataTable")?.find_nth("tbody > tr",0)?.find_nth("td", 3)?.find_nth("span",0)?.attr("title")?.string();
			
			if ver_txt == "wrong answer" || ver_txt=="time limit exceeded" {
				//console::debug(&format!("Quering {}",first_id));
				let status=self.error_table(first_id).await?;
				let tab_res=session.client.get(status)
						.send().await?.text().await?;
				
				//let re= regex::Regex::new("\"testInfo\":\"([^\"]*)\"").unwrap();
				//let cap =re.captures(&tab_res).unwrap();
         		//let tab_info=cap.get(1).unwrap().as_str();
				let table_stat = Document::new(&tab_res);
				//console::debug(&format!("Response {:?}",table_stat));
				let mut setofans: HashMap<String, i64> = HashMap::new();
				//console::debug(&format!("Response {:?}",table_stat.find(".status-table")));
				//console::debug(&format!("Response {:?}",table_stat.find(".status-table")?.find("tr")));
				//console::debug(&format!("Response {:?}",table_stat.find(".status-table")?.find("tbody")?.find_nth("tr",3)));
				
				
				let vals:Vec<_> = table_stat.find(".status-table")?.find("tbody")?.find_all("tr").map(|row| {
					//console::debug(&format!("R {:?}",row));
					if row.find_nth("td",2).is_ok() {
						//console::debug(&format!("RR {:?}",row));
						let verdict_td = row.find_nth("td", 2).unwrap().text().string();
						//console::debug(&format!("Verdict {}",verdict_td));
						let re= regex::Regex::new("([A-Z]+).*").unwrap();
						let cap =re.captures(&verdict_td).unwrap();
						let verdict=cap.get(1).unwrap().as_str();
						*setofans.entry(verdict.to_string()).or_insert(0) += 1;	
					};
				}).collect();
				//console::debug(&format!("{}",first_id));
				for (key, value) in setofans {
					//console::debug(&format!("keys {}-{}",key,value.to_string()));
					output+=&(key+" : "+&value.to_string()+",");
				}
				//console::debug(&format!("Output {}",output));
			}
		}
		
		
		
		doc.find(".dataTable")?
			.find_all("tbody > tr").enumerate()
			.map(|(i,row)| {
				let id = row.find_nth("td", 0)?.text().string();
				let verdict_td = row.find_nth("td", 3)?;
				let verdict_text = verdict_td.text();
				let stat=output.clone();
                /*let check = || -> Result<(), ErrorCode> {
                    verdict_td.find_nth("span",0)?.attr("title")?;
                    Ok(())
                };
                if let Err(_err) = check() {
                    return Ok(Submission { id, Verdict::Pending { test: None } });
                };*/
				
				let verdict = verdict_td.find_nth("span",0)?.attr("title")?.map( |verdict| match verdict {
					"accepted" => Ok(Verdict::Accepted),
					"wrong answer" =>{
						if i==0
						{Ok(Verdict::Rejected { cause: Some(RejectionCause::WrongAnswer), test: Some(stat) })}
						else 
						{Ok(Verdict::Rejected { cause: Some(RejectionCause::WrongAnswer), test: None })}
					}, 
					"waiting.." => Ok(Verdict::Pending { test: None }),
					"compilation error" => {
						Ok(Verdict::Rejected { cause: Some(RejectionCause::CompilationError), test: None })
					},
					"compiling.." => Ok(Verdict::Pending { test: None }),
					"running.." => Ok(Verdict::Pending { test: None }),
					"running judge.." => Ok(Verdict::Pending { test: None }),
					"time limit exceeded" => {
						if i==0
						{Ok(Verdict::Rejected { cause: Some(RejectionCause::TimeLimitExceeded), test:  Some(stat) })}
						else 
						{Ok(Verdict::Rejected { cause: Some(RejectionCause::TimeLimitExceeded), test: None })}
					},
					re if re.starts_with("runtime error") => {
						Ok(Verdict::Rejected { cause: Some(RejectionCause::RuntimeError), test: None })
					},
					"" if verdict_text.as_str().contains("pts") => {
						let score_regex = regex::Regex::new("\\[(.*)pts\\]").unwrap();
						let score_matches = score_regex.captures(verdict_text.as_str()).ok_or("score regex error")?;
						let score = score_matches[1].parse().map_err(|_| "score f64 parse error")?;
						Ok(Verdict::Scored { score, max: Some(100.), cause: None, test: None })
					},
					_ => Err(format!("unrecognized verdict {:?}", verdict)),
				})?;
				
                //if let Err(_err) = verdict 
				Ok(Submission { id, verdict })
			})
			.collect()
	}

	async fn task_submit(
		&self,
		session: &Self::Session,
		task: &Self::Task,
		language: &Language,
		code: &str,
	) -> Result<String> {
		session.req_user()?;
        //session.req_user();
        let submiturl= self.active_submit_url(task, session).await?;
        let doc = session.client.get(submiturl.clone()).send().await?.text().await?;
        let re= regex::Regex::new("window.csrfToken = \"([_0-9A-Za-z-]+)\"").unwrap();
        let cap =re.captures(&doc).unwrap();
         let csrf_tok=cap.get(1).unwrap().as_str();
         let url = "https://www.codechef.com/api/ide/submit".parse()?;
		let resp = session
			.client
			.post(url)
			.header("x-csrf-token",csrf_tok)
            .form(&[
                  ("language", language.id.clone()),
                  ("contestCode", task.contest.as_virt_symbol().to_owned()),
                  ("problemCode", task.task.clone()),
                  ("sourceCode",code.to_owned())
            ])
            .send()
			.await?
            .text()
            .await?;
        let resp = json::from_str::<api::Submit>(&resp)?;
        if resp.status== "OK" {
            debug!("OK submitted");
            Ok((resp.upid.ok_or("").unwrap()).to_owned())
        } else {
            return Err(ErrorCode::AlienInvasion.into());
        }
	}

	fn task_url(&self, _session: &Self::Session, task: &Self::Task) -> Result<String> {
		Ok(format!("https://www.codechef.com/{}/problems/{}", task.contest.as_virt_symbol(), task.task))
	}

	fn submission_url(&self, _session: &Self::Session, _task: &Self::Task, id: &str) -> String {
		format!("https://www.codechef.com/submit/complete/{}", id)
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
		match contest {
			Contest::Normal(contest) => format!("https://www.codechef.com/{}", contest),
			Contest::Practice => "https://www.codechef.com/problems/school".to_owned(),
		}
	}

	async fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		Ok(self.contest_details_ex(session, contest).await?.title)
	}

	async fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		/*let doc = Document::new(
			&session.client.get("https://www.codechef.com/contests".parse()?).send().await?.text().await?,
		);
		// CodeChef does not separate ongoing contests and permanent contests, so we only select the
		// upcoming ones. This is irritating, but I would like to add some general heuristics for
		// all sites later. Doing this only for CodeChef wouldn't make sense because it's better to
		// also handle SPOJ and sio2 at the same time.
		let contests = doc.find("#primary-content > .content-wrapper")?;
		let table_ongoing = contests.find_nth("table", 0)?;
		let table_upcoming = contests.find_nth("table", 1)?;
		let rows_ongoing = table_ongoing.find_all("tbody > tr").map(|row| (row, true));
		let rows_upcoming = table_upcoming.find_all("tbody > tr").map(|row| (row, false));
		rows_ongoing
			.chain(rows_upcoming)
			.map(|(row, is_ongoing)| {
				let id = Contest::Normal(row.find_nth("td", 0)?.text().string());
				let title = row.find_nth("td", 1)?.text().string();
				let datetime = row
					.find_nth("td", if is_ongoing { 3 } else { 2 })?
					.attr(if is_ongoing { "data-endtime" } else { "data-starttime" })?
					.map(|start_time| unijudge::chrono::DateTime::parse_from_rfc3339(start_time))?;
				let time = if is_ongoing {
					ContestTime::Ongoing { finish: datetime }
				} else {
					ContestTime::Upcoming { start: datetime }
				};
				Ok(ContestDetails { id, title, time })
			})
			.collect()
            */
        let resp_raw = session
            .client
            .get(format!("https://www.codechef.com/api/list/contests/all?sort_by=START&sorting_order=asc&offset=0").parse()?)
            .send()
            .await?
            .text()
            .await?;
        let resp = json::from_str::<api::ContestList>(&resp_raw)?;
        let rows_ongoing = resp.present_contests.iter().map(|row| (row, true));
        let rows_upcoming = resp.future_contests.iter().map(|row| (row, false));
        rows_upcoming
            .chain(rows_ongoing)
            .map(|(row, is_ongoing)| {
                let id = Contest::Normal(row.contest_code.to_string());
                let title = row.contest_name.to_string();
                let dtime= if is_ongoing { row.contest_end_date_iso.to_string() }
                else { row.contest_start_date_iso.to_string()};
                let datetime=unijudge::chrono::DateTime::parse_from_rfc3339(&dtime).unwrap();
                let time = if is_ongoing {
                    ContestTime::Ongoing { finish: datetime }
                } else {
                    ContestTime::Upcoming { start: datetime }
                };
                debug!("Contest Details {:?} {:?} {:?}", id,title,time);
                Ok(ContestDetails { id, title, time })
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

/*struct OtherSessions {
	others: Vec<(String, String)>,
	form_build_id: String,
	form_token: String,
}*/

impl CodeChef {
/*	fn select_other_sessions(&self, doc: &Document) -> Result<OtherSessions> {
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
*/

async fn get_next_page_list(&self, session: &Session, task: &Task, page:u64,csrf_tok:String) -> Result<api::Ranklist>{
	let url =format!("https://www.codechef.com/api/rankings/{}?itemsPerPage=100&order=asc&page={}&sortBy=rank", task.contest.as_virt_symbol(),page).parse()?;
	//console::debug(&format!("Task url:{}",format!("https://www.codechef.com/api/rankings/{}?itemsPerPage=100&order=asc&page={}&sortBy=rank", task.contest.as_virt_symbol(),page)));
	let resp=session.client.get(url)
		.header("x-csrf-token",csrf_tok)
		.send()
		.await?
		.text()
		.await?;
	Ok(json::from_str::<api::Ranklist>(&resp)?)
}
	async fn contest_details_ex(&self, session: &Session, contest: &Contest) -> Result<ContestDetailsEx> {
		session.req_user()?;
		let resp_raw = session
			.client
			.get(format!("https://www.codechef.com/api/contests/{}", contest.as_virt_symbol()).parse()?)
			.send()
			.await?
			.text()
			.await?;
		let resp = json::from_str::<api::ContestTasks>(&resp_raw)?;
		if let Some(tasks) = resp.problems {
			let mut prb_id = -1;
			let mut tasks: Vec<_> = tasks
				.into_iter()
				.map(|kv| {
					
					if kv.1.category_name=="unscored"{
						(Task { contest: contest.clone(), task: kv.1.code , prefix:-1}, kv.1.successful_submissions)
					}else {
						prb_id += 1; 
						(Task { contest: contest.clone(), task: kv.1.code , prefix:prb_id}, kv.1.successful_submissions)
					}
					
				}
				)
				.collect();
			// CodeChef does not sort problems by estimated difficulty, contrary to
			// Codeforces/AtCoder. Instead, it sorts them by submission count. This is problematic
			// when contest begin, as all problems have a submit count of 0. But since this naive
			// sort is as good as what you get with a browser, let's just ignore this.
			tasks.sort_unstable_by_key(|task| u64::max_value() - task.1);
			Ok(ContestDetailsEx { title: resp.name, tasks: tasks.into_iter().map(|kv| kv.0).collect() })
		} else if resp.time.current <= resp.time.start {
			Err(ErrorCode::NotYetStarted.into())
		} else if !resp.user.username.is_empty() {
			// If no tasks are present, that means CodeChef would present us with a "choose your
			// division" screen. Fortunately, it also checks which division are you so we can just
			// choose that one.
			let tasks: Option<_> = try {
				let div = resp.user_rating_div?.div.code;
				let child = &resp.child_contests.as_ref()?.get(&div).as_ref()?.contest_code;
				let contest = Contest::Normal(child.clone());
				self.contest_details_ex_boxed(session, &contest).await
			};
			tasks.ok_or(ErrorCode::AccessDenied)?
		} else {
			// If no username is present in the previous case, codechef assumes you're div2.
			// This behaviour is unsatisfactory, so we require a login from the user.
			Err(ErrorCode::AccessDenied.into())
		}
	}

	fn contest_details_ex_boxed<'a>(
		&'a self,
		session: &'a Session,
		contest: &'a Contest,
	) -> Pin<Box<dyn Future<Output=Result<ContestDetailsEx>>+'a>> {
		Box::pin(self.contest_details_ex(session, contest))
	}

	fn prepare_statement(&self, title: &str, compont: api::TaskComponents) -> Statement {
		//let mut html = String::new();
		// CodeChef statements are pretty wild. They seem to follow some structure and use Markdown,
		// but it's not true. They mix Markdown and HTML very liberally, and their Markdown
		// implementation is not standard-compliant. So e.g. you can have sections with "###Example
		// input", which CommonMark parsers ignore. Fortunately, we can ignore the HTML because
		// Markdown permits it. Also, we add a title so that the statement looks better.
        let mut casestr = "".to_owned();
        for tc in compont.sampleTestCases.iter(){
            casestr.push_str("\r\n\n###Example Input\r\n```\r\n");casestr.push_str( &tc.input );casestr.push_str("\t\r\n```\r\n\r\n");
             casestr.push_str("\r\n\n###Example Output\r\n```\r\n");casestr.push_str( &tc.output );casestr.push_str( "\t\r\n```\r\n\r\n");
             casestr.push_str("\r\n\n###Explanations\r\n");casestr.push_str(&tc.explanation );casestr.push_str("\r\n\n");
        }
        let inpf= "\r\n\n###Input Format\r\n".to_owned() + &compont.inputFormat;
        let outf= "\r\n\n###Output Format\r\n".to_owned()+ &compont.outputFormat;
        let consf= "\r\n\n###Constraints \r\n".to_owned()+&compont.constraints;
        let subtf= "\r\n\n###Subtasks\r\n".to_owned()+ &compont.subtasks;
        let text = compont.statement + if compont.inputFormatState  { &inpf } else {""} +
            if compont.outputFormatState  { &outf } else {""} +
            if compont.constraintsState  { &consf } else {""}+
            if compont.subtasksState  { &subtf } else {""} + &casestr;


		//pulldown_cmark::html::push_html(
		//	&mut html,
		//	pulldown_cmark::Parser::new(&format!("# {}\n\n{}", title, text.replace("###", "### "))),
		//);
        
        let  html_out=markdown::to_html(&html_escape::decode_html_entities(&format!("# {}\n\n{}", title, text.replace("###", "### "))));

		Statement::HTML {
			html: format!(
				r#"
<html>
	<head>
		<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/3.0.1/github-markdown.min.css">
		<script type="text/x-mathjax-config">
			MathJax.Hub.Config({{
				TeX: {{extensions: ['color.js'] }},tex2jax: {{inlineMath: [['$','$']],
                }}
			}});
		</script>
		<script src='https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.3/MathJax.js?config=TeX-AMS-MML_HTMLorMML' async></script>
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
				html_out
			),
		}
	}

	async fn api_task(&self, task: &Task, session: &Session) -> Result<api::Task> {
		let url: Url =
			format!("https://www.codechef.com/api/contests/{}/problems/{}", task.contest.as_virt_symbol(), task.task)
				.parse()?;
		let resp = session.client.get(url.clone()).send().await?;
		let obj = json::from_resp::<api::TaskOrError>(resp).await?;
		match obj {
			api::TaskOrError::Success { task } => Ok(task),
			api::TaskOrError::Error { message } if message == "Problem is not visible now. Please try again later." => {
				Err(ErrorCode::RateLimit.into())
			},
			api::TaskOrError::Error { message } => {
				error!("codechef api_task unexpected error message {:?}", message);
				Err(ErrorCode::AlienInvasion.into())
			},
		}
	}

	/// Queries "active" submit URL. In CodeChef, the submit URL parameters can be different from
	/// the task URL parameters for various reasons, e.g. after a contest ends, or when submitting a
	/// problem from a different division. This function performs an additional HTTP request to take
	/// this into account.
    async fn active_languages_url(&self, task: &Task, _session: &Session) -> Result<Url> {
        let url = format!("https://www.codechef.com/api/ide/{}/languages/{}", task.contest.as_virt_symbol(), task.task);
        Ok(url.parse()?)
    }
	async fn active_submit_url(&self, task: &Task, session: &Session) -> Result<Url> {
		let task = self.activate_task(task, session).await?;
		let url = format!("https://www.codechef.com/{}/submit/{}", task.contest.prefix(), task.task);
        Ok(url.parse()?)
	}

	/// See [`CodeChef::active_submit_url`], but for submission list URLs.
	async fn active_submission_url(&self, task: &Task, session: &Session) -> Result<Url> {
		let task = self.activate_task(task, session).await?;
		let url =
			format!("https://www.codechef.com/{}/status/{},{}", task.contest.prefix(), task.task, session.req_user()?);
		Ok(url.parse()?)
	}

	async fn error_table(&self, id:String) -> Result<Url> {
		
		let url =
			format!("https://www.codechef.com/error_status_table/{}/",id);
		Ok(url.parse()?)
	}

	async fn activate_task(&self, task: &Task, session: &Session) -> Result<Task> {
		let active_contest = match &task.contest {
			Contest::Normal(contest) => {
				debug!("confirming submit target");
				let details = self.api_task(task, session).await?;
				if session.req_user().err().map(|e| e.code) == Some(ErrorCode::AccessDenied)
					|| details.user.username.ok_or("").unwrap() != session.req_user()?
				{
					debug!("failed to cofirm submit target, requesting login");
					return Err(ErrorCode::AccessDenied.into());
				} else if details.time.current <= details.time.end_date {
					debug!("submit target confirmed to canonical url");
					Contest::Normal(contest.clone())
				} else if details.time.practice_submission_allowed.unwrap_or(true) {
					debug!("submit target confirmed to practice url");
					Contest::Practice
				} else {
					error!("failed to confirm submit target, falling back to canonical");
					Contest::Normal(contest.clone())
				}
			},
			Contest::Practice => Contest::Practice,
		};
		Ok(Task { contest: active_contest, task: task.task.clone(), prefix:0 })
	}
}
impl Session {
	fn req_user(&self) -> Result<String> {
		let username = self.username.lock()?.clone().ok_or(ErrorCode::AccessDenied)?;
		Ok(username)
	}
}
impl Contest {
	fn as_virt_symbol(&self) -> &str {
		match self {
			Contest::Normal(name) => name.as_str(),
			Contest::Practice => "PRACTICE",
		}
	}

	fn prefix(&self) -> String {
		match self {
			Contest::Normal(name) => format!("{}/", name),
			Contest::Practice => String::new(),
		}
	}
}

mod api {

	use serde::{
		de::{self, MapAccess, SeqAccess, Unexpected}, __private::PhantomData, Deserialize, Deserializer
	};
	use std::{collections::HashMap, fmt, hash::Hash};

	#[derive(Debug, Deserialize)]
	pub struct TaskTime {
		pub end_date: u64,
		pub current: u64,
		pub practice_submission_allowed: Option<bool>,
	}

	#[derive(Debug, Deserialize)]
	pub struct TaskUser {
        pub username: Option<String>,
    }
    #[derive(Debug, Deserialize)]
    pub struct Language{
        pub id:String,
        pub short_name:String,
        pub full_name:String,
        pub version:String
    }

    #[derive(Debug, Deserialize)]
    pub struct LanguageList{
        pub languages:Vec<Language>
    }

    #[derive(Debug, Deserialize)]
    pub struct SubmissionDetails{
        pub result_code: String,
        pub score: String,
        pub upid: String,

    }

    #[derive(Debug, Deserialize)]
    pub struct TestCase{
        pub id:String,
        pub input:String,
        pub explanation:String,
        pub output:String,
    }

    #[derive(Debug, Deserialize)]
    pub struct TaskComponents{
        pub constraints:String,
        pub constraintsState:bool,
        pub subtasks:String,
        pub subtasksState:bool,
        pub statement:String,
        pub inputFormat:String,
        pub inputFormatState:bool,
        pub outputFormat:String,
        pub outputFormatState:bool,
        pub sampleTestCases: Vec<TestCase>
    }

	#[derive(Debug, Deserialize)]
	pub struct Task {
		pub problem_name: String,
		/// Task statement in Markdown with HTML tags and MathJax $ tags.
		/// Contains example tests.
		pub body: String,
		pub time: TaskTime,
		pub user: TaskUser,
        pub problemComponents: TaskComponents,
	}
    #[derive(Debug, Deserialize)]
    pub struct SuccessorError{
        pub status:String
    }
    #[derive(Debug, Deserialize)]
    pub struct Login{
        pub form: String
    }


	#[derive(Debug, Deserialize)]
	#[serde(tag = "status")]
	pub enum TaskOrError {
		#[serde(rename = "success")]
		Success {
			#[serde(flatten)]
			task: Task,
		},
		#[serde(rename = "error")]
		Error { message: String },
	}

	#[derive(Debug, Deserialize)]
	pub struct Submit {
		pub status: String,
		#[serde(default)]
		pub upid: Option<String>,
	}
	#[derive(Debug, Deserialize)]
	pub struct Ranklist {
		pub availablePages: u64,
		pub list: Vec<Ranks>,
		pub rank_and_score:Userrank
	}
	#[derive(Debug, Deserialize)]
	pub struct Ranks {
		pub country:String,
		pub user_handle:String,
		pub rank:u64,
		pub score:f64
	}
	#[derive(Debug, Deserialize)]
	pub struct Userrank {
		pub rank:String,
		pub score:String
	}
    #[derive(Debug, Deserialize)]
    pub struct Contest{
        pub contest_code:String,
        pub contest_name:String,
        pub contest_start_date_iso:String,
        pub contest_end_date_iso:String
    }

    #[derive(Debug, Deserialize)]
    pub struct ContestList {
        pub present_contests:Vec< Contest>,
        pub future_contests:Vec< Contest>
    }

	#[derive(Debug, Deserialize)]
	pub struct ContestTasksTask {
		pub code: String,
		pub category_name:String,
		// This field is sometimes returned as an integer, and sometimes as a string.
		// The pattern seems to be that zeroes are returned as integers, and anything else as
		// strings. I don't even want to know why on earth does the backend do that.
		#[serde(deserialize_with = "de_u64_or_u64str")]
		pub successful_submissions: u64,
	}
	#[derive(Debug, Deserialize)]
	pub struct ContestTasksTime {
		pub start: i64,
		pub current: i64,
		pub end: i64,
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
		// When this fields is an object, it contains a task symbol => task details sorted in no
		// particular order. However, it can also be an empty array - which means the contest has
		// not started or is a parent contest.
		#[serde(deserialize_with = "de_hash_map_or_empty_vec")]
		pub problems: Option<HashMap<String, ContestTasksTask>>,
		pub time: ContestTasksTime,
		#[serde(default)]
		pub child_contests: Option<HashMap<String, ContestTasksChildContest>>,
		#[serde(default)]
		pub user_rating_div: Option<ContestTasksUserRatingDiv>,
	}

	fn de_hash_map_or_empty_vec<'d, D: Deserializer<'d>>(
		d: D,
	) -> Result<Option<HashMap<String, ContestTasksTask>>, D::Error> {
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
