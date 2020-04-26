use crate::{
	dir, telemetry::TELEMETRY, util, util::{expand_path, fs, workspace_root, OS}
};
use evscode::{E, R};
use log::debug;
use std::collections::HashMap;

/// Path to your C++ template file. Set this to a value like `/home/jonsmith/template.cpp` or
/// `C:\Users\JohnSmith\template.cpp`. When opening a new task or contest, the contents of this file
/// will be copy-pasted into main.cpp. Be sure to leave an empty line in the main function, because
/// ICIE will then place the cursor there!
#[evscode::config]
static SOLUTION: evscode::Config<String> = "".to_owned();

/// Paths to additional C++ template files. A list of these will appear when you press Alt+=; if you
/// want to change the basic C++ template file, see the ICIE.Template.Solution configuration option
/// instead. If you see "Edit in settings.json", click it, then add a new entry starting with
/// "icie.template.list" and if you use autocomplete, VS Code should autofill the current config.
/// Replace the path placeholder with a path to your template file, or add more templates.
#[evscode::config]
pub static LIST: evscode::Config<HashMap<String, String>> = vec![
	("C++ Brute force".to_owned(), PSEUDOPATH_BRUTE_FORCE.to_owned()),
	("C++ Test generator".to_owned(), PSEUDOPATH_TEST_GENERATOR.to_owned()),
	("C++ Checker".to_owned(), PSEUDOPATH_CHECKER.to_owned()),
]
.into_iter()
.collect();

#[evscode::command(title = "ICIE Template instantiate", key = "alt+=")]
pub async fn instantiate() -> R<()> {
	let _status = crate::STATUS.push("Instantiating template");
	TELEMETRY.template_instantiate.spark();
	let templates = LIST.get();
	let template_id = evscode::QuickPick::new()
		.items(templates.iter().map(|(name, path)| {
			evscode::quick_pick::Item::new(name.clone(), name.clone()).description(additional_suggested_filename(&path))
		}))
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let template_path = &templates[&template_id];
	let template = load_additional(&template_path).await?;
	let filename = evscode::InputBox::new()
		.ignore_focus_out()
		.placeholder(&template.suggested_filename)
		.prompt("New file name")
		.value(&template.suggested_filename)
		.value_selection(0, template.suggested_filename.rfind('.').unwrap())
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let path = workspace_root()?.join(filename);
	if fs::exists(&path).await? {
		return Err(E::error("file already exists"));
	}
	fs::write(&path, template.code).await?;
	// FIXME: This for some reason failed to open the editor after the WASM rewrite.
	evscode::open_editor(&path).cursor(util::find_cursor_place(&path).await).open().await?;
	Ok(())
}

#[evscode::command(title = "ICIE Template configure")]
async fn configure() -> R<()> {
	TELEMETRY.template_configure.spark();
	let path = evscode::OpenDialog::new().action_label("Configure C++ template").show().await.ok_or_else(E::cancel)?;
	SOLUTION.update_global(&path).await;
	evscode::Message::new::<()>("C++ template configured successfully").show().await;
	Ok(())
}

pub struct LoadedTemplate {
	pub suggested_filename: String,
	pub code: String,
}

const PSEUDOPATH_BRUTE_FORCE: &str = "(replace this with a path to your brute force solution template)";
const PSEUDOPATH_TEST_GENERATOR: &str = "(replace this with a path to your test generator template)";
const PSEUDOPATH_CHECKER: &str = "(replace this with a path to your checker template)";

pub async fn load_solution() -> R<LoadedTemplate> {
	TELEMETRY.template_solution.spark();
	let template = match SOLUTION.get() {
		raw_path if !raw_path.is_empty() => {
			debug!("found solution path, unexpanded = {:?}", raw_path);
			let path = expand_path(&raw_path);
			debug!("found solution path, {:?}", path);
			TELEMETRY.template_solution_custom.spark();
			load_additional(&path).await.map_err(|e| e.action("Configure C++ template", configure()))?
		},
		_ => LoadedTemplate {
			suggested_filename: format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()),
			code: default_solution()?,
		},
	};
	Ok(template)
}

