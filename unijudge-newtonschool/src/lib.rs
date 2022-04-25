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
	debris::{ Document, Find}, http::{Client, Cookie}, json, log::{debug, error}, reqwest::{ Url,header::{CONTENT_TYPE, REFERER}}, ContestDetails, ContestTime, ErrorCode, Language, RejectionCause, Resource, Result, Statement, Submission, TaskDetails, Verdict
};
use unescape::unescape;
#[derive(Debug)]
pub struct NewtonSchool;

#[derive(Debug, Clone)]
pub enum Contest {
	Practice(),
	Normal(String,String),
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
	token:Mutex<Option<String>>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedAuth {
	username: String,
	token:String
}


#[async_trait(?Send)]
impl unijudge::Backend for NewtonSchool {
	type CachedAuth = CachedAuth;
	type Contest = Contest;
	type Session = Session;
	type Task = Task;

	fn accepted_domains(&self) -> &'static [&'static str] {
		&["my.newtonschool.co"]
	}

	fn deconstruct_resource(&self, _domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>> {
		// There is no dedicated practice contest site, so we do not need to handle ["PRACTICE"].
		// This is the only place where PRACTICE doesn't work, it's treated as a normal contest
		// everywhere else.
		//https://my.newtonschool.co/course/jtrr9p6u9dat/assignment/rkp1ithkwgfx/dashboard/?tab=questions
		//https://my.newtonschool.co/playground/code/e1wf6yweovkl/
		match segments {
			["course",main,"assignment", course,"dashboard"] => {
				Ok(Resource::Contest(Contest::Normal((*main).to_owned(),(*course).to_owned())))
			},
			["course",main,"assignment", course] => {
				Ok(Resource::Contest(Contest::Normal((*main).to_owned(),(*course).to_owned())))
			},
			["course",main,"assignment","h", course,"question",task] => {
				Ok(Resource::Task(Task { contest:Contest::Normal((*main).to_owned(),(*course).to_owned()), task: (*task).to_owned() }))
			},
			["playground", "code", task] => {
				Ok(Resource::Task(Task { contest: Contest::Practice(), task: (*task).to_owned() }))
			}
			_ => Err(ErrorCode::WrongTaskUrl.into()),
		}
	}

	fn connect(&self, client: Client, _domain: &str) -> Self::Session {
		Session { client, username: Mutex::new(None),token:Mutex::new(None) }
	}

	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let username = session.username.lock()?.clone();
		let token = session.token.lock()?.clone();
		Ok(try { CachedAuth { username: username?, token: token? } })
	}
	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		unijudge::deserialize_auth(data)
	}
	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		//console::debug("Come here");
		let mut user:String=username.to_string();
		if !username.contains("@") {
			user+="@gmail.com";
		}
		let resp_raw = session
		.client
		.post(format!("https://my.newtonschool.co/api/v1/user/login/").parse()?)
		.form(&[
			("backend", "email".to_owned()),
			("client_id", "1I4rv6bekM8zAZfjwf4pxC6i4BFgP2M8hqWdvY7M".to_owned()),
			("client_secret","f3zqwMUUQ5VJOFQAuoAWlDJlfjauOTHNRy8djit9XgjjQcdrQn3WYj6k5qGPvpZDGNuKxacOvaSUddQ6fX9GOjVWWG2GKrUHQQIiXUE1rmveA1NihaWUavL4uqR6xRo9".to_owned()),
			("email",user.to_string()),
			("password",password.to_string())
	  	])
		.send()
		.await?;
		//console::debug(&format!("Status {}",resp_raw.status().to_string()));
		if resp_raw.status() !=  StatusCode::OK {
			return Err(ErrorCode::WrongCredentials.into());
		}
		
		let response=resp_raw.text().await?;
		let resp = json::from_str::<api::LoginForm>(&response)?;  
		//console::debug(&format!("Response {}",response));
		*session.username.lock()? = Some(username.to_owned());
		*session.token.lock()? = Some(resp.access_token.to_owned());
        debug!("seemingly logged in");
        Ok(())
	}

	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		debug!("restoring an old session");
		*session.username.lock()? = Some(auth.username.clone());
		*session.token.lock()? = Some(auth.token.clone());
		Ok(())
	}

	

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		unijudge::serialize_auth(auth)
	}

	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest> {
		Some(task.contest.clone())
	}

	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails>{
		session.req_token()?;

	/*	let resp_raw = session
		.client
		.get(self.task_url(session, task)?.parse()?)
		.send()
		.await?
		.text()
		.await?;
		let resp = json::from_str::<api::Assign>(&resp_raw)?;  
*/
		let resp_raw = session
		.client
		.get(format!("https://my.newtonschool.co/api/v1/playground/coding/h/{}/",task.task).parse()?)
		.header("Authorization","Bearer ".to_owned()+&session.req_token().unwrap())
		.send()
		.await?
		.text()
		.await?;
		let resp2 = json::from_str::<api::TaskDetails>(&resp_raw)?;  
		let re= regex::Regex::new(r"Input( \d*)?:\r\n((?s).*?)(\r\n)+Sample Output( \d*)?:\r\n((?s).*?)((\r\n)+Sample|$)").unwrap();
		
		let mut cases =Vec::new();
		for cap in re.captures_iter(&resp2.assignment_question.example) {
			//println!("I{} O{}\n\n\n", &cap[2], &cap[4]);
			cases.push(unijudge::Example {
				input: cap[2].to_string(),
				output: cap[5].to_string(),
			});
		}
		let title=resp2.assignment_question.question_title.to_owned();
		let statement = Some(self.prepare_statement( resp2.assignment_question));
		Ok(TaskDetails {
			id: task.task.clone(),
			title:title ,
			contest_id: task.contest.as_virt_symbol().to_owned(),
			site_short: "newtonschool".to_owned(),
			examples: Some(cases),
			statement,
			url: self.task_url(session, task)?,
		})
		
	}
	

	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		Ok(vec![ Language { id: "54".to_owned(), name: "C++ (GCC 9.2.0)".to_owned()}])
	}

	async fn task_submissions(&self, session: &Self::Session, task: &Self::Task)  -> Result<Vec<Submission>> {
		session.req_token()?;
		let resp_raw = session
		.client
		.get(format!("https://my.newtonschool.co/api/v1/playground/coding/h/{}/all_submission/",task.task).parse()?)
		.header("Authorization","Bearer ".to_owned()+&session.req_token().unwrap())
		.send()
		.await?
		.text()
		.await?;
		let resp2 = json::from_str::<Vec<api::Submissions>>(&resp_raw)?;  
		if resp2[0].current_status!=3 {
			Ok(vec![Submission { id:"ID_NA".to_owned(), verdict:Verdict::Pending { test: None } }])	
		}else if resp2[0].all_test_cases_passing {
			Ok(vec![Submission { id:"ID_NA".to_owned(), verdict:Verdict::Accepted }])
		}
		else if resp2[0].wrong_submission {
			Ok(vec![Submission { id:"ID_NA".to_owned(), verdict:Verdict::Rejected { cause: Some(RejectionCause::WrongAnswer), test: None }}])
		}else {
			let resp_raw2 = session
			.client
			.get(format!("https://my.newtonschool.co/api/v1/playground/coding/h/{}/latest_submission/?username={}",task.task,session.req_user().unwrap()).parse()?)
			.header("Authorization","Bearer ".to_owned()+&session.req_token().unwrap())
			.send()
			.await?
			.text()
			.await?;
			let resp = json::from_str::<api::Testcases>(&resp_raw2)?; 
			
			Ok(vec![Submission { id:"ID_NA".to_owned(), verdict:Verdict::Scored { score:resp2[0].number_of_test_cases_passing as f64, max: Some(resp.submission_test_case_mappings.len() as f64), cause: None, test: None }}])
		}
	}
	

	async fn task_submit(
		&self,
		session: &Self::Session,
		task: &Self::Task,
		language: &Language,
		code: &str,
	) ->Result<String>{
		// 
		session.req_token()?;
		let resp_raw = session
		.client
		.patch(self.submission_url(session,task,"").parse()?)
		.header("Authorization","Bearer ".to_owned()+&session.req_token().unwrap())
		.form(&[
			("autoSave", "true".to_owned()),
			("hash", task.task.to_owned()),
			("language_id",language.id.to_owned()),
			("source_code",code.to_string()),
			("standard_input","".to_string())
	  	])
		.send()
		.await?
		.text()
		.await?;
		Ok(("ID_NA".to_string()).to_owned())
		//let resp2 = json::from_str::<api::TaskDetails>(&resp_raw)?;  
	}

	fn submission_url(&self, _session: &Self::Session, _task: &Self::Task, id: &str) -> String {
		format!("https://my.newtonschool.co/api/v1/playground/coding/h/{}/?run_hidden_test_cases=true",_task.task)
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		contest.as_virt_symbol().to_owned()
	}

	fn contest_site_prefix(&self) -> &'static str {
		"NewtonSchool"
	}

	async fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		Ok(self.contest_details_ex(session, contest).await?.tasks)
	}
	fn contest_url(&self, contest: &Self::Contest) -> String {
		match contest {
			Contest::Normal(course,ass) => format!("https://my.newtonschool.co/course/{}/assignment/{}/dashboard/", course,ass),
			Contest::Practice() => "https://www.codechef.com/problems/school".to_owned(),
		}
	}
	
	fn task_url(&self, _session: &Self::Session, task: &Self::Task) -> Result<String> {
		Ok(format!("https://my.newtonschool.co/course/{}/question/{}", task.contest.prefix(), task.task))
	}
	async fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		Ok(self.contest_details_ex(session, contest).await?.title)
	}

	async fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		
		 
		  let resp_raw = session
            .client
            .get(format!("https://my.newtonschool.co/api/v1/contests/list/?offset=0&limit=100&past=false").parse()?)
            .send()
            .await?
            .text()
            .await?;
        let resp = json::from_str::<api::ContestList>(&resp_raw)?; 

       resp.results.iter().map(|row|{
		   
			let id = Contest::Normal(row.hash.clone(),row.filtering_assignment.hash.clone());
			let title = row.title.clone();
			let naive = NaiveDateTime::from_timestamp(row.start_timestamp/1000, 0);
    		let dt: DateTime<Utc> = DateTime::from_utc(naive, Utc);
			let local: DateTime<Local> = Local::now();
			let datetime= local.offset().from_local_datetime(&dt.naive_utc()).unwrap();
			let time = ContestTime::Upcoming { start: datetime };
			Ok(ContestDetails { id, title, time })
	   }).collect()
	}

	fn name_short(&self) -> &'static str {
		"newtonschool"
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

