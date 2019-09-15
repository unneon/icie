use crate::{http::Client, ContestDetails, Language, Resource, Result, Submission, TaskDetails, URL};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
	any::Any, fmt::{self, Debug}, ops::Deref
};

type BoxedObject = Box<dyn AnyDebug>;

pub struct BoxedSession(BoxedObject);
pub struct BoxedContest(BoxedObject);
pub struct BoxedTask(BoxedObject);
pub struct BoxedCachedAuth(BoxedObject);

pub type BoxedURL = URL<BoxedContest, BoxedTask>;
pub type BoxedTaskURL = URL<!, BoxedTask>;
pub type BoxedContestURL = URL<BoxedContest, !>;
pub type BoxedResource = Resource<BoxedContest, BoxedTask>;
pub type BoxedContestDetails = ContestDetails<BoxedContest>;

#[async_trait]
impl crate::Backend for (dyn DynamicBackend+'static) {
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

	async fn auth_cache(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		self.auth_cachex(session.0.deref()).await
	}

	fn auth_deserialize(&self, data: &str) -> Result<Self::CachedAuth> {
		self.auth_deserializex(data)
	}

	async fn auth_login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		self.auth_loginx(session.0.deref(), username, password).await
	}

	async fn auth_restore(&self, session: &Self::Session, auth: &Self::CachedAuth) -> Result<()> {
		self.auth_restorex(session.0.deref(), auth.0.deref()).await
	}

	fn auth_serialize(&self, auth: &Self::CachedAuth) -> Result<String> {
		self.auth_serializex(auth.0.deref())
	}

	fn task_contest(&self, task: &Self::Task) -> Option<Self::Contest> {
		self.task_contestx(task.0.deref())
	}

	async fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		self.task_detailsx(session.0.deref(), task.0.deref()).await
	}

	async fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		self.task_languagesx(session.0.deref(), task.0.deref()).await
	}

	async fn task_submissions(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Submission>> {
		self.task_submissionsx(session.0.deref(), task.0.deref()).await
	}

	async fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		self.task_submitx(session.0.deref(), task.0.deref(), language, code).await
	}

	fn task_url(&self, session: &Self::Session, task: &Self::Task) -> Result<String> {
		self.task_urlx(session.0.deref(), task.0.deref())
	}

	fn contest_id(&self, contest: &Self::Contest) -> String {
		self.contest_idx(contest.0.deref())
	}

	fn contest_site_prefix(&self) -> &'static str {
		self.contest_site_prefixx()
	}

	async fn contest_tasks(&self, session: &Self::Session, contest: &Self::Contest) -> Result<Vec<Self::Task>> {
		self.contest_tasksx(session.0.deref(), contest.0.deref()).await
	}

	fn contest_url(&self, contest: &Self::Contest) -> String {
		self.contest_urlx(contest.0.deref())
	}

	async fn contest_title(&self, session: &Self::Session, contest: &Self::Contest) -> Result<String> {
		self.contest_titlex(session.0.deref(), contest.0.deref()).await
	}

	async fn contests(&self, session: &Self::Session) -> Result<Vec<ContestDetails<Self::Contest>>> {
		self.contestsx(session.0.deref()).await
	}

	fn name_short(&self) -> &'static str {
		self.name_shortx()
	}

	fn supports_contests(&self) -> bool {
		self.supports_contestsx()
	}
}

#[async_trait]
pub trait DynamicBackend: Send+Sync {
	fn accepted_domainsx(&self) -> &'static [&'static str];
	fn deconstruct_resourcex(&self, domain: &str, segments: &[&str]) -> Result<BoxedResource>;
	fn connectx(&self, client: Client, domain: &str) -> BoxedSession;
	async fn auth_cachex(&self, session: &dyn AnyDebug) -> Result<Option<BoxedCachedAuth>>;
	fn auth_deserializex(&self, data: &str) -> Result<BoxedCachedAuth>;
	async fn auth_loginx(&self, session: &dyn AnyDebug, username: &str, password: &str) -> Result<()>;
	async fn auth_restorex(&self, session: &dyn AnyDebug, auth: &dyn AnyDebug) -> Result<()>;
	fn auth_serializex(&self, auth: &dyn AnyDebug) -> Result<String>;
	fn task_contestx(&self, task: &dyn AnyDebug) -> Option<BoxedContest>;
	async fn task_detailsx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<TaskDetails>;
	async fn task_languagesx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<Vec<Language>>;
	async fn task_submissionsx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<Vec<Submission>>;
	async fn task_submitx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug, language: &Language, code: &str) -> Result<String>;
	fn task_urlx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<String>;
	fn contest_idx(&self, contest: &dyn AnyDebug) -> String;
	fn contest_site_prefixx(&self) -> &'static str;
	async fn contest_tasksx(&self, session: &dyn AnyDebug, contest: &dyn AnyDebug) -> Result<Vec<BoxedTask>>;
	fn contest_urlx(&self, contest: &dyn AnyDebug) -> String;
	async fn contest_titlex(&self, session: &dyn AnyDebug, contest: &dyn AnyDebug) -> Result<String>;
	async fn contestsx(&self, session: &dyn AnyDebug) -> Result<Vec<BoxedContestDetails>>;
	fn name_shortx(&self) -> &'static str;
	fn supports_contestsx(&self) -> bool;
}

