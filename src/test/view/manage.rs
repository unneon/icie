use crate::{
	compile::{compile, Codegen}, debug::{gdb, rr}, dir, executable::Environment, test::{
		add_test, run, time_limit, view::{render::render, SCROLL_TO_FIRST_FAILED, SKILL_ACTIONS, SKILL_ADD}, TestRun
	}, util::{self, fs, path::Path, SourceTarget}
};
use async_trait::async_trait;
use evscode::{
	error::cancel_on, goodies::webview_collection::{Behaviour, Collection}, stdlib::webview::{Disposer, Listener}, webview::{WebviewMeta, WebviewRef}, Webview, E, R
};
use futures::StreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static COLLECTION: Lazy<Collection<TestView>> = Lazy::new(|| Collection::new(TestView));

pub struct TestView;

#[async_trait(?Send)]
impl Behaviour for TestView {
	type K = SourceTarget;
	type V = Vec<TestRun>;

	fn create_empty(&self, source: Self::K) -> R<WebviewMeta> {
		let title = util::fmt::verb_on_source("ICIE Test View", &source);
		Ok(Webview::new("icie.test.view", &title, 2).enable_scripts()
		.retain_context_when_hidden().create())
	}

	async fn compute(&self, source: Self::K) -> R<Self::V> {
		run(source.clone()).await
	}

	async fn update(&self, _: Self::K, report: &Self::V, webview: WebviewRef) -> R<()> {
		webview.set_html(&render(report,webview.clone()).await?);
		webview.reveal(2, true);
		Ok(())
	}

	async fn manage(&self, source: Self::K, webview: WebviewRef, listener: Listener, disposer: Disposer) -> R<()> {
		let mut stream = cancel_on(listener, disposer);
		while let Some(note) = stream.next().await {
			let note: Note = note?.into_serde().unwrap();
			match note {
				Note::TriggerRR { in_path } => {
					let source = source.clone();
					evscode::spawn(async move { rr(&in_path, source).await });
				},
				Note::TriggerGDB { in_path } => {
					let source = source.clone();
					evscode::spawn(async move { gdb(&in_path, source).await });
				},
				Note::NewTest { input, desired } => evscode::spawn(async move {
					if !input.is_empty() && !desired.is_empty() {
						SKILL_ADD.add_use().await;
					}
					add_test(&input, &desired).await
				}),
				Note::SetAlt { in_path, out } => evscode::spawn(async move {
					let in_alt_path = in_path.with_extension("alt.out");
					fs::write(&in_alt_path, out).await?;
					COLLECTION.update_all().await?;
					Ok(())
				}),
				Note::DelAlt { in_path } => evscode::spawn(async move {
					let in_alt_path = in_path.with_extension("alt.out");
					fs::remove_file(&in_alt_path).await?;
					COLLECTION.update_all().await?;
					Ok(())
				}),
				Note::Edit { path } => {
					if !fs::exists(&path).await? {
						fs::write(&path, "").await?;
					}
					util::open_source(&path).await?;
				},
				Note::ActionNotice => SKILL_ACTIONS.add_use().await,
				Note::AfterLoad => {
					if SCROLL_TO_FIRST_FAILED.get() {
							let _ = webview.post_message(Food::ScrollToWA).await;
					}
				},
				Note::EvalReq { id, input } => {
					if let Ok(brute_force) = dir::brute_force() {
						if fs::exists(&brute_force).await? {
							let webview = webview.clone();
							evscode::spawn(async move {
								let _status = crate::STATUS.push("Evaluating");
								let brute_force = compile(&SourceTarget::BruteForce, Codegen::Release, false).await?;
								let environment = Environment { time_limit: time_limit(), cwd: None };
								let run = brute_force.run(&input, &[], &environment).await?;
								drop(_status);
								if run.success() {
									add_test(&input, &run.stdout).await?;
									let _ = webview.post_message(Food::EvalResp { id, input }).await;
									Ok(())
								} else {
									Err(E::error("brute force solution did not evaluate test successfully"))
								}
							});
						}
					}
				},
			}
		}
		Ok(())
	}
}

#[derive(Deserialize)]
#[serde(tag = "tag")]
enum Note {
	#[serde(rename = "trigger_rr")]
	TriggerRR { in_path: Path },
	#[serde(rename = "trigger_gdb")]
	TriggerGDB { in_path: Path },
	#[serde(rename = "new_test")]
	NewTest { input: String, desired: String },
	#[serde(rename = "set_alt")]
	SetAlt { in_path: Path, out: String },
	#[serde(rename = "del_alt")]
	DelAlt { in_path: Path },
	#[serde(rename = "edit")]
	Edit { path: Path },
	#[serde(rename = "action_notice")]
	ActionNotice,
	#[serde(rename = "eval_req")]
	EvalReq { id: i64, input: String },
	#[serde(rename = "after_load")]
	AfterLoad
}

#[derive(Serialize)]
#[serde(tag = "tag")]
pub enum Food {
	#[serde(rename = "scroll_to_wa")]
	ScrollToWA,
	#[serde(rename = "eval_resp")]
	EvalResp { id: i64, input: String },
	#[serde(rename = "new_start")]
	NewStart,
}
