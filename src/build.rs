use crate::{ci, dir, util, STATUS};
use ci::lang::Language;
use evscode::R;
use std::path::PathBuf;

#[evscode::config(description = "Auto move to error position")]
static AUTO_MOVE_TO_ERROR: vscode::Config<bool> = true;

#[evscode::config(description = "Auto move to warning position")]
static AUTO_MOVE_TO_WARNING: evscode::Config<bool> = true;

#[evscode::config(description = "Executable extension")]
static EXECUTABLE_EXTENSION: evscode::Config<String> = "e";

#[evscode::config(description = "C++ language standard")]
static CPP_STANDARD: evscode::Config<CppStandard> = CppStandard::Cpp17;

#[evscode::config(description = "Additional C++ compilation flags. The flags will be appended after ICIE-sourced on both debug and release builds.")]
static ADDITIONAL_CPP_FLAGS: evscode::Config<String> = "";

pub fn build(source: impl util::MaybePath, codegen: ci::lang::Codegen) -> R<ci::exec::Executable> {
	let source = source.as_option_path();
	let _status = STATUS.push(util::fmt_verb("Building", &source));
	let workspace_source = dir::solution()?;
	let source = source.unwrap_or_else(|| workspace_source.as_path());
	if !source.exists() {
		let pretty_source = source.strip_prefix(evscode::workspace_root()?)?;
		return Err(evscode::E::error(format!("source `{}` does not exist", pretty_source.display())));
	}
	evscode::save_all().wait();
	let lang = ci::lang::CPP;
	let out = source.with_extension(&*EXECUTABLE_EXTENSION.get());
	if out.exists() {
		let source_mods = &[&source].iter().map(|source| Ok(source.metadata()?.modified()?)).collect::<R<Vec<_>>>()?;
		if out.metadata()?.modified()? > *source_mods.iter().max().unwrap() {
			return Ok(ci::exec::Executable::new(out));
		}
	}
	let standard = CPP_STANDARD.get().to_ci();
	let flags = ADDITIONAL_CPP_FLAGS.get();
	let flags = flags.split(' ').map(|flag| flag.trim()).filter(|flag| !flag.is_empty()).collect::<Vec<_>>();
	let status = lang.compile(&[&source], &out, &standard, &codegen, &flags)?;
	if !status.success {
		if let Some(error) = status.errors.first() {
			if *AUTO_MOVE_TO_ERROR.get() {
				evscode::open_editor(&error.path, Some(error.line - 1), Some(error.column - 1));
			}
			Err(evscode::E::error(error.message.clone()))
		} else {
			Err(evscode::E::error("unrecognized compilation error"))
		}
	} else {
		if !status.warnings.is_empty() {
			let warnings = status.warnings;
			evscode::spawn(move || show_warnings(warnings));
		}
		Ok(status.executable)
	}
}

pub fn exec_path(source: impl util::MaybePath) -> evscode::R<PathBuf> {
	let workspace_source = dir::solution()?;
	let source = source.as_option_path().unwrap_or_else(|| workspace_source.as_path());
	Ok(source.with_extension(&*EXECUTABLE_EXTENSION.get()))
}

#[evscode::command(title = "ICIE Build", key = "alt+;")]
pub fn debug() -> evscode::R<()> {
	build(Option::<PathBuf>::None, ci::lang::Codegen::Debug)?;
	Ok(())
}
#[evscode::command(title = "ICIE Build (current editor)", key = "alt+\\ alt+;")]
pub fn debug_current() -> evscode::R<()> {
	build(util::active_tab()?, ci::lang::Codegen::Debug)?;
	Ok(())
}
#[evscode::command(title = "ICIE Build Release", key = "shift+alt+;")]
pub fn release() -> evscode::R<()> {
	build(Option::<PathBuf>::None, ci::lang::Codegen::Release)?;
	Ok(())
}
#[evscode::command(title = "ICIE Build Release (current editor)", key = "alt+\\ shift+alt+;")]
pub fn release_current() -> evscode::R<()> {
	build(util::active_tab()?, ci::lang::Codegen::Release)?;
	Ok(())
}

fn show_warnings(warnings: Vec<ci::lang::Message>) -> R<()> {
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
		evscode::open_editor(&warning.path, Some(warning.line - 1), Some(warning.column - 1));
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

#[derive(Clone, Debug, evscode::Configurable)]
enum CppStandard {
	#[evscode(name = "C++03")]
	Cpp03,
	#[evscode(name = "C++11")]
	Cpp11,
	#[evscode(name = "C++14")]
	Cpp14,
	#[evscode(name = "C++17")]
	Cpp17,
	#[evscode(name = "C++20")]
	Cpp20,
}
impl CppStandard {
	fn to_ci(&self) -> ci::lang::CppStandard {
		match self {
			CppStandard::Cpp03 => ci::lang::CppStandard::Std03,
			CppStandard::Cpp11 => ci::lang::CppStandard::Std11,
			CppStandard::Cpp14 => ci::lang::CppStandard::Std14,
			CppStandard::Cpp17 => ci::lang::CppStandard::Std17,
			CppStandard::Cpp20 => ci::lang::CppStandard::Std2a,
		}
	}
}