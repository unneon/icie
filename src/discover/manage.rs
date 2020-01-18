use crate::{
	build::{build, Codegen}, checker::get_checker, dir, discover::render::render, executable::{Environment, Executable}, test::{
		self, add_test, judge::{simple_test, Outcome, Verdict}, time_limit, Task
	}
};
use async_trait::async_trait;
use evscode::{
	error::cancel_on, goodies::webview_collection::{Behaviour, Collection}, webview::{Disposer, Listener, WebviewMeta, WebviewRef}, E, R
};
use futures::{stream::select, Stream, StreamExt, TryStreamExt};
use serde::{Serialize, Serializer};

lazy_static::lazy_static! {
	pub static ref WEBVIEW: Collection<Discover> = Collection::new(Discover);
}

pub struct Discover;

#[async_trait(?Send)]
impl Behaviour for Discover {
	type K = ();
	type V = ();

	fn create_empty(&self, _: Self::K) -> R<WebviewMeta> {
		Ok(evscode::Webview::new("icie.discover", "ICIE Discover", 1)
			.enable_scripts()
			.retain_context_when_hidden()
			.create())
	}

	async fn compute(&self, _: Self::K) -> R<Self::V> {
		Ok(())
	}

	async fn update(&self, _: Self::K, _: &Self::V, webview: WebviewRef) -> R<()> {
		webview.set_html(&render());
		Ok(())
	}

	async fn manage(
		&self,
		_: Self::K,
		webview: WebviewRef,
		listener: Listener,
		disposer: Disposer,
	) -> R<()>
	{
		let _status = crate::STATUS.push("Discovering");
		let source = dir::solution()?;
		let solution = build(&source, Codegen::Debug, false).await?;
		let brut = build(dir::brut()?, Codegen::Release, false).await?;
		let gen = build(dir::gen()?, Codegen::Release, false).await?;
		let task = Task {
			checker: get_checker().await?,
			environment: Environment { time_limit: time_limit(), cwd: None },
		};
		let mut best_row: Option<Row> = None;
		let mut events = Box::pin(cancel_on(
			select(
				execute_runs(&solution, &brut, &gen, &task).map_ok(Event::Row),
				listener.map(|_| Event::Add).map(Ok),
			),
			disposer,
		));
		while let Some(event) = events.next().await {
			match event?? {
				Event::Row(row) => {
					let is_counterexample = !row.outcome.success();
					let is_smallest =
						best_row.as_ref().map_or(true, |best_row| row.fitness > best_row.fitness);
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
						test::view::manage::COLLECTION.get_force(None).await?;
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
	brut: &'a Executable,
	gen: &'a Executable,
	task: &'a Task,
) -> impl Stream<Item=R<Row>>+'a
{
	futures::stream::iter(1..).then(move |number| execute_run(number, solution, brut, gen, task))
}

async fn execute_run(
	number: usize,
	solution: &Executable,
	brut: &Executable,
	gen: &Executable,
	task: &Task,
) -> R<Row>
{
	let run_gen = gen
		.run("", &[], &task.environment)
		.await
		.map_err(|e| e.context("executing test generator aborted"))?;
	if !run_gen.success() {
		return Err(E::error(format!("executing test generator failed, {:?}", run_gen)));
	}
	let input = run_gen.stdout;
	let run_brut = brut
		.run(&input, &[], &task.environment)
		.await
		.map_err(|e| e.context("executing slow solution aborted"))?;
	if !run_brut.success() {
		return Err(E::error(format!("executing slow solution failed, {:?}", run_brut)));
	}
	let desired = run_brut.stdout;
	let outcome = simple_test(&solution, &input, Some(&desired), None, &task)
		.await
		.map_err(|e| e.context("failed to run test in discover"))?;
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
