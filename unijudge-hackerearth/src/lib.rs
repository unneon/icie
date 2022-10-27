#![feature(try_blocks)]
use markdown;
use html_escape;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Mutex};
use http::{
	StatusCode
};
use node_sys::console;
use unijudge::{
	chrono::{prelude::*,Duration},
	debris::{ Document, Find, Context}, http::{Client, Cookie}, json, log::{debug, error}, reqwest::{ Url,header::{CONTENT_TYPE, REFERER}}, ContestDetails, ContestTime, ErrorCode, Language, RejectionCause, Resource, Result, Statement, Submission, TaskDetails, Verdict
};
use unescape::unescape;
#[derive(Debug)]
pub struct HackerEarth;

#[derive(Debug, Clone)]
pub enum Contest {
	Practice(String,String,String),
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
	c_sess: [Cookie; 3],
}


#[async_trait(?Send)]
impl unijudge::Backend for HackerEarth {
	type CachedAuth = CachedAuth;
	type Contest = Contest;
	type Session = Session;
	type Task = Task;

	fn accepted_domains(&self) -> &'static [&'static str] {
		&["www.hackerearth.com"]
	}

	fn deconstruct_resource(&self, _domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>> {
		// There is no dedicated practice contest site, so we do not need to handle ["PRACTICE"].
		// This is the only place where PRACTICE doesn't work, it's treated as a normal contest
		// everywhere else.
		//https://www.hackerearth.com/challenges/competitive/march-circuits-22/
		//https://www.hackerearth.com/problem/algorithm/the-sum-of-squares-4e03818e-3dcd3383/
		match segments {
			["practice", maintopic,subtopic,topic,"practice-problems","algorithm",task] => Ok(Resource::Task(Task { contest: Contest::Practice((*maintopic).to_owned(),(*subtopic).to_owned(),(*topic).to_owned()), task: (*task).to_owned(),prefix:0})),
			["challenges","competitive",contest, "algorithm", task] => {
				Ok(Resource::Task(Task { contest: Contest::Normal((*contest).to_owned()), task: (*task).to_owned(),prefix:0  }))
			},
			["problem", "algorithm", task] => {
				Ok(Resource::Task(Task { contest: Contest::Normal("problem".to_owned()), task: (*task).to_owned(),prefix:0 }))
			}
			["challenges","competitive",contest] => Ok(Resource::Contest(Contest::Normal((*contest).to_owned()))),
			_ => Err(ErrorCode::WrongTaskUrl.into()),
		}
	}

	fn connect(&self, client: Client, _domain: &str) -> Self::Session {
		Session { client, username: Mutex::new(None) }
	}

	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let username = session.username.lock()?.clone();
		let autologin_login = session.client.cookie_get("lordoftherings")?;
		let autologin_hash = session.client.cookie_get("piratesofthecaribbean")?;
		let csrftoken = session.client.cookie_get("csrftoken")?;
		Ok(try { CachedAuth { username: username?, c_sess: [csrftoken?,autologin_login?,autologin_hash?] } })
	}
	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		unijudge::deserialize_auth(data)
	}
	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let mut user:String=username.to_string();
		if !username.contains("@") {
			user+="@gmail.com";
		}

        let resp = session
                       .client
                       .get(format!("https://www.hackerearth.com/login/").parse()?)
                       .send()
                       .await?
                       .text()
                       .await?;
        let re= regex::Regex::new("\'csrfmiddlewaretoken\' value=\'([_0-9A-Za-z-]+)\'").unwrap();
        
        if ! re.is_match(&resp) {
            return Err(ErrorCode::AccessDenied.into());
        }
        let cap =re.captures(&resp).unwrap();
        let form_build_id = cap.get(1).unwrap().as_str();

		/*let recsrf= regex::Regex::new("CSRF_COOKIE = \"([_0-9A-Za-z-]+)\"").unwrap();
        
        if ! recsrf.is_match(&resp) {
            return Err(ErrorCode::AccessDenied.into());
        }
        let csrf =recsrf.captures(&resp).unwrap().get(1).unwrap().as_str();
		
		session.client.cookie_set(csrf.clone(), "https://www.hackerearth.com")?;*/
		
        let resp2 = session
			.client
			.post(format!("https://www.hackerearth.com/login/").parse()?)
			.header(REFERER, "https://www.hackerearth.com/login/")
			.header(CONTENT_TYPE,"application/x-www-form-urlencoded")
			.form(&[
				("login", user.as_str()),
				("password", password),
				("csrfmiddlewaretoken", &form_build_id),
				("signin", "Log In")
			])
			.send()
			.await?;
        
		debug!("sent the login form");
		let url = resp2.url().clone();
			
        if url.as_str() == "https://www.hackerearth.com/login/" {
			return Err(ErrorCode::WrongCredentials.into());
        } else {
			debug!("OK logged in");
        }
        *session.username.lock()? = Some(user.to_owned());
        debug!("seemingly logged in");
        Ok(())
	}

	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		debug!("restoring an old session");
		*session.username.lock()? = Some(auth.username.clone());
		let [c1, c2, c3] = &auth.c_sess;
		session.client.cookie_set(c1.clone(), "https://www.hackerearth.com")?;
		session.client.cookie_set(c2.clone(), "https://www.hackerearth.com")?;
		session.client.cookie_set(c3.clone(), "https://www.hackerearth.com")?;
		Ok(())
	}

	

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		unijudge::serialize_auth(auth)
	}

	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest> {
		Some(task.contest.clone())
	}

	async fn rank_list(&self, session: &Self::Session, task: &Self::Task) -> Result<String>{
		return Ok("NA".to_string());
	}
	
	async fn remain_time(&self, session: &Self::Session, task: &Self::Task) -> Result<i64>{
		return Err(ErrorCode::AlienInvasion.into());
	}

	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails>{
		session.req_user()?;
		if ! task.contest.ispractice() {
			let url: Url =
			self.task_url(session,task)?
				.parse()?;
			console::debug(&format!("Task url:{}",self.task_url(session,task)?));
			let resp = Document::new(&session.client.get(url.clone()).send().await?.text().await?);
			//console::debug(&format!("Task output:{}",resp.find("body")?.text().string()));
			
			let title = resp.find(".problem-description")?.find_nth("div",0)?.text().string();
			let obj=api::Samples {
				sample_input:resp.find(".problem-description")?.find(".input-output-container")?.find_nth("pre",0)?.text().string(),
				sample_output:resp.find(".problem-description")?.find(".input-output-container")?.find_nth("pre",1)?.text().string(),
			};
			let descrip=format!("<strong> Example Input</strong>\n\n<pre><code>{}</code></pre>\n\n
					<strong> Example Output</strong>\n\n<pre><code>{}</code></pre>\n",obj.sample_input,obj.sample_output);
			
			let inout_html=format!("{:?}",resp.find(".problem-description")?.find(".input-output-container")?);
			let mut html_out=format!("{:?}",resp.find(".problem-description")?);
			let guide_html=format!("{:?}",resp.find(".problem-description")?.find(".problem-guidelines")?);
			html_out=html_out.replace(&inout_html,&descrip).replace(&guide_html,"");
			let statement= Some(Statement::HTML {
				html: 
				format!(r#"
				<html>
					<head>
					<script type="text/javascript" async src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.5/MathJax.js?config=TeX-AMS_SVG"></script>
					<link rel="stylesheet" href="https://static-fastly.hackerearth.com/static/hackathon/problem1.a3372113c1c6.css"/>
					<script type="text/x-mathjax-config">
					var options = {{
					messageStyle: "none",
					jax: ["input/TeX", "output/SVG", "output/HTML-CSS"],
					tex2jax: {{
							inlineMath: [['$$','$$'], ['\\(', '\\)']],
							displayMath: [['$$$', '$$$'], ['\\[', '\\]']],
							preview: "none"
						}},
					SVG: {{
							useGlobalCache: false
						}}
					}};

					// modify the options only in case of assessments, since for proxima-nova mathjax renders very small
					// text in chrome
					if (window.isProximaNova) {{
					options = {{
						messageStyle: "none",
						jax: ["input/TeX", "output/SVG", "output/HTML-CSS"],
						tex2jax: {{
							inlineMath: [['$$','$$'], ['\\(', '\\)']],
							displayMath: [['$$$', '$$$'], ['\\[', '\\]']],
							preview: "none"
						}},
						SVG: {{
							useGlobalCache: false,
							scale: MathJax.Hub.Browser.isChrome ? 175 : 100,
							minScaleAdjust: 100
						}},
						"HTML-CSS": {{
							minScaleAdjust: 100
						}},
						"CommonHTML": {{
							minScaleAdjust: 100
						}}
					}}
					}}

						MathJax.Hub.Config(options);
					</script>
					<script type="text/javascript" src="https://static-fastly.hackerearth.com/static/js/mathjax.3489d4a1e549.js" crossorigin="anonymous" ></script>
					<script type="text/javascript">
						window.addEventListener("load", function() {{
							MathJax.Hub.Queue(["Typeset", MathJax.Hub]);
						}});
					</script>
					<style>
						.markdown-body {{
							background-color: white;
							padding-bottom: 20px;
							color: black;
							font-size: large;
						}}
						.markdown-body pre {{
							background-color: #24292e;
						}}
						.problem-title{{
							font-size: 34px;
							font-weight: bold;
							text-decoration: underline;
						}}
					</style>
					</head>
					<body class="markdown-body">
					<div class="problem-page left problem-desc">
						{}
					</div>
					<body>
				</html>"#,html_escape::decode_html_entities(&html_out))
			});
			Ok(TaskDetails {
				id: task.task.clone(),
				title: unijudge::fmt_title(task.prefix)+&title,
				contest_id: task.contest.as_virt_symbol().to_owned(),
				site_short: "hackerearth".to_owned(),
				examples: Some(vec![unijudge::Example {
					input: obj.sample_input,
					output: obj.sample_output,
				}]),
				statement,
				url: self.task_url(session, task)?,
			})
		}else{
			let url: Url =
				format!("https://www.hackerearth.com/practice/api/problems/algorithm/{}", task.task)
					.parse()?;
			let resp2 = session.client.get(url.clone()).send().await?;
			let obj = json::from_resp::<api::Samples>(resp2).await?;
			
			let url: Url =
				self.task_url(session,task)?
					.parse()?;
			let resp = session.client.get(url.clone()).send().await?.text().await?;
			let re= regex::Regex::new("\"title\": \"([a-zA-Z ]+)\", \"description\": \"(.*)\", \"sample_explanation\": \"([^\"]+)\"").unwrap();
			
			if ! re.is_match(&resp) {
				return Err(ErrorCode::AccessDenied.into());
			}
			let cap =re.captures(&resp).unwrap();
			let title = cap.get(1).unwrap().as_str();
			let sample_exp=unescape(cap.get(3).unwrap().as_str()).unwrap();
			let desc= unescape(cap.get(2).unwrap().as_str()).unwrap();
			let descrip=format!("<h1> {}</h1>\n\n{}\n\n
					<strong> Example Input</strong>\n\n<pre><code>{}</code></pre>\n\n
					<strong> Example Output</strong>\n\n<pre><code>{}</code></pre>\n\n
					<strong> Sample Explanation</strong>\n\n{}", title, desc ,obj.sample_input,obj.sample_output,sample_exp);
			let html_out = html_escape::decode_html_entities(&descrip);
			
			//let  html_out=&markdown::to_html(&desc);
			//let  html_desc=format!("# {}\n\n{}\n\n ### Sample Explanation\n\n{}", title, desc,sample_exp);
			
			let statement= Some(Statement::HTML {
				html: format!(r#"
				<html>
					<head>
						<script type="text/x-mathjax-config">
							MathJax.Hub.Config({{
								jax: ["input/TeX", "output/SVG", "output/HTML-CSS"],
								tex2jax: {{inlineMath: [['$$','$$'], ['\\(', '\\)']],
								displayMath: [['$$$', '$$$'], ['\\[', '\\]']],
								preview: "none",
								}},
								SVG: {{
									useGlobalCache: false
								}}
							}});
						</script>
						<script src='https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.5/MathJax.js?config=TeX-AMS_SVG' async></script>
						<style>
						.markdown-body {{
							background-color: white;
							padding-bottom: 20px;
							color: black;
							font-size: large;
						}}
						.markdown-body pre {{
							background-color: #24292e;
						}}
					</style>
					</head>
					<body class="markdown-body">
						{}
					<body>
				</html>"#,html_out
			)});
			Ok(TaskDetails {
				id: task.task.clone(),
				title: title.to_string(),
				contest_id: task.contest.as_virt_symbol().to_owned(),
				site_short: "hackerearth".to_owned(),
				examples: Some(vec![unijudge::Example {
					input: obj.sample_input,
					output: obj.sample_output,
				}]),
				statement,
				url: self.task_url(session, task)?,
			})
		}
		
	}
	

	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		Ok(vec![ Language { id: "CPP17".to_owned(), name: "C++17".to_owned()}])
	}

	async fn task_submissions(&self, session: &Self::Session, task: &Self::Task)  -> Result<Vec<Submission>> {
		session.req_user()?;
		let submiturl= format!("https://www.hackerearth.com/practice/api/problems/algorithm/{}/user-submissions/",task.task);
		/*let mut opts = RequestInit::new();
		opts.method("GET");
		opts.mode(RequestMode::Cors);
	
		
		let request = Request::new_with_str_and_init(&submiturl, &opts).unwrap();
	
		request
			.headers()
			.set("Accept", "application/json, text/plain");
		request
			.headers()
			.append("X-Requested-With", "XMLHttpRequest, ");
		let window = web_sys::window().unwrap();
		let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
	
		// `resp_value` is a `Response` object.
		assert!(resp_value.is_instance_of::<Response>());
		let resp: Response = resp_value.dyn_into().unwrap();
		

		// Convert this other `Promise` into a rust `Future`.
		let json = JsFuture::from(resp.json().unwrap()).await.unwrap();
		console::debug(&format!("response {}",resp.status().to_string()));
		//let subs: Vec<api::Submissions> = json.into_serde().unwrap();
		let subs = json::from_str::<Vec<api::Submissions>>(&json.as_string().unwrap()).unwrap();*/
		/*let xhr=XmlHttpRequest::new().unwrap();
		xhr.open_with_async("GET",submiturl.as_str(),true).unwrap();
		xhr.send();
		let resp2=xhr.response_text().unwrap();
		if xhr.status().unwrap() != 200 {
			return Err(ErrorCode::AlienInvasion.into());
		}
		let subs = json::from_str::<Vec<api::Submissions>>(&resp2.unwrap()).unwrap();*/
		let resp2 = session
			.client
			.get(submiturl.parse()?)
			.header("X-Requested-With","XMLHttpRequest")
			.send()
			.await?;
		//	console::debug(&format!("Status {}",resp2.status().to_string()));
		if resp2.status() !=  StatusCode::OK {
			return Err(ErrorCode::AlienInvasion.into());
		}
		let resp_data=resp2.text().await?;
		//console::debug(&format!("Response {}",resp_data));
		let subs = json::from_str::<Vec<api::Submissions>>(&resp_data).unwrap();

		//
		subs.iter().map(|sub|{
			let id = sub.url[0..sub.url.len() - 1].split("/").last().unwrap().to_string();
			let verdict =  match sub.result.as_str() {
				"CE" => {
					Verdict::Rejected { cause: Some(RejectionCause::CompilationError), test: None }
				},
				"NA" => {
					Verdict::Pending { test: None }
				},
				"RE" => {
					Verdict::Rejected { cause: Some(RejectionCause::RuntimeError), test: None }
				},
				"WA" => {
					Verdict::Rejected { cause: Some(RejectionCause::WrongAnswer), test: None }
				},
				_ => Verdict::Scored { score: sub.score, max: Some(100.), cause: None, test: None }
			};
			Ok(Submission { id, verdict })
		}).collect()
		//Ok(vec![Submission { id:"1234".to_owned(), verdict:Verdict::Accepted }])
	}
	

	async fn task_submit(
		&self,
		session: &Self::Session,
		task: &Self::Task,
		language: &Language,
		code: &str,
	) ->Result<String>{
		session.req_user()?;
		//console::debug("Come here");
		//Ok(("1234".to_string()).to_owned())
		
		let url_hash=self.get_private_hash(task,session).await?;
		let submiturl= format!("https://www.hackerearth.com/submit/AJAX/");
		let dt: DateTime<Utc> = Utc::now();
		/*let mut opts = RequestInit::new();
		opts.method("GET");
		opts.mode(RequestMode::Cors);
	
		
		let request = Request::new_with_str_and_init(&submiturl, &opts).unwrap();
	
		request
			.headers()
			.set("Accept", "application/json, text/plain, ")
			.set("X-Requested-With","XMLHttpRequest");
		let window = web_sys::window().unwrap();
		let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
	
		// `resp_value` is a `Response` object.
		assert!(resp_value.is_instance_of::<Response>());
		let resp: Response = resp_value.dyn_into().unwrap();
		

		// Convert this other `Promise` into a rust `Future`.
		let json = JsFuture::from(resp.json().unwrap()).await.unwrap();
		let resp: api::Submit = json.into_serde().unwrap();
		debug!("OK submitted");
		//let resp = json::from_str::<api::Submit>(&json)?;
		*/
		/*let xhr=XmlHttpRequest::new().unwrap();
		let form=FormData::new().unwrap();
		form.append_with_str("problem_slug", &task.task);
		form.append_with_str("private_hash", &url_hash);
		form.append_with_str("source", &code);
		form.append_with_str("lang", &language.id);
		form.append_with_str("changeset_timestamp",&(dt.timestamp()*1000).to_string());
		form.append_with_str("problem_type","algorithm");
		xhr.open_with_async("POST",submiturl.as_str(),true).unwrap();
		xhr.send_with_opt_form_data(Some(&form));
		let resp2=xhr.response_text().unwrap();
		if xhr.status().unwrap() != 200 {
			return Err(ErrorCode::AlienInvasion.into());
		}
		let resp = json::from_str::<api::Submit>(&resp2.unwrap()).unwrap();*/
		let resp2 = session
			.client
			.post(submiturl.parse()?)
			.header("X-Requested-With","XMLHttpRequest")
			.header(CONTENT_TYPE,"application/x-www-form-urlencoded")
			.form(&[
				("problem_slug", task.task.clone()),
				("private_hash", url_hash),
				("source", code.to_string()),
				("lang", language.id.clone()),
				("changeset_timestamp",(dt.timestamp()*1000).to_string()),
				("problem_type","algorithm".to_string()),
			])
			.send()
			.await?;
			//console::debug(&format!("Status {}",resp2.status().to_string()));

		if resp2.status() !=  http::StatusCode::OK{
			//return Ok(("1234".to_string()).to_owned());
			return Err(ErrorCode::AlienInvasion.into());
		}
		let resp_data=resp2.text().await?;
		//console::debug(&format!("Response {}",resp_data));
		let resp = json::from_str::<api::Submit>(&resp_data)?;
		
		Ok((resp.submission_id.to_string()).to_owned())
	}

	fn submission_url(&self, _session: &Self::Session, _task: &Self::Task, id: &str) -> String {
		format!("https://www.hackerearth.com/submit/AJAX/")
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		contest.as_virt_symbol().to_owned()
	}

	fn contest_site_prefix(&self) -> &'static str {
		"Hackerearth"
	}

	async fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		Ok(self.contest_details_ex(session, contest).await?.tasks)
	}

	fn contest_url(&self, contest: &Self::Contest) -> String {
		match contest {
			Contest::Normal(contest) => format!("https://www.hackerearth.com/challenges/competitive/{}", contest),
			Contest::Practice(main,sub,topic) => "https://www.codechef.com/problems/school".to_owned(),
		}
	}
	fn task_url(&self, _session: &Self::Session, task: &Self::Task) -> Result<String> {
		Ok(format!("https://www.hackerearth.com/{}/algorithm/{}", task.contest.prefix(), task.task))
	}
	async fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		Ok(self.contest_id(contest))
	}

	async fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		let doc = Document::new(
			&session.client.get("https://www.hackerearth.com/challenges/competitive/".parse()?).send().await?.text().await?,
			//&session.client.get("https://www.hackerearth.com/challenges/hackathon/".parse()?).send().await?.text().await?,
		);
		// CodeChef does not separate ongoing contests and permanent contests, so we only select the
		// upcoming ones. This is irritating, but I would like to add some general heuristics for
		// all sites later. Doing this only for CodeChef wouldn't make sense because it's better to
		// also handle SPOJ and sio2 at the same time.
		//let contests = doc.find("#primary-content > .content-wrapper")?;
		let table_ongoing = doc.find(".ongoing")?;
		let table_upcoming = doc.find(".upcoming")?;
		let rows_ongoing = table_ongoing.find_all(".challenge-card-modern").map(|row| (row, true));
		let rows_upcoming = table_upcoming.find_all(".challenge-card-modern").map(|row| (row, false));
		rows_upcoming
			.chain(rows_ongoing)
			.map(|(row, is_ongoing)| {

				let url = row.find_nth("a",0)?.attr("href")?.string();
				let id = Contest::Normal(url[0..url.len() - 1].split("/").last().unwrap().to_string());
				let title = row.find(".challenge-content")?.find_nth("div", 1)?.find("span")?.text().string();
				let utc: DateTime<Utc> = Utc::now();
				let local: DateTime<Local> = Local::now();
				let dt =if is_ongoing {
					//let endtime =local+ Duration::seconds(1_000);
					//endtime.with_timezone(endtime.offset())
					utc+ Duration::seconds(1_000)
				}
				else {
					row.find(".challenge-content")?
					.find(".challenge-desc")?
					.find_nth("div", 1)?
					.text()
					.map(|start_time| {
						let start =start_time.to_owned()+" "+&local.year().to_string();
						Utc.datetime_from_str(&start,"%b %e, %I:%M %p %Z %Y")
					})?
				};
				let datetime= local.offset().from_local_datetime(&dt.naive_utc()).unwrap();
				let time = if is_ongoing {
					ContestTime::Ongoing { finish: datetime }
				} else {
					ContestTime::Upcoming { start: datetime }
				};
				Ok(ContestDetails { id, title, time })
			})
			.collect()
            
       
	}

	fn name_short(&self) -> &'static str {
		"hackerearth"
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

