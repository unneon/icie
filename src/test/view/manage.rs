use crate::{
	build::{build, Codegen}, debug::{gdb, rr}, dir, executable::Environment, telemetry::TELEMETRY, test::{
		add_test, run, time_limit, view::{render::render, SCROLL_TO_FIRST_FAILED, SKILL_ACTIONS}, TestRun
	}, util::{fmt_verb, fs, path::Path}
};
use async_trait::async_trait;
use evscode::{
	error::cancel_on, goodies::webview_collection::{Behaviour, Collection}, stdlib::webview::{Disposer, Listener}, webview::{WebviewMeta, WebviewRef}, Webview, E, R
};
use futures::StreamExt;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
	pub static ref COLLECTION: Collection<TestView> = Collection::new(TestView);
}

pub struct TestView;

#[async_trait(?Send)]
impl Behaviour for TestView {
	type K = Option<Path>;
	type V = Vec<TestRun>;

	fn create_empty(&self, source: Self::K) -> R<WebviewMeta> {
		let title = fmt_verb("ICIE Test View", &source);
		Ok(Webview::new("icie.test.view", &title, 2)
			.enable_scripts()
			.retain_context_when_hidden()
			.create())
	}

	async fn compute(&self, source: Self::K) -> R<Self::V> {
		run(&source).await
	}

	async fn update(&self, _: Self::K, report: &Self::V, webview: WebviewRef) -> R<()> {
		webview.set_html(&render(&report).await?);
		webview.reveal(2, true);
		if SCROLL_TO_FIRST_FAILED.get() {
			webview.post_message(Food::ScrollToWA).await;
		}
		Ok(())
	}

	async fn manage(
		&self,
		source: Self::K,
		webview: WebviewRef,
		listener: Listener,
		disposer: Disposer,
	) -> R<()>
	{
		let mut stream = cancel_on(listener, disposer);
		while let Some(note) = stream.next().await {
			let note: Note = note?.into_serde().unwrap();
			match note {
				Note::TriggerRR { in_path } => {
					let source = source.clone();
					evscode::spawn(rr(in_path, source));
				},
				Note::TriggerGDB { in_path } => {
					let source = source.clone();
					evscode::spawn(gdb(in_path, source));
				},
				Note::NewTest { input, desired } => {
					evscode::spawn(async move { add_test(&input, &desired).await })
				},
				Note::SetAlt { in_path, out } => evscode::spawn(async move {
					TELEMETRY.test_alternative_add.spark();
					let in_alt_path = in_path.with_extension("alt.out");
					fs::write(&in_alt_path, out).await?;
					COLLECTION.update_all().await?;
					Ok(())
				}),
				Note::DelAlt { in_path } => evscode::spawn(async move {
					TELEMETRY.test_alternative_delete.spark();
					let in_alt_path = in_path.with_extension("alt.out");
					fs::remove_file(&in_alt_path).await?;
					COLLECTION.update_all().await?;
					Ok(())
				}),
				Note::Edit { path } => {
					TELEMETRY.test_edit.spark();
					if !fs::exists(&path).await? {
						fs::write(&path, "").await?;
					}
					evscode::open_editor(&path).open().await?;
				},
				Note::ActionNotice => SKILL_ACTIONS.add_use().await,
				Note::EvalReq { id, input } => {
					if let Ok(brut) = dir::brut() {
						if fs::exists(&brut).await? {
							let webview = webview.clone();
							evscode::spawn(async move {
								TELEMETRY.test_eval.spark();
								let _status = crate::STATUS.push("Evaluating");
								let brut = build(brut, Codegen::Release, false).await?;
								let environment =
									Environment { time_limit: time_limit(), cwd: None };
								let run = brut.run(&input, &[], &environment).await?;
								drop(_status);
								if run.success() {
									add_test(&input, &run.stdout).await?;
									webview.post_message(Food::EvalResp { id, input }).await;
									Ok(())
								} else {
									Err(E::error("brut did not evaluate test successfully"))
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
