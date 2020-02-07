mod clang;

use crate::{
	build::clang::compile, dir, executable::Executable, telemetry::TELEMETRY, util::{self, fs, path::Path}
};
use evscode::{error::ResultExt, Position, R};

/// When a compilation error appears, the cursor will automatically move to the file and location
/// which caused the error. Regardless of this setting, an error message containing error details
/// will be shown.
#[evscode::config]
static AUTO_MOVE_TO_ERROR: vscode::Config<bool> = true;

/// When a compilation warning appears, the cursor will automatically move to the file and location
/// which caused the warning. If this is not set, a warning message will be shown with a "Show"
/// button which will move the cursor to the location of the warning.
#[evscode::config]
static AUTO_MOVE_TO_WARNING: evscode::Config<bool> = true;

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

#[evscode::command(title = "ICIE Manual Build", key = "alt+;")]
async fn manual() -> evscode::R<()> {
	let _status = crate::STATUS.push("Manually building");
	TELEMETRY.build_manual.spark();
	let root = Path::from_native(evscode::workspace_root()?);
	let sources = fs::read_dir(root.as_ref()).await?.into_iter().filter(|path| {
		ALLOWED_EXTENSIONS.iter().any(|ext| Some((*ext).to_owned()) == path.extension())
	});
	let source = Path::from_native(
		evscode::QuickPick::new()
			.items(sources.map(|path| {
				let text = path
					.strip_prefix(&root)
					.unwrap_or_else(|_| path.clone())
					.to_str()
					.unwrap()
					.to_owned();
				evscode::quick_pick::Item::new(path.to_str().unwrap().to_owned(), text)
			}))
			.show()
			.await
			.ok_or_else(evscode::E::cancel)?,
	);
	let codegen = CODEGEN_LIST[evscode::QuickPick::new()
		.ignore_focus_out()
		.match_on_all()
		.items(CODEGEN_LIST.iter().enumerate().map(|(i, codegen)| {
			let label = format!("{:?}", codegen);
			let description = clang::flags_codegen(*codegen).join(" ");
			evscode::quick_pick::Item::new(i.to_string(), label).description(description)
		}))
		.show()
		.await
		.ok_or_else(evscode::E::cancel)?
		.parse::<usize>()
		.unwrap()];
	build(source, codegen, true).await?;
	Ok(())
}

pub async fn build(
	source: impl util::MaybePath,
	codegen: Codegen,
	force_rebuild: bool,
) -> R<Executable>
{
	TELEMETRY.build_all.spark();
	let source = source.as_option_path();
	let _status = crate::STATUS.push(util::fmt_verb("Building", &source));
	let workspace_source = dir::solution()?;
	let source = source.unwrap_or(&workspace_source);
	if !fs::exists(source).await? {
		let pretty_source = source
			.strip_prefix(Path::from_native(evscode::workspace_root()?).as_ref())
			.wrap("tried to build source outside of project directory")?;
		return Err(evscode::E::error(format!("source `{}` does not exist", pretty_source)));
	}
	evscode::save_all().await?;
	let out = source.with_extension(&*EXECUTABLE_EXTENSION.get());
	if !force_rebuild && should_cache(source, out.as_ref()).await? {
		return Ok(Executable::new(out));
	}
	let standard = CPP_STANDARD.get();
	let flags = format!("{} {}", ADDITIONAL_CPP_FLAGS.get(), match codegen {
		Codegen::Debug => ADDITIONAL_CPP_FLAGS_DEBUG.get(),
		Codegen::Release => ADDITIONAL_CPP_FLAGS_RELEASE.get(),
		Codegen::Profile => ADDITIONAL_CPP_FLAGS_PROFILE.get(),
	});
	let flags = flags
		.split(' ')
		.map(|flag| flag.trim())
		.filter(|flag| !flag.is_empty())
		.collect::<Vec<_>>();
	let sources = [source];
	let status = compile(&sources, out.as_ref(), standard, codegen, &flags).await?;
	if !status.success {
		if let Some(error) = status.errors.first() {
			if let Some(location) = &error.location {
				if AUTO_MOVE_TO_ERROR.get() {
					evscode::open_editor(location.path.to_str().unwrap())
						.cursor(Position { line: location.line - 1, column: location.column - 1 })
						.open()
						.await?;
				}
			}
			Err(evscode::E::error(error.message.clone())
				.context("compilation error")
				.workflow_error())
		} else {
			Err(evscode::E::error("unrecognized compilation error").extended(status.stderr))
		}
	} else {
		if !status.warnings.is_empty() {
			let warnings = status.warnings;
			evscode::spawn(show_warnings(warnings));
		}
		Ok(status.executable)
	}
}

async fn should_cache(source: &Path, out: &Path) -> R<bool> {
	Ok(fs::exists(out).await?
		&& fs::metadata(source).await?.modified < fs::metadata(out).await?.modified)
}

pub fn exec_path(source: impl util::MaybePath) -> evscode::R<Path> {
	let workspace_source = dir::solution()?;
	let source = source.as_option_path().unwrap_or(&workspace_source);
	Ok(source.with_extension(&*EXECUTABLE_EXTENSION.get()))
}

async fn show_warnings(warnings: Vec<Message>) -> R<()> {
	if !AUTO_MOVE_TO_WARNING.get() {
		let message = format!(
			"{} compilation warning{}",
			warnings.len(),
			if warnings.len() == 1 { "" } else { "s" }
		);
		if evscode::Message::new(&message).warning().item((), "Show", false).show().await.is_none()
		{
			return Ok(());
		}
	}
	for (i, warning) in warnings.iter().enumerate() {
		if let Some(location) = &warning.location {
			evscode::open_editor(location.path.to_str().unwrap())
				.cursor(Position { line: location.line - 1, column: location.column - 1 })
				.open()
				.await?;
		}
		let msg = evscode::Message::new(&warning.message).warning();
		let choice = if i + 1 != warnings.len() {
			msg.item((), "Next", false).show().await
		} else {
			msg.show().await
		};
		if choice.is_none() {
			break;
		}
	}
	Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, evscode::Configurable)]
pub enum Standard {
	#[evscode(name = "C++03")]
	Cpp03,
	#[evscode(name = "C++11")]
	Cpp11,
	#[evscode(name = "C++14")]
	Cpp14,
	#[evscode(name = "C++17")]
	Cpp17,
	#[evscode(name = "C++20")]
	FutureCpp20,
}

#[derive(Clone, Copy, Debug)]
pub enum Codegen {
	Debug,
	Release,
	Profile,
}

pub static CODEGEN_LIST: &[Codegen] = &[Codegen::Debug, Codegen::Release, Codegen::Profile];

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
	pub success: bool,
	pub executable: Executable,
	pub errors: Vec<Message>,
	pub warnings: Vec<Message>,
	pub stderr: String,
}

pub static ALLOWED_EXTENSIONS: &[&str] = &["cpp", "cxx", "cc"];
