use crate::{
	checker::get_checker, compile::{compile, Codegen}, dir, executable::{Environment, Executable}, stress::render::render, test::{self, add_test, judge::simple_test, time_limit, Outcome, Task, Verdict}, util::SourceTarget
};
use async_trait::async_trait;
use evscode::{
	error::cancel_on, goodies::webview_collection::{Behaviour, Collection}, webview::{Disposer, Listener, WebviewMeta, WebviewRef}, E, R
};
use futures::{stream::select, Stream, StreamExt, TryStreamExt};
use once_cell::sync::Lazy;
use serde::{Serialize, Serializer};

pub static WEBVIEW: Lazy<Collection<Stress>> = Lazy::new(|| Collection::new(Stress));

pub struct Stress;

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
		let solution = compile(&SourceTarget::Main, Codegen::Debug, false).await?;
		let brute_force =
			compile(&SourceTarget::Custom(dir::brute_force()?), Codegen::Release, false).await.map_err(|e| {
				e.context("could not start stress testing").action("Only run normal tests (Alt+0)", crate::test::view())
			})?;
		let gen = compile(&SourceTarget::Custom(dir::test_generator()?), Codegen::Release, false).await?;
		let task =
			Task { checker: get_checker().await?, environment: Environment { time_limit: time_limit(), cwd: None } };
		let mut best_row: Option<Row> = None;
		let mut events = Box::pin(cancel_on(
			select(
				execute_runs(&solution, &brute_force, &gen, &task).map_ok(Event::Row),
				listener.map(|_| Event::Add).map(Ok),
			),
			disposer,
		));
		while let Some(event) = events.next().await {
			match event?? {
				Event::Row(row) => {
					let is_counterexample = !row.outcome.success();
					let is_smallest = best_row.as_ref().map_or(true, |best_row| row.fitness > best_row.fitness);
					let is_new_best = is_counterexample && is_smallest;
					webview
						.post_message(Food::Row {
							number: row.number,
							outcome: row.outcome.verdict,
							fitness: row.fitness,
							input: if is_new_best { Some(row.input.as_str()) } else { None },
						})
						.await;
					if is_new_best {
						best_row = Some(row);
					}
				},
				Event::Add => match &best_row {
					Some(best_row) => {
						add_test(&best_row.input, &best_row.desired).await?;
						test::view::manage::COLLECTION.get_force(SourceTarget::Main).await?;
						break;
					},
					None => E::error("no test with non-AC verdict was found yet").emit(),
				},
			}
		}
		Ok(())
	}
}

#[derive(Debug)]
pub enum Event {
	Row(Row),
	Add,
}

#[derive(Debug)]
pub struct Row {
	pub number: usize,
	pub outcome: Outcome,
	pub fitness: i64,
	pub input: String,
	pub desired: String,
}

fn execute_runs<'a>(
	solution: &'a Executable,
	brute_force: &'a Executable,
	test_generator: &'a Executable,
	task: &'a Task,
) -> impl Stream<Item=R<Row>>+'a
{
	futures::stream::iter(1..).then(move |number| execute_run(number, solution, brute_force, test_generator, task))
}

async fn execute_run(
	number: usize,
	solution: &Executable,
	brute_force: &Executable,
	test_generator: &Executable,
	task: &Task,
) -> R<Row>
{
	let run_test_generator = test_generator
		.run("", &[], &task.environment)
		.await
		.map_err(|e| e.context("executing test generator aborted"))?;
	if !run_test_generator.success() {
		return Err(E::error(format!("executing test generator failed, {:?}", run_test_generator)));
	}
	let input = run_test_generator.stdout;
	let run_brute_force = brute_force
		.run(&input, &[], &task.environment)
		.await
		.map_err(|e| e.context("executing brute force solution aborted"))?;
	if !run_brute_force.success() {
		return Err(E::error(format!("executing brute force solution failed, {:?}", run_brute_force)));
	}
	let desired = run_brute_force.stdout;
	let outcome = simple_test(&solution, &input, Some(&desired), None, &task)
		.await
		.map_err(|e| e.context("failed to run test in stress"))?;
	let fitness = -(input.len() as i64);
	let row = Row { number, outcome, fitness, input, desired };
	Ok(row)
}

#[derive(Serialize)]
#[serde(tag = "tag")]
enum Food<'a> {
	#[serde(rename = "row")]
	Row {
		number: usize,
		#[serde(serialize_with = "ser_verdict")]
		outcome: Verdict,
		fitness: i64,
		input: Option<&'a str>,
	},
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
