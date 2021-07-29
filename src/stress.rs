pub mod manage;
mod render;

use crate::{
	compile::{compile, Codegen}, executable::{Environment, Executable}, test::{judge::simple_test, Outcome, Task}, util::SourceTarget
};
use evscode::{E, R};
use futures::{Stream, StreamExt};

#[derive(Debug)]
pub struct Row {
	pub number: usize,
	pub outcome: Outcome,
	pub fitness: i64,
	pub input: String,
	pub desired: String,
}

pub struct StressState {
	solution: Executable,
	brute_force: Executable,
	test_generator: Executable,
	task: Task,
}

#[evscode::command(title = "ICIE Stress", key = "alt+9")]
async fn open() -> R<()> {
	let webview = manage::WEBVIEW.get_lazy(()).await?;
	webview.reveal(1, false);
	Ok(())
}

pub async fn prepare_state() -> R<StressState> {
	let solution = compile(&SourceTarget::Main, Codegen::Debug, false).await?;
	let brute_force = compile(&SourceTarget::BruteForce, Codegen::Release, false).await?;
	let test_generator = compile(&SourceTarget::TestGenerator, Codegen::Release, false).await?;
	let task = Task::simple().await?;
	Ok(StressState { solution, brute_force, test_generator, task })
}

pub fn execute_runs(state: &StressState) -> impl Stream<Item=R<Row>>+'_ {
	futures::stream::iter(1..).then(move |number| async move { execute_run(number, state).await })
}

async fn execute_run(number: usize, state: &StressState) -> R<Row> {
	let environment = &state.task.environment;
	let input = run_test_generator(&state.test_generator, environment).await?;
	let desired = run_brute_force(&input, &state.brute_force, environment).await?;
	let outcome = simple_test(&state.solution, &input, Some(&desired), None, &state.task)
		.await
		.map_err(|e| e.context("failed to run test in stress"))?;
	let fitness = -(input.len() as i64);
	let row = Row { number, outcome, fitness, input, desired };
	Ok(row)
}

async fn run_test_generator(test_generator: &Executable, environment: &Environment) -> R<String> {
	let run_test_generator =
		test_generator.run("", &[], environment).await.map_err(|e| e.context("executing test generator aborted"))?;
	if !run_test_generator.success() {
		return Err(E::error(format!("executing test generator failed, {:?}", run_test_generator)));
	}
	Ok(run_test_generator.stdout)
}

async fn run_brute_force(input: &str, brute_force: &Executable, environment: &Environment) -> R<String> {
	let run_brute_force = brute_force
		.run(input, &[], environment)
		.await
		.map_err(|e| e.context("executing brute force solution aborted"))?;
	if !run_brute_force.success() {
		return Err(E::error(format!("executing brute force solution failed, {:?}", run_brute_force)));
	}
	Ok(run_brute_force.stdout)
}