pub async fn load_additional(path: &str) -> R<LoadedTemplate> {
	let suggested_filename = additional_suggested_filename(path);
	let template = if path == PSEUDOPATH_BRUTE_FORCE {
		LoadedTemplate { suggested_filename, code: default_brute_force()? }
	} else if path == PSEUDOPATH_TEST_GENERATOR {
		LoadedTemplate { suggested_filename, code: default_test_generator()? }
	} else if path == PSEUDOPATH_CHECKER {
		LoadedTemplate { suggested_filename, code: default_checker()? }
	} else {
		let path = util::expand_path(path);
		let code = fs::read_to_string(&path).await?;
		LoadedTemplate { suggested_filename, code }
	};
	Ok(template)
}

fn additional_suggested_filename(path: &str) -> String {
	if path == PSEUDOPATH_BRUTE_FORCE {
		format!("{}.{}", dir::BRUTE_FORCE_STEM.get(), dir::CPP_EXTENSION.get())
	} else if path == PSEUDOPATH_TEST_GENERATOR {
		format!("{}.{}", dir::TEST_GENERATOR_STEM.get(), dir::CPP_EXTENSION.get())
	} else if path == PSEUDOPATH_CHECKER {
		format!("{}.{}", dir::CHECKER_STEM.get(), dir::CPP_EXTENSION.get())
	} else {
		let path = util::expand_path(path);
		path.file_name()
	}
}

pub fn default_solution() -> R<String> {
	generate(
		r#"// 💖 Hi, thanks for using ICIE! 💖
// 🔧 To use a custom code template, press Ctrl+Shift+P and select "ICIE Template configure" from the list 🔧
// 📝 If you spot any bugs or miss any features, create an issue at https://github.com/pustaczek/icie/issues 📝
// 🎉 ... or just ask me about anything via Discord at https://discord.gg/ffSEhtb 🎉
"#,
		false,
		"",
	)
}

fn default_brute_force() -> R<String> {
	generate(
		r#"// 💻 Here in brute-force.cpp, write a straighforward, brute-force solution that will be used to generate correct test outputs from inputs. 💻
// 💡 Then, press Alt+F9 to have ICIE automatically test your solution on thousands of tests! 💡
// 😕 Just write O(n^6), O(2^n) code; it doesn't need to be fast, but correct. 😕
"#,
		false,
		"",
	)
}

fn default_test_generator() -> R<String> {
	generate(
		r#"minstd_rand rng(chrono::high_resolution_clock::now().time_since_epoch().count());
template <typename T> T randint(T a, T b) { return uniform_int_distribution<T>(a, b)(rng); }
template <typename T> T uniform(T a, T b) { return uniform_real_distribution<T>(a, b)(rng); }

// 💻 Here in test-generator.cpp, write code that prints one random test input with cout/printf. 💻
// 💡 Then, press Alt+F9 to have ICIE automatically test your solution on thousands of tests! 💡
// 😕 How to randomize a dice roll: int dice = randint<int>(1, 6); 😕
// 😕 How to randomize a probability: double probability = uniform<double>(0., 1.); 😕
"#,
		false,
		"",
	)
}

fn default_checker() -> R<String> {
	generate(
		r#"// 💻 Here in checker.cpp, write code that checks whether your output is correct. 💻
// 🤢 This helps when there are many correct outputs, like 3.000000005 and 3. 🤢
// ⏲️ During contests, it's better go to Alt+0, move your mouse to the wrong output and click "Mark as Accepted" ⏲️
// 💡 Then, ICIE will check if checker.cpp file exists, and if so, use it for all tests! 💡
// 😕 You can read from input/... variables like from cin (`int aaa; your_output >> aaa;`). 😕
// 😕 If the output is correct, return 0 from main. If not, return 1. 😕
"#,
		true,
		r#"ifstream input(argv[1]), your_output(argv[2]), good_output(argv[3]);
"#,
	)
}

// TODO: Check Windows headers in ingen.
fn generate(prelude: &str, main_args: bool, main_prelude: &str) -> R<String> {
	// TODO: Does bits/stdc++.h work on macOS? I heard it doesn't.
	let includes = match OS::query()? {
		OS::Linux => "#include <bits/stdc++.h>",
		OS::Windows | OS::MacOS => "#include <iostream>\n#include <vector>\n#include <algorithm>\n#include <random>",
	};
	let main_args = if main_args { "int argc, char* argv[]" } else { "" };
	Ok(format!(
		r#"{}
using namespace std;

{}
int main({}) {{
    ios::sync_with_stdio(false);
    cin.tie(nullptr);
{}
}}
"#,
		includes, prelude, main_args, main_prelude,
	))
}