impl HackerEarth {


	async fn contest_details_ex(&self, session: &Session, contest: &Contest) -> Result<ContestDetailsEx> {
		session.req_user()?;
		//let  taks: Vec<Task> =Vec::new();
			// CodeChef does not sort problems by estimated difficulty, contrary to
			// Codeforces/AtCoder. Instead, it sorts them by submission count. This is problematic
			// when contest begin, as all problems have a submit count of 0. But since this naive
			// sort is as good as what you get with a browser, let's just ignore this.
			//tasks.sort_unstable_by_key(|task| u64::max_value() - task.1);
			//Ok(ContestDetailsEx { title: "Title ".to_owned(), tasks: taks })
			let resp=session.client.get(format!("https://www.hackerearth.com/challenges/competitive/{}/problems/",contest.as_virt_symbol()).parse()?).send().await?.text().await?;
			let doc = Document::new(
				&session.client.get(format!("https://www.hackerearth.com/challenges/competitive/{}/problems/",contest.as_virt_symbol()).parse()?).send().await?.text().await?,
				//&session.client.get("https://www.hackerearth.com/challenges/hackathon/".parse()?).send().await?.text().await?,
			);
			let tasks:Vec<Result<Task>>= doc.find("#problems-list-table")?
			.find_all("tbody > tr")
			.enumerate()
			.filter(|(i,row)| {
				let row_class=row.attr("class").unwrap().string();
				row_class != "empty-tr" && row_class!= "disabled-problem" 
			})
    		.map(|(i,row)| {
				//let row_class=row.attr("class")?.string();
				//if row_class == "empty-tr" || row_class== "disabled-problem" {return Some(None);}
				//let ind=if i==0 || i==4 || i==7 || i==10 {3} else {2};
				let name = row.find_nth("a",1)?.attr("id")?.string().replace("-accuracy", "");
				Ok(Task { contest: contest.clone(), task: name, prefix:i as i64})
			}).collect();
			console::debug(&format!("Taks Count:{}",tasks.len()));
			Ok(ContestDetailsEx { title: contest.as_virt_symbol().to_string(), tasks: tasks.into_iter().map(|kv| kv.unwrap()).collect() })
		
	}

	

