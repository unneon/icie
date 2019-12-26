use crate::{
	dir, telemetry::TELEMETRY, util, util::{fs, path::Path, OS}
};
use evscode::{E, R};
use std::collections::HashMap;

/// A list of files used as code templates. If you see "Edit in settings.json", click it, then add a
/// new entry starting with "icie.template.list" and if you use autocomplete, VS Code should
/// autofill the current config. Replace the path placeholder with a path to your template file or
/// add more templates
#[evscode::config]
pub static LIST: evscode::Config<HashMap<String, String>> =
	vec![("C++".to_owned(), BUILTIN_TEMPLATE_PSEUDOPATH.to_owned())].into_iter().collect();

#[evscode::command(title = "ICIE Template instantiate", key = "alt+=")]
async fn instantiate() -> R<()> {
	let _status = crate::STATUS.push("Instantiating template");
	TELEMETRY.template_instantiate.spark();
	let templates = LIST.get();
	let template_id = evscode::QuickPick::new()
		.items(
			templates
				.iter()
				.map(|(name, _path)| evscode::quick_pick::Item::new(name.clone(), name.clone())),
		)
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let template_path = &templates[&template_id];
	let tpl = load(&template_path).await?;
	let filename = evscode::InputBox::new()
		.ignore_focus_out()
		.placeholder(&tpl.suggested_filename)
		.prompt("New file name")
		.value(&tpl.suggested_filename)
		.value_selection(0, tpl.suggested_filename.rfind('.').unwrap())
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let path = Path::from_native(evscode::workspace_root()?).join(filename);
	if fs::exists(&path).await? {
		return Err(E::error("file already exists"));
	}
	fs::write(&path, tpl.code).await?;
	// FIXME: This for some reason failed to open the editor after the WASM rewrite.
	evscode::open_editor(&path).cursor(util::find_cursor_place(&path).await).open().await?;
	Ok(())
}

pub struct LoadedTemplate {
	pub suggested_filename: String,
	pub code: String,
}
pub async fn load(path: &str) -> R<LoadedTemplate> {
	TELEMETRY.template_load.spark();
	if path != BUILTIN_TEMPLATE_PSEUDOPATH {
		TELEMETRY.template_load_custom.spark();
		let path = util::expand_path(path);
		let suggested_filename = path.file_name();
		let code = fs::read_to_string(&path).await?;
		Ok(LoadedTemplate { suggested_filename, code })
	} else {
		TELEMETRY.template_load_builtin.spark();
		Ok(LoadedTemplate {
			suggested_filename: format!(
				"{}.{}",
				dir::SOLUTION_STEM.get(),
				dir::CPP_EXTENSION.get()
			),
			code: format!("{}\n", builtin_template()?),
		})
	}
}

const BUILTIN_TEMPLATE_PSEUDOPATH: &str = "<enter a path to use a custom template>";

fn builtin_template() -> R<String> {
	let includes = match OS::query()? {
		OS::Linux => "#include <bits/stdc++.h>",
		OS::Windows | OS::MacOS => "#include <iostream>\n#include <vector>\n#include <algorithm>",
	};
	Ok(format!(
		r#"
{}
using namespace std;

// 💖 Hi, thanks for using ICIE! 💖
// 🔧 To use a custom code template, set it in Settings(Ctrl+,) in "Icie Template List" entry 🔧
// 📝 If you spot any bugs or miss any features, create an issue at https://github.com/pustaczek/icie/issues 📝

int main() {{
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

}}
"#,
		includes
	))
}
