//! Definitions of common types used throughout VS Code API.

use json::JsonValue;

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