impl NewtonSchool {

	async fn contest_details_ex(&self, session: &Session, contest: &Contest) -> Result<ContestDetailsEx> {
		//console::debug("Come here1");
		let  taks: Vec<Task> =Vec::new();
			// CodeChef does not sort problems by estimated difficulty, contrary to
			// Codeforces/AtCoder. Instead, it sorts them by submission count. This is problematic
			// when contest begin, as all problems have a submit count of 0. But since this naive
			// sort is as good as what you get with a browser, let's just ignore this.
			//tasks.sort_unstable_by_key(|task| u64::max_value() - task.1);
			//Ok(ContestDetailsEx { title: "Title ".to_owned(), tasks: taks })
			session.req_token()?;
			let resp_raw = session
            .client
            .get(format!("https://my.newtonschool.co/api/v1/course/h/{}/details/public/", contest.prefix()).parse()?)
            .send()
            .await?
            .text()
            .await?;
        let mut resp = json::from_str::<api::TaskList>(&resp_raw)?;  
		for i in &mut resp.assignment_questions {
			// iterate mutably
			let t: &mut api::Tasks = i; // elements are mutable pointers
			let resp_raw = session
			.client
			.get(format!("https://my.newtonschool.co/api/v1/course/h/{}/question/h/{}/details/", contest.prefix(),t.hash).parse()?)
			.header("Authorization","Bearer ".to_owned()+&session.req_token().unwrap())
			.send()
			.await?
			.text()
			.await?;
			let resp = json::from_str::<api::Assign>(&resp_raw)?; 
			t.hash=resp.hash;
		}
		 
        let tasks=resp.assignment_questions.iter().map(|row|{
				Task { contest: contest.clone(), task: row.hash.to_owned() }
	   }).collect();
	   Ok(ContestDetailsEx { title: resp.title, tasks: tasks })
	}

