use crate::terminal::{bash_escape_command, BashTerminal, Terminal};
use async_trait::async_trait;
use evscode::R;

pub struct Internal;

#[async_trait(?Send)]
impl Terminal for Internal {
	async fn spawn(&self, title: &str, command: &[&str]) -> R<()> {
		self.spawn_bash(title, &bash_escape_command(command)).await
	}
}

#[async_trait(?Send)]
impl BashTerminal for Internal {
	async fn spawn_bash(&self, title: &str, data: &str) -> R<()> {
		let term = evscode::Terminal::new().name(title).create();
		if data != "\"bash\"" {
			term.write(data);
		}
		term.focus();
		Ok(())
	}
}
