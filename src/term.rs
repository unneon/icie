use crate::{
	executable::{Environment, Executable}, telemetry::TELEMETRY, util, util::path::Path
};
use evscode::{error::ResultExt, E, R};

#[evscode::command(title = "ICIE Terminal", key = "alt+t")]
async fn spawn() -> R<()> {
	evscode::workspace_root()?;
	External::command::<Vec<String>, String>(None, None)
}

pub struct Internal;
pub struct External;

pub fn debugger<A: AsRef<str>>(
	app: impl AsRef<str>,
	test: &Path,
	command: impl IntoIterator<Item=A>,
) -> R<()>
{
	let test = util::without_extension(
		&test
			.as_ref()
			.strip_prefix(&Path::from_native(evscode::workspace_root()?))
			.wrap("found test outside of test directory")?,
	);
	External::command(
		Some(&format!("{} - {} - ICIE", test.to_str().unwrap(), app.as_ref())),
		Some(command),
	)
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

	fn command<T: AsRef<str>, I: IntoIterator<Item=A>, A: AsRef<str>>(
		title: T,
		command: Option<I>,
	) -> R<()>
	{
		Internal::raw(title, command.map(bash_escape_command).unwrap_or_default())
	}
}

/// The external terminal emulator that should be used on your system. Is set to
/// `x-terminal-emulator`, a common alias for the default terminal emulator on many Linux systems.
/// The command will be called like `x-terminal-emulator --title 'ICIE Thingy' -e 'bash'`.
#[evscode::config]
static EXTERNAL_COMMAND: evscode::Config<String> = "x-terminal-emulator";

/// Whether the external terminal should set a custom title.
#[evscode::config]
static EXTERNAL_CUSTOM_TITLE: evscode::Config<bool> = true;

impl External {
	fn command<I: IntoIterator<Item=A>, A: AsRef<str>>(
		title: Option<&str>,
		command: Option<I>,
	) -> R<()>
	{
		let title = title.map(str::to_owned);
		let command = command
			.map(|command| command.into_iter().map(|a| a.as_ref().to_owned()).collect::<Vec<_>>());
		evscode::spawn(async move {
			let program = Executable::new_name(EXTERNAL_COMMAND.get());
			let mut args = Vec::new();
			if EXTERNAL_CUSTOM_TITLE.get() {
				if let Some(title) = &title {
					args.push("--title");
					args.push(title.as_ref());
				}
			}
			let command = command.map(bash_escape_command);
			if let Some(command) = &command {
				args.push("-e");
				args.push(&command);
			}
			let run = program.run("", &args, &Environment { time_limit: None, cwd: None }).await?;
			if run.success() {
				Ok(())
			} else {
				Err(E::error(format!("{:?} {:?} {:?}", run.exit_code, run.stdout, run.stderr))
					.context(format!("failed to run external terminal {:?}", program.command)))
			}
		});
		Ok(())
	}
}

pub fn bash_escape_command<A: AsRef<str>>(command: impl IntoIterator<Item=A>) -> String {
	let escaped = command.into_iter().map(|p| util::bash_escape(p.as_ref())).collect::<Vec<_>>();
	escaped.join(" ")
}
