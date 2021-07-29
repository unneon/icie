use crate::{
	executable::{Environment, Executable}, terminal::{bash_escape_command, Terminal}, util
};
use async_trait::async_trait;
use evscode::{error::ResultExt, E, R};
use log::debug;

pub struct External;

#[async_trait(?Send)]
impl Terminal for External {
	async fn spawn(&self, title: &str, command: &[&str]) -> R<()> {
		let emulator = Emulator::detect().await?;
		let mut args = Vec::new();
		emulator.args_title(title, &mut args);
		emulator.args_command(command, &mut args);
		let args = args.iter().map(String::as_str).collect::<Vec<_>>();
		let run = emulator.executable.run("", &args, &Environment { time_limit: None, cwd: None }).await?;
		if run.success() {
			Ok(())
		} else {
			Err(E::error(format!("{:?} {:?} {:?}", run.exit_code, run.stdout, run.stderr))
				.context(format!("failed to run {:?} terminal emulator", emulator.executable.command)))
		}
	}
}

struct Emulator {
	executable: Executable,
	kind: EmulatorKind,
}

#[derive(Debug)]
enum EmulatorKind {
	Alacritty,
	Generic,
}

impl Emulator {
	async fn detect() -> R<Emulator> {
		let command = super::EXTERNAL_COMMAND.get();
		let app = util::find_app(&command).await?.wrap(&format!("terminal emulator {} not found", command))?;
		let app_path = app.read_link_8x().await?;
		let executable = Executable::new_name(command.clone());
		let kind = if app_path.ends_with("alacritty") { EmulatorKind::Alacritty } else { EmulatorKind::Generic };
		debug!("terminal found: {}, {}, {}, {:?}", command, app, app_path, kind);
		Ok(Emulator { executable, kind })
	}

	fn args_title(&self, title: &str, args: &mut Vec<String>) {
		args.push("--title".to_owned());
		args.push(title.to_owned());
	}

	fn args_command(&self, command: &[&str], args: &mut Vec<String>) {
		match self.kind {
			EmulatorKind::Generic => {
				args.push("-e".to_owned());
				args.push(bash_escape_command(command));
			},
			EmulatorKind::Alacritty => {
				args.push("-e".to_owned());
				args.extend(command.iter().map(|s| (*s).to_owned()));
			},
		}
	}
}
