mod external;
mod fallback;
mod internal;

pub use internal::Internal;

use crate::{
	util, util::{path::Path, workspace_root}
};
use async_trait::async_trait;
use evscode::R;

#[async_trait(?Send)]
pub trait Terminal {
	async fn spawn(&self, title: &str, command: &[&str]) -> R<()>;
}

#[async_trait(?Send)]
pub trait BashTerminal {
	async fn spawn_bash(&self, title: &str, data: &str) -> R<()>;
}

/// The external terminal emulator that should be used on your system. Is set to
/// `x-terminal-emulator`, a common alias for the default terminal emulator on many Linux systems.
/// The command will be called like `x-terminal-emulator --title 'ICIE Thingy' -e 'bash'`.
#[evscode::config]
static EXTERNAL_COMMAND: evscode::Config<String> = "x-terminal-emulator";

#[evscode::command(title = "ICIE Terminal", key = "alt+t")]
async fn spawn() -> R<()> {
	workspace_root()?;
	get_preferred().spawn("ICIE Terminal", &["bash"]).await
}

pub async fn debugger(name: &str, test: &Path, command: &[&str]) -> R<()> {
	let test = test.without_extension().fmt_workspace();
	let title = format!("ICIE {} - {}", name, test);
	get_preferred().spawn(&title, command).await
}

pub async fn install(name: &str, command: &[&str]) -> R<()> {
	let title = format!("ICIE Install {}", name);
	get_unobtrusive().spawn(&title, command).await
}

fn get_preferred() -> impl Terminal {
	fallback::Fallback(external::External, Internal)
}

fn get_unobtrusive() -> impl Terminal {
	Internal
}

fn bash_escape_command(command: &[&str]) -> String {
	command.iter().copied().map(util::bash_escape).collect::<Vec<_>>().join(" ")
}
