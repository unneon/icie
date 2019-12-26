//! Builder pattern implementation for opening an editor.

use crate::{Column, Position, Range, R};
use wasm_bindgen::JsCast;

/// Builder for opening text files in a VS Code editor.
#[must_use]
pub struct Builder<'a> {
	path: &'a str,
	cursor: Option<Position>,
	selection: Option<Range>,
	view_column: Option<Column>,
	preserve_focus: bool,
	preview: Option<bool>,
	force_new: bool,
}

/// Open a text file in an editor, or focuses an existing one if it exists. Uses the builder
/// pattern.
pub fn open_editor(path: &str) -> Builder {
	Builder {
		path,
		cursor: None,
		selection: None,
		view_column: None,
		preserve_focus: false,
		preview: None,
		force_new: false,
	}
}

impl<'a> Builder<'a> {
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

	/// Even if an existing editor could be used, open a new one regardless.
	pub fn force_new(mut self) -> Self {
		self.force_new = true;
		self
	}

	/// Open the text file in the specified way.
	pub async fn open(self) -> R<()> {
		let editor = if !self.force_new {
			vscode_sys::window::VISIBLE_TEXT_EDITORS
				.values()
				.into_iter()
				.map(|edi| edi.unwrap().unchecked_into::<vscode_sys::TextEditor>())
				.find(|edi| edi.document().file_name() == self.path)
		} else {
			None
		};
		let editor = match editor {
			Some(editor) => (*editor).clone().unchecked_into(),
			None => {
				let doc = vscode_sys::workspace::open_text_document(self.path).await?;
				vscode_sys::window::show_text_document(&doc).await
			},
		};
		if let Some(cursor) = self.cursor {
			let pos = vscode_sys::Position::new(cursor.line, cursor.column);
			editor.set_selection(vscode_sys::Selection::new(&pos, &pos));
			editor.reveal_range(
				&vscode_sys::Range::new(&pos, &pos),
				vscode_sys::TextEditorRevealType::InCenter,
			);
		}
		Ok(())
	}
}
