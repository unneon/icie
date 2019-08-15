use crate::{dir, util};
use evscode::{E, R};
use std::{collections::HashMap, path::PathBuf};

/// A list of files used as code templates. If you see "Edit in settings.json", click it, then add a new entry starting with "icie.template.list" and if you use autocomplete, VS Code should autofill the current config. Replace the path placeholder with a path to your template file or add more templates
#[evscode::config]
pub static LIST: evscode::Config<HashMap<String, String>> = vec![("C++".to_owned(), BUILTIN_TEMPLATE_PSEUDOPATH.to_owned())].into_iter().collect();

#[evscode::command(title = "ICIE Template instantiate", key = "alt+=")]
pub fn instantiate() -> R<()> {
	let _status = crate::STATUS.push("Instantiating template");
	let templates = LIST.get();
	let qpick =
		evscode::QuickPick::new().items(templates.iter().map(|(name, _path)| evscode::quick_pick::Item::new(name.clone(), name.clone()))).build();
	let template_id = qpick.wait().ok_or_else(E::cancel)?;
	let template_path = &templates[&template_id];
	let tpl = load(&template_path)?;
	let filename = evscode::InputBox::new()
		.ignore_focus_out()
		.placeholder(tpl.suggested_filename.clone())
		.prompt("New file name")
		.value(tpl.suggested_filename.clone())
		.value_selection(0, tpl.suggested_filename.rfind('.').unwrap())
		.build()
		.wait()
		.ok_or_else(E::cancel)?;
	let path = evscode::workspace_root()?.join(filename);
	if path.exists() {
		return Err(E::error("file already exists"));
	}
	util::fs_write(&path, tpl.code)?;
	evscode::open_editor(&path).cursor(util::find_cursor_place(&path)).open();
	Ok(())
}

pub struct LoadedTemplate {
	pub suggested_filename: String,
	pub code: String,
}
pub fn load(path: &str) -> R<LoadedTemplate> {
	if path != BUILTIN_TEMPLATE_PSEUDOPATH {
		let path = PathBuf::from(shellexpand::tilde(path).into_owned());
		let suggested_filename = path.file_name().unwrap().to_str().unwrap().to_owned();
		let code = util::fs_read_to_string(path)?;
		Ok(LoadedTemplate { suggested_filename, code })
	} else {
		Ok(LoadedTemplate {
			suggested_filename: format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()),
			code: format!("{}\n", BUILTIN_TEMPLATE_CODE.trim()),
		})
	}
}

const BUILTIN_TEMPLATE_PSEUDOPATH: &str = "<enter a path to use a custom template>";
const BUILTIN_TEMPLATE_CODE: &str = r#"
#include <bits/stdc++.h>
using namespace std;

// 💖 Hi, thanks for using ICIE! 💖
// 🔧 To use a custom code template, set it in Settings(Ctrl+,) in "Icie Template List" entry 🔧
// 📝 If you spot any bugs or miss any features, create an issue at https://github.com/pustaczek/icie/issues 📝

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);
    
}
"#;