#[async_trait]
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

	async fn auth_cachex(&self, session: &dyn AnyDebug) -> Result<Option<BoxedCachedAuth>> {
		Ok(<T as crate::Backend>::auth_cache(self, ujcast::<T::Session>(session)).await?.map(|c| BoxedCachedAuth(Box::new(c))))
	}

	fn auth_deserializex(&self, data: &str) -> Result<BoxedCachedAuth> {
		Ok(BoxedCachedAuth(Box::new(<T as crate::Backend>::auth_deserialize(self, data)?)))
	}

	async fn auth_loginx(&self, session: &dyn AnyDebug, username: &str, password: &str) -> Result<()> {
		<T as crate::Backend>::auth_login(self, ujcast::<T::Session>(session), username, password).await
	}

	async fn auth_restorex(&self, session: &dyn AnyDebug, auth: &dyn AnyDebug) -> Result<()> {
		<T as crate::Backend>::auth_restore(self, ujcast::<T::Session>(session), ujcast::<T::CachedAuth>(auth)).await
	}

	fn auth_serializex(&self, auth: &dyn AnyDebug) -> Result<String> {
		<T as crate::Backend>::auth_serialize(self, ujcast::<T::CachedAuth>(auth))
	}

	fn task_contestx(&self, task: &dyn AnyDebug) -> Option<BoxedContest> {
		<T as crate::Backend>::task_contest(self, ujcast::<T::Task>(task)).map(|contest| BoxedContest(Box::new(contest)))
	}

	async fn task_detailsx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<TaskDetails> {
		<T as crate::Backend>::task_details(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task)).await
	}

	async fn task_languagesx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<Vec<Language>> {
		<T as crate::Backend>::task_languages(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task)).await
	}

	async fn task_submissionsx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<Vec<Submission>> {
		<T as crate::Backend>::task_submissions(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task)).await
	}

	async fn task_submitx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug, language: &Language, code: &str) -> Result<String> {
		<T as crate::Backend>::task_submit(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task), language, code).await
	}

	fn task_urlx(&self, session: &dyn AnyDebug, task: &dyn AnyDebug) -> Result<String> {
		<T as crate::Backend>::task_url(self, ujcast::<T::Session>(session), ujcast::<T::Task>(task))
	}

	fn contest_idx(&self, contest: &dyn AnyDebug) -> String {
		<T as crate::Backend>::contest_id(self, ujcast::<T::Contest>(contest))
	}

	fn contest_site_prefixx(&self) -> &'static str {
		<T as crate::Backend>::contest_site_prefix(self)
	}

	async fn contest_tasksx(&self, session: &dyn AnyDebug, contest: &dyn AnyDebug) -> Result<Vec<BoxedTask>> {
		Ok(<T as crate::Backend>::contest_tasks(self, ujcast::<T::Session>(session), ujcast::<T::Contest>(contest))
			.await?
			.into_iter()
			.map(|task| BoxedTask(Box::new(task)))
			.collect())
	}

	fn contest_urlx(&self, contest: &dyn AnyDebug) -> String {
		<T as crate::Backend>::contest_url(self, ujcast::<T::Contest>(contest))
	}

	async fn contest_titlex(&self, session: &dyn AnyDebug, contest: &dyn AnyDebug) -> Result<String> {
		<T as crate::Backend>::contest_title(self, ujcast::<T::Session>(session), ujcast::<T::Contest>(contest)).await
	}

	async fn contestsx(&self, session: &dyn AnyDebug) -> Result<Vec<BoxedContestDetails>> {
		Ok(<T as crate::Backend>::contests(self, ujcast::<T::Session>(session))
			.await?
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

fn ujcast<T: 'static>(x: &dyn AnyDebug) -> &T {
	x.as_any().downcast_ref::<T>().expect("unijudge type error mixing incompatible backends")
}

pub trait AnyDebug: Any+Debug+Send+Sync+'static {
	fn as_any(&self) -> &dyn Any;
	fn as_debug(&self) -> &dyn Debug;
}
impl<T: Any+Debug+Send+Sync+'static> AnyDebug for T {
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
