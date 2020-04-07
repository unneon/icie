mod clang;
mod options;

use crate::{
	build::clang::compile, dir, executable::{Executable, Run}, telemetry::TELEMETRY, template, util::{self, fs, path::Path, suggest_init, workspace_root, Tempfile}
};
use evscode::{
	error::Severity, quick_pick, state::Scope, stdlib::output_channel::OutputChannel, Position, QuickPick, State, E, R
};
use once_cell::sync::Lazy;

use crate::util::SourceTarget;
pub use options::{Codegen, Standard};

#[derive(Debug)]
pub struct Location {
	pub path: Path,
	pub line: usize,
	pub column: usize,
}

#[derive(Debug)]
pub struct Message {
	pub message: String,
	pub location: Option<Location>,
}

#[derive(Debug)]
pub struct Status {
	pub run: Run,
	pub executable: Executable,
	pub errors: Vec<Message>,
	pub warnings: Vec<Message>,
}

pub const SOURCE_EXTENSIONS: &[&str] = &["cpp", "cxx", "cc"];

/// When a compilation error appears, the cursor will automatically move to the file and location
/// which caused the error. Regardless of this setting, an error message containing error details
/// will be shown.
#[evscode::config]
static AUTO_MOVE_TO_ERROR: vscode::Config<bool> = true;

/// An extension used to denote executable files. For example, if this entry is set to "xyz",
/// compiling a source file called main.cpp will create an executable called main.xyz.
#[evscode::config]
static EXECUTABLE_EXTENSION: evscode::Config<String> = "e";

/// C++ ISO language standard version. This corresponds to e.g. -std=c++17 flag on GCC/Clang. Be
/// aware some of these options may not be supported by your compiler, which will result in an
/// error.
#[evscode::config]
static CPP_STANDARD: evscode::Config<Standard> = Standard::Cpp17;

/// Additional C++ compilation flags. The flags will be appended to the command line after the
/// standard, warning, debug symbols and optimization flags. These flags will be used both in Debug
/// and Release profiles.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS: evscode::Config<String> = "";

/// Additional C++ compilation flags used in Debug profile. The flags will be appended to the
/// command line after the standard, warning, debug symbols, optimization flags and
/// profile-independent custom flags.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS_DEBUG: evscode::Config<String> = "";

/// Additional C++ compilation flags used in Release profile. The flags will be appended to the
/// command line after the standard, warning, debug symbols, optimization flags and
/// profile-independent custom flags.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS_RELEASE: evscode::Config<String> = "";

/// Additional C++ compilation flags used in Profile profile. The flags will be appended to the
/// command line after the standard, warning, debug symbols, optimization flags and
/// profile-independent custom flags.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS_PROFILE: evscode::Config<String> = "";

/// Custom path of your MinGW installation. If not set, ICIE will try, in order, "C:\MinGW" and
/// "C:\MinGW\mingw32".
#[evscode::config]
static WINDOWS_MINGW_PATH: evscode::Config<String> = "";

static COMPILER_INSTALL_CONFIRMED: State<bool> = State::new("icie.build.compiler_install_confirmed", Scope::Global);

#[evscode::command(title = "ICIE Manual Build", key = "alt+;")]
async fn manual() -> R<()> {
	let _status = crate::STATUS.push("Building manually");
	TELEMETRY.build_manual.spark();
	let sources = collect_possible_sources().await?;
	let source = select_source(&sources).await?;
	let codegen = select_codegen().await?;
	build(&SourceTarget::Custom(source), codegen, true).await?;
	Ok(())
}

async fn collect_possible_sources() -> R<Vec<Path>> {
	Ok(fs::read_dir(&workspace_root()?)
		.await?
		.into_iter()
		.filter(|path| SOURCE_EXTENSIONS.iter().any(|ext| Some(*ext) == path.extension().as_deref()))
		.collect())
}

async fn select_source(sources: &[Path]) -> R<Path> {
	let items = sources.iter().map(|source| quick_pick::Item::new(source.clone(), source.fmt_workspace()));
	let source = QuickPick::new().items(items).show().await.ok_or_else(E::cancel)?;
	Ok(source)
}

async fn select_codegen() -> R<Codegen> {
	let items = Codegen::LIST.iter().map(|codegen| {
		let label = format!("{:?}", codegen);
		let description = codegen.flags_clang().join(" ");
		quick_pick::Item::new(*codegen, label).description(description)
	});
	let codegen = QuickPick::new().ignore_focus_out().match_on_all().items(items).show().await.ok_or_else(E::cancel)?;
	Ok(codegen)
}

