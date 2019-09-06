use crate::{ContestDetails, Language, Resource, Result, Submission, TaskDetails, URL};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
	any::Any, fmt::{self, Debug}, ops::Deref
};

type BoxedObject = Box<dyn AnyDebug+Send+Sync+'static>;

pub struct BoxedSession(BoxedObject);
pub struct BoxedContest(BoxedObject);
pub struct BoxedTask(BoxedObject);
pub struct BoxedCachedAuth(BoxedObject);

pub type BoxedURL = URL<BoxedContest, BoxedTask>;
pub type BoxedTaskURL = URL<!, BoxedTask>;
pub type BoxedContestURL = URL<BoxedContest, !>;
pub type BoxedResource = Resource<BoxedContest, BoxedTask>;
pub type BoxedContestDetails = ContestDetails<BoxedContest>;

impl crate::Backend for dyn DynamicBackend {
	type CachedAuth = BoxedCachedAuth;
	type Contest = BoxedContest;
	type Session = BoxedSession;
	type Task = BoxedTask;

	fn accepted_domains(&self) -> &'static [&'static str] {
		self.accepted_domainsx()
	}

	fn deconstruct_resource(&self, domain: &str, segments: &[&str]) -> Result<Resource<Self::Contest, Self::Task>> {
		self.deconstruct_resourcex(domain, segments)
	}

	fn connect(&self, client: Client, domain: &str) -> Self::Session {
		self.connectx(client, domain)
	}

	fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		self.auth_cachex(session.0.deref().as_any())
	}

	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		self.auth_deserializex(data)
	}

	fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		self.auth_loginx(session.0.deref().as_any(), username, password)
	}

	fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		self.auth_restorex(session.0.deref().as_any(), auth.0.deref().as_any())
	}

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		self.auth_serializex(auth.0.deref().as_any())
	}

	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest> {
		self.task_contestx(task.0.deref().as_any())
	}

	fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		self.task_detailsx(session.0.deref().as_any(), task.0.deref().as_any())
	}

	fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		self.task_languagesx(session.0.deref().as_any(), task.0.deref().as_any())
	}

	fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>> {
		self.task_submissionsx(session.0.deref().as_any(), task.0.deref().as_any())
	}

	fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		self.task_submitx(session.0.deref().as_any(), task.0.deref().as_any(), language, code)
	}

	fn task_url(&self, session: &Self::Session, task: &Self::Task) -> Result<String> {
		self.task_urlx(session.0.deref().as_any(), task.0.deref().as_any())
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		self.contest_idx(contest.0.deref().as_any())
	}

	fn contest_site_prefix(&self) -> &'static str {
		self.contest_site_prefixx()
	}

	fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		self.contest_tasksx(session.0.deref().as_any(), contest.0.deref().as_any())
	}

	fn contest_url(&self, contest: &Self::Contest) -> String {
		self.contest_urlx(contest.0.deref().as_any())
	}

	fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		self.contest_titlex(session.0.deref().as_any(), contest.0.deref().as_any())
	}

	fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		self.contestsx(session.0.deref().as_any())
	}

	fn name_short(&self) -> &'static str {
		self.name_shortx()
	}

	fn supports_contests(&self) -> bool {
		self.supports_contestsx()
	}
}

