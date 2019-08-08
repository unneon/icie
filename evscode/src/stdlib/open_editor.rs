//! Builder pattern implementation for opening an editor.

use crate::{internal::executor::send_object, Column, Position, Range};
use std::path::Path;

/// Builder for opening text files in a VS Code editor.
pub struct Builder<'a> {
	path: &'a Path,
	cursor: Option<Position>,
	selection: Option<Range>,
	view_column: Option<Column>,
	preserve_focus: bool,
	preview: Option<bool>,
}

/// Open a text file in an editor. Uses the builder pattern.
pub fn open_editor(path: &Path) -> Builder {
	Builder { path, cursor: None, selection: None, view_column: None, preserve_focus: false, preview: None }
}

#[must_use]
impl Builder<'_> {
	/// Set cursor position in the text editor. The indices are 0-based.
	pub fn cursor(mut self, pos: impl Into<Option<Position>>) -> Self {
		self.cursor = pos.into();
		self
	}

	/// Set the selection in the text editor. The indices are 0-based.
	pub fn selection(mut self, range: Range) -> Self {
		self.selection = Some(range);
		self
	}

	/// Set the [`Column`] in which the editor will be opened.
	pub fn view_column(mut self, column: impl Into<Column>) -> Self {
		self.view_column = Some(column.into());
		self
	}

	/// Make the newly opened editor do not take focus.
	pub fn preserve_focus(mut self) -> Self {
		self.preserve_focus = true;
		self
	}

	/// No clue what this does or what is it's default value.
	/// The [official docs](https://code.visualstudio.com/api/references/vscode-api#TextDocumentShowOptions) aren't too clear about this.
	pub fn preview(mut self, value: bool) -> Self {
		self.preview = Some(value);
		self
	}

	/// Open the text file in the specified way.
	pub fn open(self) {
		send_object(json::object! {
			"tag" => "open_editor",
			"path" => self.path.to_str().expect("evscode::open_editor non-utf8 path"),
			"cursor" => self.cursor,
			"preserve_focus" => self.preserve_focus,
			"preview" => self.preview,
			"selection" => self.selection,
			"view_column" => self.view_column,
		});
	}
}