pub async fn build(source: &SourceTarget, codegen: Codegen, force_rebuild: bool) -> R<Executable> {
	let _status = crate::STATUS.push(util::fmt_verb("Building", &source));
	TELEMETRY.build_all.spark();
	let source = source.clone().into_path()?;
	evscode::save_all().await?;
	check_source_exists(&source).await?;
	let output_path = source.with_extension(&*EXECUTABLE_EXTENSION.get());
	if !force_rebuild && should_cache(&source, &output_path).await? {
		return Ok(Executable::new(output_path));
	}
	let sources = [&source];
	let standard = CPP_STANDARD.get();
	let custom_flags = get_custom_flags(codegen);
	let status = compile(&sources, &output_path, standard, codegen, &custom_flags).await?;
	display_compiler_stderr(&status.run.stderr);
	check_compiler_errors(&status).await?;
	COMPILER_INSTALL_CONFIRMED.set(&true).await;
	Ok(status.executable)
}

async fn should_cache(source: &Path, out: &Path) -> R<bool> {
	Ok(fs::exists(out).await? && fs::metadata(source).await?.modified < fs::metadata(out).await?.modified)
}

fn get_custom_flags(codegen: Codegen) -> Vec<String> {
	let flags = format!("{} {}", ADDITIONAL_CPP_FLAGS.get(), match codegen {
		Codegen::Debug => ADDITIONAL_CPP_FLAGS_DEBUG.get(),
		Codegen::Release => ADDITIONAL_CPP_FLAGS_RELEASE.get(),
		Codegen::Profile => ADDITIONAL_CPP_FLAGS_PROFILE.get(),
	});
	flags.split(' ').map(|flag| flag.trim().to_owned()).filter(|flag| !flag.is_empty()).collect::<Vec<_>>()
}

async fn check_source_exists(source: &Path) -> R<()> {
	if fs::exists(source).await? {
		Ok(())
	} else {
		let pretty_source = source.fmt_workspace();
		let error = E::error(format!("source {} does not exist at {}", pretty_source, source));
		let error = if source == &dir::solution()? {
			suggest_init(error)
		} else {
			error.action(format!("Create {} (Alt++)", pretty_source), crate::template::instantiate())
		};
		Err(error)
	}
}

fn display_compiler_stderr(stderr: &str) {
	thread_local! {
		static OUTPUT_CHANNEL: Lazy<OutputChannel> = Lazy::new(|| OutputChannel::new("ICIE Build"));
	}
	if !stderr.is_empty() {
		OUTPUT_CHANNEL.with(|output| {
			output.clear();
			output.append(&stderr);
			output.show(true);
		});
	}
}

async fn check_compiler_errors(status: &Status) -> R<()> {
	if status.run.success() {
		Ok(())
	} else if let Some(error) = status.errors.first() {
		try_move_cursor_to_error(error).await?;
		Err(E::error(&error.message).context("compilation error").severity(Severity::Workflow))
	} else {
		Err(E::error("unrecognized compilation error").extended(&status.run.stderr))
	}
}

async fn try_move_cursor_to_error(error: &Message) -> R<()> {
	if let Some(location) = &error.location {
		if AUTO_MOVE_TO_ERROR.get() {
			evscode::open_editor(location.path.as_str())
				.cursor(Position { line: location.line - 1, column: location.column - 1 })
				.open()
				.await?;
		}
	}
	Ok(())
}

pub async fn suggest_install_compiler() -> R<()> {
	let already_checked = COMPILER_INSTALL_CONFIRMED.get()? != Some(true);
	if already_checked {
		let message = "You have not compiled anything yet, should ICIE check if a C++ compiler is installed?";
		let should_check = evscode::Message::new(message).item((), "Check", false).warning().show().await.is_some();
		if should_check {
			dummy_compiler_run().await?;
			COMPILER_INSTALL_CONFIRMED.set(&true).await;
			evscode::Message::new::<()>("Compiling C++ was tested and it works. Good luck!").show().await;
		}
	}
	Ok(())
}

async fn dummy_compiler_run() -> R<()> {
	let code = template::generate_default_solution()?;
	let code_file = Tempfile::new("compilerinstallcheck", ".cpp", code).await?;
	let source = SourceTarget::Custom(code_file.path().to_owned());
	let executable = build(&source, Codegen::Debug, true).await?;
	fs::remove_file(&Path::from_native(executable.command)).await?;
	Ok(())
}

pub fn executable_path(source: SourceTarget) -> R<Path> {
	Ok(source.into_path()?.with_extension(&*EXECUTABLE_EXTENSION.get()))
}