	async fn get_private_hash(&self, task: &Task, session: &Session) -> Result<String> {
		let url: Url =
				format!("https://www.hackerearth.com/{}/algorithm/{}", task.contest.prefix(), task.task)
				.parse()?;
		let resp = session.client.get(url.clone()).send().await?.text().await?;
		
		let re= if task.contest.ispractice(){ regex::Regex::new("\"private_url_hash\": \"([^\"]+)\"").unwrap()}
				else { regex::Regex::new("\"PROBLEM_DATA\": [{]\"([^\"]+)\":").unwrap()	};
        
        if ! re.is_match(&resp) {
            return Err(ErrorCode::AccessDenied.into());
        }
        let cap =re.captures(&resp).unwrap();
        let title = cap.get(1).unwrap().as_str();
		Ok(title.to_string())
	}
	/*async fn get_max_score(&self, task: &Task, session: &Session) -> Result<String> {
		let url: Url =
				format!("https://www.hackerearth.com/{}/algorithm/{}", task.contest.prefix(), task.task)
				.parse()?;
		let resp = session.client.get(url.clone()).send().await?.text().await?;
		let re= regex::Regex::new("\"private_url_hash\": \"([^\"]+)\"").unwrap();
        
        if ! re.is_match(&resp) {
            return Err(ErrorCode::AccessDenied.into());
        }
        let cap =re.captures(&resp).unwrap();
        let title = cap.get(1).unwrap().as_str();
		Ok(title.to_string())
	}*/

	

