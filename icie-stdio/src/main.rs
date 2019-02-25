extern crate icie_logic;
#[macro_use]
extern crate json;

use icie_logic::{Impulse, Reaction};
use std::{
	io::{self, BufRead}, path::PathBuf, sync::Arc, thread
};

fn main() {
	let icie1 = Arc::new(icie_logic::Handle::spawn());
	let icie2 = icie1.clone();
	let t1 = thread::spawn(move || {
		for line in io::stdin().lock().lines() {
			let line = line.expect("stdin failure");
			let imp = json::parse(&line).expect("invalid impulse JSON");
			let impulse = match imp["tag"].as_str() {
				Some("quick_pick") => Impulse::QuickPick {
					response: imp["response"].as_str().map(String::from),
				},
				Some("input_box") => Impulse::InputBox {
					response: imp["response"].as_str().map(String::from),
				},
				Some("trigger_build") => Impulse::TriggerBuild,
				Some("workspace_info") => Impulse::WorkspaceInfo {
					root_path: imp["root_path"].as_str().map(String::from),
				},
				Some("trigger_test") => Impulse::TriggerTest,
				Some("saved_all") => Impulse::SavedAll,
				Some("trigger_init") => Impulse::TriggerInit,
				Some("trigger_submit") => Impulse::TriggerSubmit,
				Some("trigger_manual_submit") => Impulse::TriggerManualSubmit,
				Some("trigger_template_instantiate") => Impulse::TriggerTemplateInstantiate,
				Some("trigger_testview") => Impulse::TriggerTestview,
				Some("trigger_rr") => Impulse::TriggerRR {
					in_path: PathBuf::from(imp["in_path"].as_str().expect("invalid impulse JSON trigger_rr")),
				},
				Some("new_test") => Impulse::NewTest {
					input: imp["input"].as_str().expect("invalid impulse JSON trigger_new_test").to_owned(),
					desired: imp["desired"].as_str().expect("invalid impulse JSON trigger_new_test").to_owned(),
				},
				Some("message_response") => Impulse::MessageResponse {
					id: imp["id"].as_str().expect("invalid impulse JSON message_response").to_owned(),
					response: imp["response"].as_str().map(String::from),
				},
				_ => panic!("unrecognized impulse tag {:?}", imp["tag"]),
			};
			icie1.send(impulse);
		}
	});
	let t2 = thread::spawn(move || loop {
		let reaction = icie2.recv();
		let rea: json::JsonValue = match reaction {
			Reaction::Status { message } => object! {
				"tag" => "status",
				"message" => message,
			},
			Reaction::Message { message, kind, items, modal } => object! {
				"tag" => "message",
				"message" => message,
				"kind" => match kind {
					icie_logic::vscode::MessageKind::Info => "info",
					icie_logic::vscode::MessageKind::Warning => "warning",
					icie_logic::vscode::MessageKind::Error => "error",
				},
				"items" => items.map(|items| object! {
					"id" => items.id,
					"list" => items.list.into_iter().map(|item| object! {
						"title" => item.title,
						"is_close_affordance" => item.is_close_affordance,
						"id" => item.id,
					}).collect::<Vec<_>>(),
				}),
				"modal" => modal,
			},
			Reaction::QuickPick { items } => object! {
				"tag" => "quick_pick",
				"items" => items.into_iter().map(|item| object! {
					"label" => item.label,
					"description" => item.description,
					"detail" => item.detail,
					"id" => item.id,
				}).collect::<Vec<_>>(),
			},
			Reaction::InputBox { options } => object! {
				"tag" => "input_box",
				"prompt" => options.prompt,
				"placeholder" => options.placeholder,
				"password" => options.password,
				"ignoreFocusOut" => options.ignore_focus_out,
			},
			Reaction::ConsoleLog { message } => object! {
				"tag" => "console_log",
				"message" => message,
			},
			Reaction::SaveAll => object! {
				"tag" => "save_all",
			},
			Reaction::OpenFolder { path, in_new_window } => object! {
				"tag" => "open_folder",
				"path" => path.to_str().unwrap(),
				"in_new_window" => in_new_window,
			},
			Reaction::ConsoleError { message } => object! {
				"tag" => "console_error",
				"message" => message,
			},
			Reaction::OpenEditor { path, row, column } => object! {
				"tag" => "open_editor",
				"path" => path.to_str().unwrap(),
				"row" => row,
				"column" => column,
			},
			Reaction::ProgressStart { id, title } => object! {
				"tag" => "progress_start",
				"id" => id,
				"title" => title,
			},
			Reaction::ProgressUpdate { id, increment, message } => object! {
				"tag" => "progress_update",
				"id" => id,
				"increment" => increment,
				"message" => message,
			},
			Reaction::ProgressEnd { id } => object! {
				"tag" => "progress_end",
				"id" => id,
			},
			Reaction::TestviewFocus => object! {
				"tag" => "testview_focus",
			},
			Reaction::TestviewUpdate { tree } => object! {
				"tag" => "testview_update",
				"tree" => serialize_tree(tree),
			},
		};
		println!("{}", rea.to_string());
	});
	t1.join().expect("impulse thread errored");
	t2.join().expect("reaction thread errored");
}

fn serialize_tree(tree: icie_logic::testview::Tree) -> json::JsonValue {
	match tree {
		icie_logic::testview::Tree::Test {
			name,
			input,
			output,
			desired,
			timing,
			in_path,
			outcome,
		} => object! {
			"name" => name,
			"input" => input,
			"output" => output,
			"desired" => desired,
			"timing" => timing.map(|t| t.as_secs() * 1000 + t.subsec_millis() as u64),
			"in_path" => in_path.to_str().expect("non utf8 path in Rust-TS conversion"),
			"outcome" => match outcome {
				icie_logic::TestResult::Accept => "accept",
				icie_logic::TestResult::WrongAnswer => "wrong_answer",
				icie_logic::TestResult::RuntimeError => "runtime_error",
				icie_logic::TestResult::IgnoredNoOut => "ignored_no_out",
			},
		},
		icie_logic::testview::Tree::Directory { files } => json::from(files.into_iter().map(serialize_tree).collect::<Vec<_>>()),
	}
}
