use crate::STATUS;
use ci::lang::Language;
use evscode::R;
use std::path::Path;

#[evscode::config(description = "Auto move to error position")]
static AUTO_MOVE_TO_ERROR: vscode::Config<bool> = true.into();

#[evscode::config(description = "Auto move to warning position")]
static AUTO_MOVE_TO_WARNING: evscode::Config<bool> = true.into();

#[evscode::config(description = "Executable extension")]
static EXECUTABLE_EXTENSION: evscode::Config<String> = "e".into();

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
	let status = lang.compile(&sources, &out, &ci::lang::CppStandard::Std17, &ci::lang::Codegen::Debug)?;
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
