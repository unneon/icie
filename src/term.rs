use crate::util;
use std::{
	path::Path, process::{Command, Stdio}
};

#[evscode::command(title = "ICIE Terminal", key = "alt+t")]
fn spawn() -> evscode::R<()> {
	External::command::<String, Vec<String>, String>(None, None)
}

pub struct Internal;
pub struct External;

pub fn debugger<A: AsRef<str>>(app: impl AsRef<str>, test: impl AsRef<Path>, command: impl IntoIterator<Item=A>) -> evscode::R<()> {
	let test = util::without_extension(test.as_ref().strip_prefix(evscode::workspace_root()?)?);
	External::command(Some(format!("{} - {} - ICIE", test.to_str().unwrap(), app.as_ref())), Some(command))
}

pub fn install<A: AsRef<str>>(name: impl AsRef<str>, command: impl IntoIterator<Item=A>) -> evscode::R<()> {
	Internal::command(format!("ICIE Install {}", name.as_ref()), Some(command))
}

impl Internal {
	pub fn raw<T: AsRef<str>, S: AsRef<str>>(title: T, data: S) -> evscode::R<()> {
		let term = evscode::Terminal::new().name(title).create();
		term.write(data);
		term.reveal();
		Ok(())
	}

	fn command<T: AsRef<str>, I: IntoIterator<Item=A>, A: AsRef<str>>(title: T, command: Option<I>) -> evscode::R<()> {
		Internal::raw(title, command.map(|command| bash_escape_command(command)).unwrap_or(String::new()))
	}
}

impl External {
	fn command<T: AsRef<str>, I: IntoIterator<Item=A>, A: AsRef<str>>(title: Option<T>, command: Option<I>) -> evscode::R<()> {
		let mut cmd = Command::new("x-terminal-emulator");
		if let Some(title) = title {
			cmd.arg("-T").arg(title.as_ref());
		}
		if let Some(command) = command {
			cmd.arg("-e").arg(bash_escape_command(command));
		}
		cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
		Ok(())
	}
}

pub fn bash_escape_command<A: AsRef<str>>(command: impl IntoIterator<Item=A>) -> String {
	let escaped = command.into_iter().map(|p| util::bash_escape(p.as_ref())).collect::<Vec<_>>();
	escaped.join(" ")
}
