use crate::{
	executable::{Executable, ExitKind, Run}, test::{Outcome, Task, Verdict}
};
use evscode::R;

pub async fn simple_test(
	exec: &Executable,
	input: &str,
	output: Option<&str>,
	output_alt: Option<&str>,
	task: &Task,
) -> R<Outcome>
{
	let run = exec.run(input, &[], &task.environment).await?;
	let verdict = select_verdict(&run, input, output, output_alt, task).await?;
	Ok(Outcome { verdict, out: run.stdout, stderr: run.stderr, time: run.time })
}

async fn select_verdict(
	run: &Run,
	input: &str,
	output: Option<&str>,
	output_alt: Option<&str>,
	task: &Task,
) -> R<Verdict>
{
	Ok(match run.exit_kind {
		ExitKind::Normal => {
			if !run.success() {
				Verdict::RuntimeError
			} else if let Some(output) = output {
				if task.checker.judge(input, output, &run.stdout).await? {
					Verdict::Accepted { alternative: false }
				} else if let Some(output_alt) = output_alt {
					if task.checker.judge(input, output_alt, &run.stdout).await? {
						Verdict::Accepted { alternative: true }
					} else {
						Verdict::WrongAnswer
					}
				} else {
					Verdict::WrongAnswer
				}
			} else {
				Verdict::IgnoredNoOut
			}
		},
		ExitKind::TimeLimitExceeded => Verdict::TimeLimitExceeded,
	})
}