pub trait DynamicBackend: Send+Sync {
	fn accepted_domainsx(&self) -> &'static [&'static str];
	fn deconstruct_resourcex(&self, domain: &str, segments: &[&str]) -> Result<BoxedResource>;
	fn connectx(&self, client: Client, domain: &str) -> BoxedSession;
	fn auth_cachex(&self, session: &dyn Any) -> Result<Option<BoxedCachedAuth>>;
	fn auth_deserializex(&self, data: &str) -> Result<BoxedCachedAuth>;
	fn auth_loginx(&self, session: &dyn Any, username: &str, password: &str) -> Result<()>;
	fn auth_restorex(&self, session: &dyn Any, auth: &dyn Any) -> Result<()>;
	fn auth_serializex(&self, auth: &dyn Any) -> Result<String>;
	fn task_contestx(&self, task: &dyn Any) -> Option<BoxedContest>;
	fn task_detailsx(&self, session: &dyn Any, task: &dyn Any) -> Result<TaskDetails>;
	fn task_languagesx(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Language>>;
	fn task_submissionsx(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Submission>>;
	fn task_submitx(&self, session: &dyn Any, task: &dyn Any, language: &Language, code: &str) -> Result<String>;
	fn task_urlx(&self, session: &dyn Any, task: &dyn Any) -> Result<String>;
	fn contest_idx(&self, contest: &dyn Any) -> String;
	fn contest_site_prefixx(&self) -> &'static str;
	fn contest_tasksx(&self, session: &dyn Any, contest: &dyn Any) -> Result<Vec<BoxedTask>>;
	fn contest_urlx(&self, contest: &dyn Any) -> String;
	fn contest_titlex(&self, session: &dyn Any, contest: &dyn Any) -> Result<String>;
	fn contestsx(&self, session: &dyn Any) -> Result<Vec<BoxedContestDetails>>;
	fn name_shortx(&self) -> &'static str;
	fn supports_contestsx(&self) -> bool;
}

impl<T> DynamicBackend for T
where
	T: crate::Backend,
	<T as crate::Backend>::CachedAuth: Serialize+for<'d> Deserialize<'d>,
{
	fn accepted_domainsx(&self) -> &'static [&'static str] {
		T::accepted_domains(self)
	}

	fn deconstruct_resourcex(&self, domain: &str, segments: &[&str]) -> Result<BoxedResource> {
		Ok(match self.deconstruct_resource(domain, segments)? {
			Resource::Contest(c) => Resource::Contest(BoxedContest(Box::new(c))),
			Resource::Task(t) => Resource::Task(BoxedTask(Box::new(t))),
		})
	}

	fn connectx(&self, client: Client, domain: &str) -> BoxedSession {
		BoxedSession(Box::new(<T as crate::Backend>::connect(self, client, domain)))
	}

	fn auth_cachex(&self, session: &dyn Any) -> Result<Option<BoxedCachedAuth>> {
		Ok(<T as crate::Backend>::auth_cache(self, ujcast::<T::Session>(session))?.map(|c| BoxedCachedAuth(Box::new(c))))
	}

	fn auth_deserializex(&self, data: &str) -> Result<BoxedCachedAuth> {
		Ok(BoxedCachedAuth(Box::new(<T as crate::Backend>::auth_deserialize(self, data)?)))
	}

	fn auth_loginx(&self, session: &dyn Any, username: &str, password: &str) -> Result<()> {
		<T as crate::Backend>::auth_login(self, ujcast::<T::Session>(session), username, password)
	}

	fn auth_restorex(&self, session: &dyn Any, auth: &dyn Any) -> Result<()> {
		<T as crate::Backend>::auth_restore(self, ujcast::<T::Session>(session), ujcast::<T::CachedAuth>(auth))
	}

	fn auth_serializex(&self, auth: &dyn Any) -> Result<String> {
		<T as crate::Backend>::auth_serialize(self, ujcast::<T::CachedAuth>(auth))
	}

	fn task_contestx(&self, task: &dyn Any) -> Option<BoxedContest> {
		<T as crate::Backend>::task_contest(self, ujcast::<T::Task>(task)).map(|contest| BoxedContest(Box::new(contest)))
	}

	fn task_detailsx(&self, session: &dyn Any, task: &dyn Any) -> Result<TaskDetails> {
		<T as crate::Backend>::task_details(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task))
	}

	fn task_languagesx(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Language>> {
		<T as crate::Backend>::task_languages(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task))
	}

	fn task_submissionsx(&self, session: &dyn Any, task: &dyn Any) -> Result<Vec<Submission>> {
		<T as crate::Backend>::task_submissions(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task))
	}

	fn task_submitx(&self, session: &dyn Any, task: &dyn Any, language: &Language, code: &str) -> Result<String> {
		<T as crate::Backend>::task_submit(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task), language, code)
	}

	fn task_urlx(&self, session: &dyn Any, task: &dyn Any) -> Result<String> {
		<T as crate::Backend>::task_url(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task))
	}

	fn contest_idx(&self, contest: &dyn Any) -> String {
		<T as crate::Backend>::contest_id(self, ujcast::<T::Contest>(contest))
	}

	fn contest_site_prefixx(&self) -> &'static str {
		<T as crate::Backend>::contest_site_prefix(self)
	}

	fn contest_tasksx(&self, session: &dyn Any, contest: &dyn Any) -> Result<Vec<BoxedTask>> {
		Ok(<T as crate::Backend>::contest_tasks(self, ujcast::<T::Session>(session), ujcast::<T::Contest>(contest))?
			.into_iter()
			.map(|task| BoxedTask(Box::new(task)))
			.collect())
	}

	fn contest_urlx(&self, contest: &dyn Any) -> String {
		<T as crate::Backend>::contest_url(self, ujcast::<T::Contest>(contest))
	}

	fn contest_titlex(&self, session: &dyn Any, contest: &dyn Any) -> Result<String> {
		<T as crate::Backend>::contest_title(self, ujcast::<T::Session>(session), ujcast::<T::Contest>(contest))
	}

	fn contestsx(&self, session: &dyn Any) -> Result<Vec<BoxedContestDetails>> {
		Ok(<T as crate::Backend>::contests(self, ujcast::<T::Session>(session))?
			.into_iter()
			.map(|ContestDetails { id, title, start }| ContestDetails { id: BoxedContest(Box::new(id)), title, start })
			.collect())
	}

	fn name_shortx(&self) -> &'static str {
		<T as crate::Backend>::name_short(self)
	}

	fn supports_contestsx(&self) -> bool {
		<T as crate::Backend>::supports_contests(self)
	}
}

fn ujcast<T: 'static>(x: &dyn Any) -> &T {
	x.downcast_ref::<T>().expect("unijudge type error mixing incompatible backends")
}

pub trait AnyDebug: Any+Debug {
	fn as_any(&self) -> &dyn Any;
	fn as_debug(&self) -> &dyn Debug;
}
impl<T: Any+Debug> AnyDebug for T {
	fn as_any(&self) -> &dyn Any {
		self
	}

	fn as_debug(&self) -> &dyn Debug {
		self
	}
}

impl Debug for BoxedSession {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		Debug::fmt(self.0.deref(), f)
	}
}
impl Debug for BoxedContest {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		Debug::fmt(self.0.deref(), f)
	}
}
impl Debug for BoxedTask {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		Debug::fmt(self.0.deref(), f)
	}
}
impl Debug for BoxedCachedAuth {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		Debug::fmt(self.0.deref(), f)
	}
}
