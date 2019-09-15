use crate::{
	build::{build, clang::Codegen}, debug::{gdb, rr}, dir, telemetry::TELEMETRY, test::{
		add_test, exec::Environment, run, time_limit, view::{render::render, SCROLL_TO_FIRST_FAILED, SKILL_ACTIONS}, TestRun
	}, util::{fmt_verb, fs_remove_file, fs_write}
};
use async_trait::async_trait;
use evscode::{
	error::cancel_on, goodies::webview_collection::{Behaviour, Collection}, stdlib::webview::{Disposer, Listener}, webview::{WebviewMeta, WebviewRef}, Webview, E, R
};
use futures::StreamExt;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};

lazy_static! {
	pub static ref COLLECTION: Collection<TestView> = Collection::new(TestView);
}

pub struct TestView;

#[async_trait]
impl Behaviour for TestView {
	type K = Option<PathBuf>;
	type V = Vec<TestRun>;

	fn create_empty(&self, source: Self::K) -> R<WebviewMeta> {
		let title = fmt_verb("ICIE Test View", &source);
		Ok(Webview::new("icie.test.view", &title, 2).enable_scripts().retain_context_when_hidden().create())
	}

	async fn compute(&self, source: Self::K) -> R<Self::V> {
		run(&source).await
	}

	async fn update(&self, _: Self::K, report: &Self::V, webview: WebviewRef) -> R<()> {
		webview.set_html(&render(&report).await?);
		webview.reveal(2, true);
		if *SCROLL_TO_FIRST_FAILED.get() {
			webview.post_message(json::object! {
				"tag" => "scroll_to_wa",
			});
		}
		Ok(())
	}

	async fn manage(&self, source: Self::K, webview: WebviewRef, listener: Listener, disposer: Disposer) -> R<()> {
		let mut stream = cancel_on(listener, disposer);
		while let Some(note) = stream.next().await {
			let note = note?;
			match note["tag"].as_str() {
				Some("trigger_rr") => {
					let source = source.clone();
					let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
					evscode::spawn(rr(in_path, source));
				},
				Some("trigger_gdb") => {
					let source = source.clone();
					let in_path = PathBuf::from(note["in_path"].as_str().unwrap());
					evscode::spawn(gdb(in_path, source));
				},
				Some("new_test") => evscode::spawn(async move {
					let input = note["input"].as_str().unwrap();
					let desired = note["desired"].as_str().unwrap();
					add_test(input, desired).await
				}),
				Some("set_alt") => evscode::spawn(async move {
					TELEMETRY.test_alternative_add.spark();
					let in_path = Path::new(note["in_path"].as_str().unwrap());
					let in_alt_path = in_path.with_extension("alt.out");
					let out = note["out"].as_str().unwrap().trim();
					fs_write(&in_alt_path, out).await?;
					COLLECTION.update_all().await?;
					Ok(())
				}),
				Some("del_alt") => evscode::spawn(async move {
					TELEMETRY.test_alternative_delete.spark();
					let in_path = Path::new(note["in_path"].as_str().unwrap());
					let in_alt_path = in_path.with_extension("alt.out");
					fs_remove_file(&in_alt_path).await?;
					COLLECTION.update_all().await?;
					Ok(())
				}),
				Some("edit") => {
					TELEMETRY.test_edit.spark();
					let path = Path::new(note["path"].as_str().unwrap());
					evscode::open_editor(path).open().await;
				},
				Some("action_notice") => SKILL_ACTIONS.add_use().await,
				Some("eval_req") => {
					if let Ok(brut) = dir::brut() {
						if brut.exists() {
							let webview = webview.clone();
							evscode::spawn(async move {
								TELEMETRY.test_eval.spark();
								let _status = crate::STATUS.push("Evaluating");
								let id = note["id"].as_i64().unwrap();
								let input = note["input"].as_str().unwrap();
								let brut = build(brut, &Codegen::Release, false).await?;
								let environment = Environment { time_limit: time_limit() };
								let run = brut.run(input, &[], &environment).await?;
								drop(_status);
								if run.success() {
									add_test(input, &run.stdout).await?;
									webview.post_message(json::object! {
										"tag" => "eval_resp",
										"id" => id,
										"input" => input,
									});
									Ok(())
								} else {
									Err(E::error("brut did not evaluate test successfully"))
								}
							});
						}
					}
				},
				_ => return Err(E::error(format!("invalid webview message `{}`", note.dump()))),
			}
		}
		Ok(())
	}
}