	fn prepare_statement(&self, compont: api::TaskDetail) -> Statement {
		//let mut html = String::new();
		// CodeChef statements are pretty wild. They seem to follow some structure and use Markdown,
		// but it's not true. They mix Markdown and HTML very liberally, and their Markdown
		// implementation is not standard-compliant. So e.g. you can have sections with "###Example
		// input", which CommonMark parsers ignore. Fortunately, we can ignore the HTML because
		// Markdown permits it. Also, we add a title so that the statement looks better.
        let re= regex::Regex::new(r"Input( \d*)?:\r\n((?s).*?)(\r\n)+Sample Output( \d*)?:\r\n((?s).*?)((\r\n)+Sample|$)").unwrap();
		let mut casestr = "".to_owned();
		for cap in re.captures_iter(&compont.example) {
			casestr.push_str("\r\n\n###Example Input\r\n```\r\n");casestr.push_str( &cap[2] );casestr.push_str("\t\r\n```\r\n\r\n");
			casestr.push_str("\r\n\n###Example Output\r\n```\r\n");casestr.push_str( &cap[5] );casestr.push_str( "\t\r\n```\r\n\r\n");
			//println!("I{} O{}\n\n\n", &cap[2], &cap[4]);
		}
        let inpf= "\r\n\n###Input Format\r\n".to_owned() + &compont.input;
        let outf= "\r\n\n###Output Format\r\n".to_owned()+ &compont.output;
        
        let text = compont.question_text + &inpf  + &outf + &casestr;

        let  html_out=markdown::to_html(&format!("# {}\n\n{}", compont.question_title, text.replace("###", "### ")));

		Statement::HTML {
			html: format!(
				r#"
<html>
	<head>
		
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
</html>"#,
				html_out
			),
		}
	}

	
	

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
		let re= regex::Regex::new(r"(.*)@(.*)").unwrap();
		let cap =re.captures(&username).unwrap();
        let id = cap.get(1).unwrap().as_str();
		Ok(id.to_string())
	}
	fn req_token(&self) -> Result<String> {
		let token = self.token.lock()?.clone().ok_or(ErrorCode::AccessDenied)?;
		Ok(token)
	}
}
impl Contest {
	fn as_virt_symbol(&self) -> String {
		match self {
			Contest::Normal(course,assi) =>  format!("{}-{}", course,assi),
			Contest::Practice() => format!("Pratice"),
		}
	}

