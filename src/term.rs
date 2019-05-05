use crate::util;
use std::process::{Command, Stdio};

pub fn install(name: &str, command: &str, args: &[&str]) -> evscode::R<()> {
	internal(&format!("ICIE Install {}", name), &bash_escape_command(command, args));
	Ok(())
}

pub fn debugger(command: &str, args: &[&str]) -> evscode::R<()> {
	external(command, args)
}

fn external(command: &str, args: &[&str]) -> evscode::R<()> {
	Command::new("x-terminal-emulator")
		.arg("-e")
		.arg(bash_escape_command(command, args))
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()?;
	Ok(())
}

pub fn internal(title: &str, raw: &str) {
	let term = evscode::Terminal::new().name(format!("ICIE Install {}", title)).create();
	term.write(raw);
	term.reveal();
}

pub fn bash_escape_command(command: &str, args: &[&str]) -> String {
	let mut line = String::from(command);
	for arg in args {
		line += " ";
		line += &util::bash_escape(arg);
	}
	line
}
