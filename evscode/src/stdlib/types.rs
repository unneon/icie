//! Definitions of common types used throughout VS Code API.

/// Represents a line and character position.
pub struct Position {
	/// The number of characters from the left.
	pub column: usize,
	/// The line number.
	pub line: usize,
}

/// Represents an ordered pair of two positions.
pub struct Range {
	/// The beginning of the range.
	pub start: Position,
	/// The ending of the range, this position on the boundary.
	pub end: Position,
}

/// View column where a tab can appear.
///
/// The values are from the [docs](1), and hopefully no one changes them.
///
/// [1]: https://code.visualstudio.com/api/references/vscode-api#ViewColumn
#[derive(Clone, Copy)]
#[repr(i32)]
pub enum Column {
	/// View column of the currently active tab.
	Active = -1,
	/// View column to the right of the currently active tab.
	/// This can create new columns depending on what is currently selected.
	/// Examples:
	/// - One column exists: the column is split in half, the right half is taken by the new
	///   webview.
	/// - Two columns exist, left active: the new webvieb is added to the right column as a new
	///   tab.
	/// - Two columns exist, right active: the right column is split in half, the right half of the
	///   right half is taken by the new webview.
	Beside = -2,
	/// First, leftmost column.
	One = 1,
	/// Second column.
	Two = 2,
	/// Third column.
	Three = 3,
	/// Fourth column.
	Four = 4,
	/// Fifth column.
	Five = 5,
	/// Sixth column.
	Six = 6,
	/// Seventh column.
	Seven = 7,
	/// Eighth column.
	Eight = 8,
	/// Ninth column.
	Nine = 9,
}
impl Column {
	pub(crate) fn as_enum_id(self) -> i32 {
		self as i32
	}
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