	/// Queries "active" submit URL. In CodeChef, the submit URL parameters can be different from
	/// the task URL parameters for various reasons, e.g. after a contest ends, or when submitting a
	/// problem from a different division. This function performs an additional HTTP request to take
	/// this into account.
    
	async fn active_submit_url(&self, task: &Task, session: &Session) -> Result<Url> {
		let url = format!("https://www.hackerearth.com/submit/AJAX/");
        Ok(url.parse()?)
	}
	
	

	
}
impl Session {
	fn req_user(&self) -> Result<String> {
		let username = self.username.lock()?.clone().ok_or(ErrorCode::AccessDenied)?;
		Ok(username)
	}
}
impl Contest {
	fn as_virt_symbol(&self) -> String {
		match self {
			Contest::Normal(name) =>  format!("{}", name),
			Contest::Practice(main,sub,topic) => format!("{}-{}-{}",main,sub,topic),
		}
	}

	fn prefix(&self) -> String {
		match self {
			Contest::Normal(name) => format!("challenges/competitive/{}/", name),
			Contest::Practice(main,sub,topic) =>format!("practice/{}/{}/{}/practice-problems/",main,sub,topic),
		}
	}
	fn ispractice(&self) -> bool {
		match self {
			Contest::Normal(_) => return false,
			Contest::Practice(_,_,_) =>return true,
		}
	}
}

mod api {
	#[derive(Debug, Deserialize)]
	pub struct ContestTasksUser {
		pub username: String,
	}
	#[derive(Debug, Deserialize)]
	pub struct Samples {
		pub sample_input: String,
		pub sample_output: String,
	}
	#[derive(Debug, Deserialize)]
	pub struct Submit {
		pub submission_id: u64,
	}
	#[derive(Debug, Deserialize)]
	pub struct Submissions {
		pub score: f64,
		pub result: String,
		pub url: String,
	}
	use serde::{
		de::{self, MapAccess, SeqAccess, Unexpected}, __private::PhantomData, Deserialize, Deserializer
	};
	use std::{collections::HashMap, fmt, hash::Hash};

	
	

	
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
