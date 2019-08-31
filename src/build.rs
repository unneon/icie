use crate::{
	ci::{self, exec::Executable}, dir, util, STATUS
};
use evscode::{error::ResultExt, Position, R};
use std::{
	path::{Path, PathBuf}, time::SystemTime
};

/// When a compilation error appears, the cursor will automatically move to the file and location which caused the error. Regardless of this setting, an error message containing error details will be shown.
#[evscode::config]
static AUTO_MOVE_TO_ERROR: vscode::Config<bool> = true;

/// When a compilation warning appears, the cursor will automatically move to the file and location which caused the warning. If this is not set, a warning message will be shown with a "Show" button which will move the cursor to the location of the warning.
#[evscode::config]
static AUTO_MOVE_TO_WARNING: evscode::Config<bool> = true;

/// An extension used to denote executable files. For example, if this entry is set to "xyz", compiling a source file called main.cpp will create an executable called main.xyz.
#[evscode::config]
static EXECUTABLE_EXTENSION: evscode::Config<String> = "e";

/// C++ ISO language standard version. This corresponds to e.g. -std=c++17 flag on GCC/Clang. Be aware some of these options may not be supported by your compiler, which will result in an error.
#[evscode::config]
static CPP_STANDARD: evscode::Config<Standard> = Standard::Cpp17;

/// Additional C++ compilation flags. The flags will be appended to the command line after the standard, warning, debug symbols and optimization flags. These flags will be used both in Debug and Release profiles.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS: evscode::Config<String> = "";

/// Additional C++ compilation flags used in Debug profile. The flags will be appended to the command line after the standard, warning, debug symbols, optimization flags and profile-independent custom flags.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS_DEBUG: evscode::Config<String> = "";

/// Additional C++ compilation flags used in Release profile. The flags will be appended to the command line after the standard, warning, debug symbols, optimization flags and profile-independent custom flags.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS_RELEASE: evscode::Config<String> = "";

/// Additional C++ compilation flags used in Profile profile. The flags will be appended to the command line after the standard, warning, debug symbols, optimization flags and profile-independent custom flags.
#[evscode::config]
static ADDITIONAL_CPP_FLAGS_PROFILE: evscode::Config<String> = "";

#[evscode::command(title = "ICIE Manual Build", key = "alt+;")]
fn manual() -> evscode::R<()> {
	let _status = crate::STATUS.push("Manually building");
	let root = evscode::workspace_root()?;
	let sources = walkdir::WalkDir::new(&root)
		.follow_links(true)
		.into_iter()
		.filter(|entry| {
			entry
				.as_ref()
				.map(|entry| ci::cpp::ALLOWED_EXTENSIONS.iter().any(|ext| Some(std::ffi::OsStr::new(ext)) == entry.path().extension()))
				.unwrap_or(true)
		})
		.collect::<walkdir::Result<Vec<_>>>()
		.wrap("failed to scan tests directory")?;
	let source = PathBuf::from(
		evscode::QuickPick::new()
			.items(sources.into_iter().map(|entry| {
				let path = entry.path();
				let text = path.strip_prefix(&root).unwrap_or(path).to_str().unwrap();
				evscode::quick_pick::Item::new(path.to_str().unwrap(), text)
			}))
			.build()
			.spawn()
			.wait()
			.ok_or_else(evscode::E::cancel)?,
	);
	let codegen = &ci::cpp::CODEGEN_LIST[evscode::QuickPick::new()
		.ignore_focus_out()
		.match_on_all()
		.items(
			ci::cpp::CODEGEN_LIST
				.iter()
				.enumerate()
				.map(|(i, codegen)| evscode::quick_pick::Item::new(i.to_string(), format!("{:?}", codegen)).description(codegen.flags().join(" "))),
		)
		.build()
		.spawn()
		.wait()
		.ok_or_else(evscode::E::cancel)?
		.parse::<usize>()
		.unwrap()];
	build(source, codegen, true)?;
	Ok(())
}

