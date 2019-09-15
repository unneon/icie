//! Extension API used for interacting with VS Code.

pub mod console;
pub mod input_box;
pub mod message;
pub mod open_dialog;
pub mod open_editor;
pub mod progress;
pub mod quick_pick;
pub mod state;
pub mod terminal;
pub mod types;
pub mod webview;

pub use input_box::InputBox;
pub use message::Message;
pub use open_dialog::OpenDialog;
pub use open_editor::open_editor;
pub use progress::Progress;
pub use quick_pick::QuickPick;
pub use state::State;
pub use terminal::Terminal;
pub use types::*;
pub use webview::Webview;

use crate::{future::Pong, internal::executor::send_object, E, R};
use std::{
	borrow::Borrow, path::{Path, PathBuf}
};

/// Save all modified files in the workspace.
///
/// The returned future will wait until all the files are actually saved.
///
/// According to the [source code][1], this will fail without a specific reason if any operation fails.
///
/// [1]: https://github.com/microsoft/vscode/blob/c467419e0e3023668b8f031d3be768b79eeb1eb7/src/vs/workbench/api/browser/mainThreadWorkspace.ts#L207-L211
pub async fn save_all() -> R<()> {
	let pong = Pong::new();
	send_object(json::object! {
		"tag" => "save_all",
		"aid" => pong.aid(),
	});
	if pong.await.as_bool().expect("internal evscode type error in save_all") { Ok(()) } else { Err(E::error("could not save all files")) }
}

/// Open a folder in a new or existing VS Code window.
pub fn open_folder(path: impl AsRef<Path>, in_new_window: bool) {
	send_object(json::object! {
		"tag" => "open_folder",
		"path" => path.as_ref().to_str().expect("evscode::open_folder non-utf8 path"),
		"in_new_window" => in_new_window,
	});
}

/// Open an external item(e.g. http/https/mailto URL), using the default system application.
///
/// Use [`open_editor()`] to open text files instead.
pub async fn open_external(url: &str) -> R<()> {
	let pong = Pong::new();
	send_object(json::object! {
		"tag" => "open_external",
		"url" => url,
		"aid" => pong.aid(),
	});
	if pong.await.as_bool().expect("internal evscode type error in open_external") {
		Ok(())
	} else {
		Err(E::error(format!("could not open external URL {}", url)))
	}
}

/// Get the text present in the editor of a given path.
pub async fn query_document_text(path: &Path) -> String {
	let pong = Pong::new();
	send_object(json::object! {
		"tag" => "query_document_text",
		"path" => path.to_str().expect("evscode::query_document_text_lazy non-utf8 path"),
		"aid" => pong.aid(),
	});
	pong.await.as_str().expect("internal evscode type error in query_document_text").to_string()
}

/// Make an edit action that consists of pasting a given text in a given position in a given file.
///
/// The indices in the (row, column) tuple are 0-based.
pub async fn edit_paste(path: &Path, text: &str, position: (usize, usize)) {
	let pong = Pong::new();
	send_object(json::object! {
		"tag" => "edit_paste",
		"position" => json::object! {
			"line" => position.0,
			"character" => position.1,
		},
		"text" => text,
		"path" => path.to_str().expect("evscode::edit_paste_lazy non-utf8 path"),
		"aid" => pong.aid(),
	});
	pong.await;
}

/// Get the path to workspace folder.
/// Returns an error if no folder is opened.
pub fn workspace_root() -> R<PathBuf> {
	crate::internal::executor::ROOT_WORKSPACE.lock().unwrap().clone().ok_or_else(|| E::error("this operation requires a folder to be open"))
}

/// Get the path to the root directory of the extension installation.
pub fn extension_root() -> PathBuf {
	crate::internal::executor::ROOT_EXTENSION.lock().unwrap().clone().unwrap()
}

/// Get the path to the currently edited file.
pub async fn active_editor_file() -> Option<PathBuf> {
	let pong = Pong::new();
	send_object(json::object! {
		"tag" => "active_editor_file",
		"aid" => pong.aid(),
	});
	pong.await.as_str().map(PathBuf::from)
}

/// Set the OS clipboard content to a given value.
pub async fn clipboard_write(val: &str) {
	let pong = Pong::new();
	send_object(json::object! {
		"tag" => "clipboard_write",
		"aid" => pong.aid(),
		"val" => val,
	});
	pong.await;
}

/// Return an URI pointing to a given path for use with webviews.
pub fn asset(rel_path: impl AsRef<Path>) -> String {
	format!("vscode-resource://{}", extension_root().join("data/assets").join(rel_path.as_ref()).to_str().unwrap())
}

/// Set the status message on a global widget.
///
/// This will interfere with other threads, use [`crate::StackedStatus`] instead.
pub fn status(msg: Option<&str>) {
	send_object(json::object! {
		"tag" => "status",
		"message" => msg,
	})
}

/// Sends a telemetry event through [vscode-extension-telemetry](https://github.com/microsoft/vscode-extension-telemetry).
pub fn telemetry<'a>(
	event_name: &'a str,
	properties: impl IntoIterator<Item=impl Borrow<(&'a str, &'a str)>>,
	measurements: impl IntoIterator<Item=impl Borrow<(&'a str, f64)>>,
) {
	send_object(json::object! {
		"tag" => "telemetry_event",
		"event_name" => event_name,
		"properties" => properties.into_iter().map(|prop| (prop.borrow().0, prop.borrow().1)).collect::<json::object::Object>(),
		"measurements" => measurements.into_iter().map(|meas| (meas.borrow().0, meas.borrow().1)).collect::<json::object::Object>(),
	});
}