	fn prefix(&self) -> String {
		match self {
			Contest::Normal(course,assi) =>  format!("{}/assignment/h/{}", course,assi),
			Contest::Practice() =>format!("practice"),
		}
	}
}

mod api {
	#[derive(Debug, Deserialize)]
    pub struct TaskList {
        pub assignment_questions:Vec< Tasks>,
		pub title:String
    }
	#[derive(Debug, Deserialize)]
    pub struct Tasks {
        pub question_title:String,
		 pub hash:String,
    }
	#[derive(Debug, Deserialize)]
    pub struct TaskDetails{
		pub assignment_question:TaskDetail
	}
	#[derive(Debug, Deserialize)]
    pub struct TaskDetail{
		pub assignment_question_language_mappings:Vec<Languages>,
		pub example:String,
		pub input:String,
		pub output:String,
		pub question_text:String,
		pub question_title:String
	}
	#[derive(Debug, Deserialize)]
    pub struct Languages {
        pub language_id:i64,
		pub language_text:String
    }
	#[derive(Debug, Deserialize)]
    pub struct ContestList {
        pub results:Vec<Contest>
    }
	#[derive(Debug, Deserialize)]
    pub struct Contest {
        pub title:String,
		pub hash:String,
		pub start_timestamp:i64,
		pub filtering_assignment: Assign
    }
	#[derive(Debug, Deserialize)]
    pub struct Assign{
		pub hash :String
	}
	
	#[derive(Debug, Deserialize)]
    pub struct Submissions{
		pub all_test_cases_passing :bool,
		pub current_status:i64,
		pub number_of_test_cases_passing:i64,
		pub wrong_submission:bool
	}
	
	#[derive(Debug, Deserialize)]
    pub struct LoginForm{
		pub access_token :String
	}

	#[derive(Debug, Deserialize)]
    pub struct Testcases{
		pub submission_test_case_mappings :Vec<mappings>
	}
	#[derive(Debug, Deserialize)]
    pub struct mappings{
		pub status :i64,
		pub status_text:String
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
