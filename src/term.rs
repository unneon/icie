use crate::util;
use evscode::{E, R};
use std::{
	path::Path, process::{Command, Stdio}
};

#[evscode::command(title = "ICIE Terminal", key = "alt+t")]
fn spawn() -> R<()> {
	External::command::<String, Vec<String>, String>(None, None)
}

pub struct Internal;
pub struct External;

pub fn debugger<A: AsRef<str>>(app: impl AsRef<str>, test: impl AsRef<Path>, command: impl IntoIterator<Item=A>) -> R<()> {
	let test = util::without_extension(
		test.as_ref()
			.strip_prefix(evscode::workspace_root()?)
			.map_err(|e| E::from_std(e).context("found test outside of test directory"))?,
	);
	External::command(Some(format!("{} - {} - ICIE", test.to_str().unwrap(), app.as_ref())), Some(command))
}

pub fn install<A: AsRef<str>>(name: impl AsRef<str>, command: impl IntoIterator<Item=A>) -> R<()> {
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

impl External {
	fn command<T: AsRef<str>, I: IntoIterator<Item=A>, A: AsRef<str>>(title: Option<T>, command: Option<I>) -> R<()> {
		let mut cmd = Command::new("x-terminal-emulator");
		if let Some(title) = title {
			cmd.arg("-T").arg(title.as_ref());
		}
		if let Some(command) = command {
			cmd.arg("-e").arg(bash_escape_command(command));
		}
		cmd.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.spawn()
			.map_err(|e| E::from_std(e).context("failed to launch x-terminal-emulator"))?;
		Ok(())
	}
}

pub fn bash_escape_command<A: AsRef<str>>(command: impl IntoIterator<Item=A>) -> String {
	let escaped = command.into_iter().map(|p| util::bash_escape(p.as_ref())).collect::<Vec<_>>();
	escaped.join(" ")
}
