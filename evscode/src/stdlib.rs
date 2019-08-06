pub mod console;
pub mod input_box;
pub mod message;
pub mod open_dialog;
pub mod progress;
pub mod quick_pick;
pub mod state;
pub mod terminal;
pub mod webview;

pub use input_box::InputBox;
pub use message::Message;
pub use open_dialog::OpenDialog;
pub use progress::Progress;
pub use quick_pick::QuickPick;
pub use state::State;
pub use terminal::Terminal;
pub use webview::Webview;

use crate::{internal::executor::send_object, LazyFuture, E, R};
use json::JsonValue;
use std::path::{Path, PathBuf};

/// Represents a line and character position.
pub struct Position {
	/// The number of characters from the left.
	pub column: usize,
	/// The line number.
	pub line: usize,
}

impl From<Position> for JsonValue {
	fn from(pos: Position) -> Self {
		json::object! {
			"column" => pos.column,
			"line" => pos.line,
		}
	}
}

/// Represents an ordered pair of two positions.
pub struct Range {
	/// The beginning of the range.
	pub start: Position,
	/// The ending of the range, this position on the boundary.
	pub end: Position,
}

impl From<Range> for JsonValue {
	fn from(r: Range) -> Self {
		json::object! {
			"start" => r.start,
			"end" => r.end,
		}
	}
}

/// View column where a tab can appear.
#[derive(Clone)]
pub enum Column {
	/// View column of the currently active tab.
	Active,
	/// View column to the right of the currently active tab.
	/// This can create new columns depending on what is currently selected.
	/// Examples:
	/// - One column exists: the column is split in half, the right half is taken by the new webview.
	/// - Two columns exist, left active: the new webvieb is added to the right column as a new tab.
	/// - Two columns exist, right active: the right column is split in half, the right half of the right half is taken by the new webview.
	Beside,
	/// First, leftmost column.
	One,
	/// Second column.
	Two,
	/// Third column.
	Three,
	/// Fourth column.
	Four,
	/// Fifth column.
	Five,
	/// Sixth column.
	Six,
	/// Seventh column.
	Seven,
	/// Eighth column.
	Eight,
	/// Ninth column.
	Nine,
}
impl From<i32> for Column {
	fn from(x: i32) -> Self {
		use Column::*;
		match x {
			1 => One,
			2 => Two,
			3 => Three,
			4 => Four,
			5 => Five,
			6 => Six,
			7 => Seven,
			8 => Eight,
			9 => Nine,
			_ => panic!("view column number should be in [1, 9]"),
		}
	}
}
impl From<Column> for JsonValue {
	fn from(col: Column) -> JsonValue {
		use Column::*;
		json::from(match col {
			Active => "active",
			Beside => "beside",
			Eight => "eight",
			Five => "five",
			Four => "four",
			Nine => "nine",
			One => "one",
			Seven => "seven",
			Six => "six",
			Three => "three",
			Two => "two",
		})
	}
}

/// Save all modified files in the workspace.
pub fn save_all() -> LazyFuture<()> {
	LazyFuture::new_vscode(
		|aid| {
			send_object(json::object! {
				"tag" => "save_all",
				"aid" => aid,
			})
		},
		|_| (),
	)
}

/// Open a folder in a new or existing VS Code window.
pub fn open_folder(path: impl AsRef<Path>, in_new_window: bool) {
	send_object(json::object! {
		"tag" => "open_folder",
		"path" => path.as_ref().to_str().expect("evscode::open_folder non-utf8 path"),
		"in_new_window" => in_new_window,
	});
}

/// Open a file, possibly setting the cursor to the given position.
/// The indices are 0-based.
pub fn open_editor(
	path: impl AsRef<Path>,
	cursor: Option<Position>,
	preserve_focus: Option<bool>,
	preview: Option<bool>,
	selection: Option<Range>,
	view_column: Option<Column>,
) {
	send_object(json::object! {
		"tag" => "open_editor",
		"path" => path.as_ref().to_str().expect("evscode::open_editor non-utf8 path"),
		"cursor" => cursor,
		"preserve_focus" => preserve_focus,
		"preview" => preview,
		"selection" => selection,
		"view_column" => view_column,
	});
}

