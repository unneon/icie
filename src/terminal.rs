use crate::{
	executable::{Environment, Executable}, telemetry::TELEMETRY, util, util::{path::Path, workspace_root}
};
use evscode::{E, R};
use log::debug;

#[evscode::command(title = "ICIE Terminal", key = "alt+t")]
async fn spawn() -> R<()> {
	workspace_root()?;
	External::command::<Vec<String>, String>(None, None)
}

pub struct Internal;
pub struct External;

pub fn debugger<A: AsRef<str>>(app: impl AsRef<str>, test: &Path, command: impl IntoIterator<Item=A>) -> R<()> {
	let test = test.without_extension().fmt_workspace();
	External::command(Some(&format!("{} - {} - ICIE", test.as_str(), app.as_ref())), Some(command))
}

pub fn install<A: AsRef<str>>(name: impl AsRef<str>, command: impl IntoIterator<Item=A>) -> R<()> {
	TELEMETRY.term_install.spark();
	Internal::command(format!("ICIE Install {}", name.as_ref()), Some(command))
}

impl Internal {
	pub fn raw<T: AsRef<str>, S: AsRef<str>>(title: T, data: S) -> R<()> {
		let term = evscode::Terminal::new().name(title).create();
		term.write(data.as_ref());
		term.reveal();
		Ok(())
	}

	fn command<T: AsRef<str>, I: IntoIterator<Item=A>, A: AsRef<str>>(title: T, command: Option<I>) -> R<()> {
		Internal::raw(title, command.map(bash_escape_command).unwrap_or_default())
	}
}

/// The external terminal emulator that should be used on your system. Is set to
/// `x-terminal-emulator`, a common alias for the default terminal emulator on many Linux systems.
/// The command will be called like `x-terminal-emulator --title 'ICIE Thingy' -e 'bash'`.
#[evscode::config]
static EXTERNAL_COMMAND: evscode::Config<String> = "x-terminal-emulator";

impl External {
	fn command<I: IntoIterator<Item=A>, A: AsRef<str>>(title: Option<&str>, command: Option<I>) -> R<()> {
		let title = title.map(str::to_owned);
		let command = command.map(|command| command.into_iter().map(|a| a.as_ref().to_owned()).collect::<Vec<_>>());
		evscode::spawn(async move {
			let emulator = Emulator::detect().await?;
			let mut args = Vec::new();
			if let Some(title) = &title {
				emulator.args_title(&title, &mut args);
			}
			if let Some(command) = &command {
				emulator.args_command(command, &mut args);
			}
			let args: Vec<_> = args.iter().map(String::as_str).collect();
			let run = emulator.executable.run("", &args, &Environment { time_limit: None, cwd: None }).await?;
			if run.success() {
				Ok(())
			} else {
				Err(E::error(format!("{:?} {:?} {:?}", run.exit_code, run.stdout, run.stderr))
					.context(format!("failed to run {:?} terminal emulator", emulator.executable.command)))
			}
		});
		Ok(())
	}
}

pub fn bash_escape_command<A: AsRef<str>>(command: impl IntoIterator<Item=A>) -> String {
	let escaped = command.into_iter().map(|p| util::bash_escape(p.as_ref())).collect::<Vec<_>>();
	escaped.join(" ")
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
		let command = EXTERNAL_COMMAND.get();
		let app = util::find_app(&command).await?.unwrap();
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

	fn args_command<I>(&self, command: I, args: &mut Vec<String>)
	where I: IntoIterator<Item: AsRef<str>> {
		match self.kind {
			EmulatorKind::Generic => {
				args.push("-e".to_owned());
				args.push(bash_escape_command(command));
			},
			EmulatorKind::Alacritty => {
				args.push("-e".to_owned());
				args.extend(command.into_iter().map(|c| c.as_ref().to_owned()));
			},
		}
	}
}
