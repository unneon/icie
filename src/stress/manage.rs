use crate::{
	stress, stress::{execute_runs, render::render, Row}, test::{add_test, Verdict}
};
use async_trait::async_trait;
use evscode::{
	error::cancel_on, goodies::webview_collection::{Behaviour, Collection}, webview::{Disposer, Listener, WebviewMeta, WebviewRef}, E, R
};
use futures::{stream::select, StreamExt, TryStreamExt};
use once_cell::sync::Lazy;
use serde::{Serialize, Serializer};

pub struct Stress;

#[derive(Debug)]
pub enum Event {
	Row(Row),
	Add,
}

#[derive(Serialize)]
#[serde(tag = "tag")]
enum Food<'a> {
	#[serde(rename = "row")]
	Row {
		number: usize,
		#[serde(serialize_with = "ser_verdict")]
		verdict: Verdict,
		fitness: i64,
		input: Option<&'a str>,
	},
}

pub static WEBVIEW: Lazy<Collection<Stress>> = Lazy::new(|| Collection::new(Stress));

#[async_trait(?Send)]
impl Behaviour for Stress {
	type K = ();
	type V = ();

	fn create_empty(&self, _: Self::K) -> R<WebviewMeta> {
		Ok(evscode::Webview::new("icie.stress", "ICIE Stress", 1)
			.enable_scripts()
			.retain_context_when_hidden()
			.create())
	}

	async fn compute(&self, _: Self::K) -> R<Self::V> {
		Ok(())
	}

	async fn update(&self, _: Self::K, _: &Self::V, webview: WebviewRef) -> R<()> {
		webview.set_html(&render().await);
		Ok(())
	}

	async fn manage(&self, _: Self::K, webview: WebviewRef, listener: Listener, disposer: Disposer) -> R<()> {
		let _status = crate::STATUS.push("Stress testing");
		let state = stress::prepare_state().await.map_err(|e| e.context("could not start stress testing"))?;
		let mut best_row: Option<Row> = None;
		let mut events = Box::pin(cancel_on(
			select(execute_runs(&state).map_ok(Event::Row), listener.map(|_| Ok(Event::Add))),
			disposer,
		));
		while let Some(event) = events.next().await {
			match event?? {
				Event::Row(row) => {
					let is_counterexample = !row.outcome.success();
					let is_smallest = best_row.as_ref().map_or(true, |best_row| row.fitness > best_row.fitness);
					let is_new_best = is_counterexample && is_smallest;
					webview.post_message(Food::from_row(&row, is_new_best)).await;
					if is_new_best {
						best_row = Some(row);
					}
				},
				Event::Add => match &best_row {
					Some(best_row) => {
						add_test(&best_row.input, &best_row.desired).await?;
						break;
					},
					None => E::error("no test with non-AC verdict was found yet").emit(),
				},
			}
		}
		Ok(())
	}
}

impl<'a> Food<'a> {
	fn from_row(row: &'a Row, is_new_best: bool) -> Food {
		Food::Row {
			number: row.number,
			verdict: row.outcome.verdict,
			fitness: row.fitness,
			input: if is_new_best { Some(row.input.as_str()) } else { None },
		}
	}
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn ser_verdict<S>(verdict: &Verdict, s: S) -> Result<S::Ok, S::Error>
where S: Serializer {
	s.serialize_str(match verdict {
		Verdict::Accepted { .. } => "accept",
		Verdict::WrongAnswer => "wrong_answer",
		Verdict::RuntimeError => "runtime_error",
		Verdict::TimeLimitExceeded => "time_limit_exceeded",
		Verdict::IgnoredNoOut => "ignored_no_out",
	})
}
