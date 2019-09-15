use crate::{telemetry::TELEMETRY, util};
use evscode::{error::ResultExt, E, R};
use std::{
	path::Path, process::{Command, Stdio}
};

#[evscode::command(title = "ICIE Terminal", key = "alt+t")]
async fn spawn() -> R<()> {
	External::command::<String, Vec<String>, String>(None, None)
}

pub struct Internal;
pub struct External;

pub fn debugger<A: AsRef<str>>(app: impl AsRef<str>, test: impl AsRef<Path>, command: impl IntoIterator<Item=A>) -> R<()> {
	let test = util::without_extension(test.as_ref().strip_prefix(evscode::workspace_root()?).wrap("found test outside of test directory")?);
	External::command(Some(format!("{} - {} - ICIE", test.to_str().unwrap(), app.as_ref())), Some(command))
}

pub fn install<A: AsRef<str>>(name: impl AsRef<str>, command: impl IntoIterator<Item=A>) -> R<()> {
	TELEMETRY.term_install.spark();
	Internal::command(format!("ICIE Install {}", name.as_ref()), Some(command))
}

impl Internal {
	pub fn raw<T: AsRef<str>, S: AsRef<str>>(title: T, data: S) -> R<()> {
		let term = evscode::Terminal::new().name(title).create();
		term.write(data);
		term.reveal();
		Ok(())
	}

	fn command<T: AsRef<str>, I: IntoIterator<Item=A>, A: AsRef<str>>(title: T, command: Option<I>) -> R<()> {
		Internal::raw(title, command.map(bash_escape_command).unwrap_or_default())
	}
}

/// The external terminal emulator that should be used on your system. Is set to `x-terminal-emulator`, a common alias for the default terminal emulator on many Linux systems. The command will be called like `x-terminal-emulator --title 'ICIE Thingy' -e 'bash'`.
#[evscode::config]
static EXTERNAL_COMMAND: evscode::Config<String> = "x-terminal-emulator";

/// Whether the external terminal should set a custom title.
#[evscode::config]
static EXTERNAL_CUSTOM_TITLE: evscode::Config<bool> = true;

impl External {
	fn command<T: AsRef<str>, I: IntoIterator<Item=A>, A: AsRef<str>>(title: Option<T>, command: Option<I>) -> R<()> {
		let program = EXTERNAL_COMMAND.get();
		let mut cmd = Command::new(&*program);
		if *EXTERNAL_CUSTOM_TITLE.get() {
			if let Some(title) = title {
				cmd.arg("--title").arg(title.as_ref());
			}
		}
		if let Some(command) = command {
			cmd.arg("-e").arg(bash_escape_command(command));
		}
		let kid = cmd
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.wrap(format!("failed to launch external terminal {:?}", program))?;
		evscode::spawn(async move {
			let out = kid.wait_with_output().wrap(format!("waiting for external terminal {:?} failed", program))?;
			if !out.status.success() {
				E::error(format!(
					"{:?} {:?} {:?}",
					out.status,
					String::from_utf8(out.stdout).wrap(format!("external terminal {:?} stdout is not utf8", program))?,
					String::from_utf8(out.stderr).wrap(format!("external terminal {:?} stderr is not utf8", program))?
				))
				.context(format!("failed to run external terminal {:?}", program))
				.emit();
			}
			Ok(())
		});
		Ok(())
	}
}

pub fn bash_escape_command<A: AsRef<str>>(command: impl IntoIterator<Item=A>) -> String {
	let escaped = command.into_iter().map(|p| util::bash_escape(p.as_ref())).collect::<Vec<_>>();
	escaped.join(" ")
}
