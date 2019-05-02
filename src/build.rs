use crate::STATUS;
use ci::lang::Language;
use evscode::R;
use std::path::Path;

#[evscode::config(description = "Auto move to error position")]
static AUTO_MOVE_TO_ERROR: vscode::Config<bool> = true;

#[evscode::config(description = "Auto move to warning position")]
static AUTO_MOVE_TO_WARNING: evscode::Config<bool> = true;

#[evscode::config(description = "Executable extension")]
static EXECUTABLE_EXTENSION: evscode::Config<String> = "e";

#[evscode::config(description = "C++ language standard")]
static CPP_STANDARD: evscode::Config<CppStandard> = CppStandard::Cpp17;

fn build(sources: &[&Path]) -> R<ci::exec::Executable> {
	evscode::save_all().wait();
	let _status = STATUS.push("Building");
	let lang = ci::lang::CPP;
	let out = sources[0].with_extension(EXECUTABLE_EXTENSION.get());
	if out.exists() {
		let source_mods = sources.iter().map(|source| Ok(source.metadata()?.modified()?)).collect::<R<Vec<_>>>()?;
		if out.metadata()?.modified()? > *source_mods.iter().max().unwrap() {
			return Ok(ci::exec::Executable::new(out));
		}
	}
	let standard = CPP_STANDARD.get().to_ci();
	let status = lang.compile(&sources, &out, &standard, &ci::lang::Codegen::Debug)?;
	if !status.success {
		if let Some(error) = status.errors.first() {
			if AUTO_MOVE_TO_ERROR.get() {
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

fn show_warnings(warnings: Vec<ci::lang::Message>) -> R<()> {
	if !AUTO_MOVE_TO_WARNING.get() {
		let msg = evscode::InfoMessage::new(format!("{} compilation warning{}", warnings.len(), if warnings.len() == 1 { "" } else { "s" }))
			.warning()
			.item("show", "Show", false)
			.spawn();
		if msg.wait().is_none() {
			return Ok(());
		}
	}
	for (i, warning) in warnings.iter().enumerate() {
		evscode::open_editor(&warning.path, Some(warning.line - 1), Some(warning.column - 1));
		let mut msg = evscode::InfoMessage::new(warning.message.as_str()).warning();
		if i + 1 != warnings.len() {
			msg = msg.item("next", "Next", false);
		}
		if msg.spawn().wait().is_none() {
			break;
		}
	}
	Ok(())
}

pub fn solution() -> R<ci::exec::Executable> {
	build(&[&crate::dir::solution()])
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
