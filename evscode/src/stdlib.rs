//! Extension API used for interacting with VS Code.

pub mod console;
pub mod input_box;
pub mod message;
pub mod open_dialog;
pub mod open_editor;
pub mod output_channel;
pub mod progress;
pub mod quick_pick;
pub mod state;
pub mod terminal;
pub mod types;
pub mod webview;
pub mod treedata;

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
pub use treedata::TreeData;

use crate::{error::ResultExt, E, R};
use std::cell::RefCell;
use wasm_bindgen::{closure::Closure, JsValue};

/// Save all modified files in the workspace.
///
/// The returned future will wait until all the files are actually saved.
///
/// According to the [source code][1], this will fail without a specific reason if any operation
/// fails.
///
/// [1]: https://github.com/microsoft/vscode/blob/c467419e0e3023668b8f031d3be768b79eeb1eb7/src/vs/workbench/api/browser/mainThreadWorkspace.ts#L207-L211
pub async fn save_all() -> R<()> {
	if vscode_sys::workspace::save_all(false).await { Ok(()) } else { Err(E::error("could not save all files")) }
}

/// Open a folder in a new or existing VS Code window.
pub async fn open_folder(path: &str, in_new_window: bool) {
	let uri = vscode_sys::Uri::file(path);
	let in_new_window = JsValue::from_bool(in_new_window);
	vscode_sys::commands::execute_command("vscode.openFolder", js_sys::Array::of2(&uri, &in_new_window)).await;
}

/// Open an external item(e.g. http/https/mailto URL), using the default system application.
///
/// Use [`open_editor()`] to open text files instead.
pub async fn open_external(url: &str) -> R<()> {
	let uri = vscode_sys::Uri::parse(url, true);
	let success = vscode_sys::env::open_external(&uri).await;
	if success {
		Ok(())
	} else {
		let url = url.to_owned();
		Err(E::error(format!("could not open a link in browser {}", url))
			.action("Try again", async move { open_external(&url).await }))
	}
}

/// Get the text present in the editor of a given path.
pub async fn query_document_text(path: &str) -> R<String> {
	let doc = vscode_sys::workspace::open_text_document(path).await?;
	Ok(doc.text())
}

/// Make an edit action that consists of pasting a given text in a given position in a given file.
///
/// The indices in the (row, column) tuple are 0-based.
pub async fn edit_paste(path: &str, text: &str, position: (usize, usize)) -> R<()> {
	let text = text.to_owned();
	let doc = vscode_sys::workspace::open_text_document(path).await.expect("unwrap in evscode.edit_paste");
	let edi = vscode_sys::window::show_text_document(&doc).await;
	let suc = edi
		.edit(&Closure::wrap(Box::new(move |edit_builder: &vscode_sys::TextEditorEdit| {
			edit_builder.insert(&vscode_sys::Position::new(position.0, position.1), &text);
		}) as Box<dyn FnMut(&vscode_sys::TextEditorEdit)>))
		.await;
	if suc { Ok(()) } else { Err(E::error("could not apply requested edits")) }
}

/// Get the path to workspace folder.
/// Returns an error if no folder is opened.
pub fn workspace_root() -> R<String> {
	vscode_sys::workspace::ROOT_PATH.as_string().wrap("this operation requires a folder to be open")
}

/// Get the path to the root directory of the extension installation.
pub fn extension_root() -> &'static str {
	crate::glue::EXTENSION_PATH.get().unwrap()
}

/// Get the path to the currently edited file.
pub async fn active_editor_file() -> Option<String> {
	vscode_sys::window::ACTIVE_TEXT_EDITOR.as_ref().map(|edi| edi.document().file_name())
}

/// Set the OS clipboard content to a given value.
pub async fn clipboard_write(val: &str) {
	vscode_sys::env::CLIPBOARD.write_text(val).await;
}

/// Return an URI pointing to a given path for use with webviews.
pub fn asset(name: &str) -> String {
	let asset_path = node_sys::path::join(extension_root(), name);
	format!("vscode-resource:{}", asset_path)
}

/// Return an URI pointing to a given path for use with webviews.
pub fn get_path(name: &str) -> String {
	node_sys::path::join(extension_root(), name)
}
/// Set the status message on a global widget.
///
/// This will interfere with other threads, use [`crate::goodies::MultiStatus`] instead.
pub fn status(msg: Option<&str>) {
	STATUS.with(|status| {
		let status = status.borrow();
		let status = status.as_ref().unwrap();
		match msg {
			Some(msg) => {
				status.set_text(msg);
				status.show();
			},
			None => status.hide(),
		}
	});
}

thread_local! {
	pub(crate) static STATUS: RefCell<Option<vscode_sys::StatusBarItem>> = RefCell::new(None);
}