pub fn build(source: impl util::MaybePath, codegen: &ci::cpp::Codegen, force_rebuild: bool) -> R<Executable> {
	let source = source.as_option_path();
	let _status = STATUS.push(util::fmt_verb("Building", &source));
	let workspace_source = dir::solution()?;
	let source = source.unwrap_or_else(|| workspace_source.as_path());
	if !source.exists() {
		let pretty_source = source.strip_prefix(evscode::workspace_root()?).wrap("tried to build source outside of project directory")?;
		return Err(evscode::E::error(format!("source `{}` does not exist", pretty_source.display())));
	}
	evscode::save_all().wait();
	let out = source.with_extension(&*EXECUTABLE_EXTENSION.get());
	if !force_rebuild && should_cache(&source, &out)? {
		return Ok(Executable::new(out));
	}
	let standard = CPP_STANDARD.get();
	let flags = format!("{} {}", ADDITIONAL_CPP_FLAGS.get(), match codegen {
		ci::cpp::Codegen::Debug => ADDITIONAL_CPP_FLAGS_DEBUG.get(),
		ci::cpp::Codegen::Release => ADDITIONAL_CPP_FLAGS_RELEASE.get(),
		ci::cpp::Codegen::Profile => ADDITIONAL_CPP_FLAGS_PROFILE.get(),
	});
	let flags = flags.split(' ').map(|flag| flag.trim()).filter(|flag| !flag.is_empty()).collect::<Vec<_>>();
	let status = ci::cpp::compile(&[&source], &out, &*standard, &codegen, &flags)?;
	if !status.success {
		if let Some(error) = status.errors.first() {
			if let Some(location) = &error.location {
				if *AUTO_MOVE_TO_ERROR.get() {
					evscode::open_editor(&location.path).cursor(Position { line: location.line - 1, column: location.column - 1 }).open().spawn();
				}
			}
			Err(evscode::E::error(error.message.clone()).context("compilation error").workflow_error())
		} else {
			Err(evscode::E::error("unrecognized compilation error").extended(status.stderr))
		}
	} else {
		if !status.warnings.is_empty() {
			let warnings = status.warnings;
			evscode::runtime::spawn(move || show_warnings(warnings));
		}
		Ok(status.executable)
	}
}

fn should_cache(source: &Path, out: &Path) -> R<bool> {
	Ok(out.exists() && {
		let source_mod = query_modification_time(source)?;
		let out_mod = query_modification_time(out)?;
		source_mod < out_mod
	})
}

fn query_modification_time(path: &Path) -> R<SystemTime> {
	path.metadata().wrap("file metadata query failed")?.modified().wrap("file modification time query failed")
}

pub fn exec_path(source: impl util::MaybePath) -> evscode::R<PathBuf> {
	let workspace_source = dir::solution()?;
	let source = source.as_option_path().unwrap_or_else(|| workspace_source.as_path());
	Ok(source.with_extension(&*EXECUTABLE_EXTENSION.get()))
}

fn show_warnings(warnings: Vec<ci::cpp::Message>) -> R<()> {
	if !*AUTO_MOVE_TO_WARNING.get() {
		let msg = evscode::Message::new(format!("{} compilation warning{}", warnings.len(), if warnings.len() == 1 { "" } else { "s" }))
			.warning()
			.item("show", "Show", false)
			.build();
		if msg.wait().is_none() {
			return Ok(());
		}
	}
	for (i, warning) in warnings.iter().enumerate() {
		if let Some(location) = &warning.location {
			evscode::open_editor(&location.path).cursor(Position { line: location.line - 1, column: location.column - 1 }).open().spawn();
		}
		let mut msg = evscode::Message::new(&warning.message).warning();
		if i + 1 != warnings.len() {
			msg = msg.item("next", "Next", false);
		}
		if msg.build().wait().is_none() {
			break;
		}
	}
	Ok(())
}

#[derive(Debug, evscode::Configurable)]
enum Standard {
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
impl ci::cpp::Standard for Standard {
	fn as_gcc_flag(&self) -> &'static str {
		match self {
			Standard::Cpp03 => "-std=c++03",
			Standard::Cpp11 => "-std=c++11",
			Standard::Cpp14 => "-std=c++14",
			Standard::Cpp17 => "-std=c++17",
			Standard::FutureCpp20 => "-std=c++2a",
		}
	}
}