/// Open an external item(e.g. http/https/mailto URL), using the default system application.
/// Use [`open_editor`] to open text files instead.
pub fn open_external(url: impl AsRef<str>) -> LazyFuture<R<()>> {
	let url = url.as_ref().to_owned();
	LazyFuture::new_vscode(
		{
			let url = url.clone();
			move |aid| {
				send_object(json::object! {
					"tag" => "open_external",
					"url" => url,
					"aid" => aid,
				})
			}
		},
		{
			let url = url;
			move |raw| {
				if raw.as_bool().expect("evscode::open_external raw not a [bool]") {
					Ok(())
				} else {
					Err(E::error(format!("could not open external URL {}", url)))
				}
			}
		},
	)
}

/// Get the text present in the editor of a given path.
pub fn query_document_text(path: impl AsRef<Path>+'static) -> LazyFuture<String> {
	LazyFuture::new_vscode(
		move |aid| {
			send_object(json::object! {
				"tag" => "query_document_text",
				"path" => path.as_ref().to_str().expect("evscode::query_document_text_lazy non-utf8 path"),
				"aid" => aid,
			})
		},
		|raw| raw.as_str().expect("evscode::query_document_text_lazy raw not a [str]").to_string(),
	)
}

/// Make an edit action that consists of pasting a given text in a given position in a given file.
/// The indices in the (row, column) tuple are 0-based.
pub fn edit_paste(path: impl AsRef<Path>+'static, text: impl AsRef<str>+'static, position: (usize, usize)) -> LazyFuture<()> {
	LazyFuture::new_vscode(
		move |aid| {
			send_object(json::object! {
				"tag" => "edit_paste",
				"position" => json::object! {
					"line" => position.0,
					"character" => position.1,
				},
				"text" => text.as_ref(),
				"path" => path.as_ref().to_str().expect("evscode::edit_paste_lazy non-utf8 path"),
				"aid" => aid,
			})
		},
		|_| (),
	)
}

/// Get the path to workspace folder.
/// Returns an error if no folder is opened.
pub fn workspace_root() -> R<PathBuf> {
	crate::internal::executor::WORKSPACE_ROOT.lock().unwrap().clone().ok_or_else(|| E::error("this operation requires a folder to be open"))
}

/// Get the path to the root directory of the extension installation.
pub fn extension_root() -> PathBuf {
	crate::internal::executor::EXTENSION_ROOT.lock().unwrap().clone().unwrap()
}

/// Get the path to the currently edited file.
pub fn active_editor_file() -> LazyFuture<Option<PathBuf>> {
	LazyFuture::new_vscode(
		move |aid| {
			send_object(json::object! {
				"tag" => "active_editor_file",
				"aid" => aid,
			})
		},
		|raw| raw.as_str().map(PathBuf::from),
	)
}

/// Set the OS clipboard content to a given value.
pub fn clipboard_write(val: impl AsRef<str>) -> LazyFuture<()> {
	let val = val.as_ref().to_owned();
	LazyFuture::new_vscode(
		move |aid| {
			send_object(json::object! {
				"tag" => "clipboard_write",
				"aid" => aid,
				"val" => val,
			})
		},
		|_| (),
	)
}

/// Return an URI pointing to a given path for use with webviews.
pub fn asset(rel_path: impl AsRef<Path>) -> String {
	format!("vscode-resource://{}", extension_root().join("data/assets").join(rel_path.as_ref()).to_str().unwrap())
}

/// Set the status message.
/// This will interfere with other threads, use [`StackedStatus`](../goodies/stacked_status/index.html) instead.
pub fn status(msg: Option<&str>) {
	send_object(json::object! {
		"tag" => "status",
		"message" => msg,
	})
}
