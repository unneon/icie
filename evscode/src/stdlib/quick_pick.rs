//! Selecting one or some of the given options.

use crate::{future::Pong, internal::executor::send_object};
use std::iter::{Chain, Empty, Once};

/// Builder object for an item that can be selected.
#[must_use]
pub struct Item {
	always_show: bool,
	description: Option<String>,
	detail: Option<String>,
	label: String,
	id: String,
}
impl Item {
	/// Create a new item with the given ID and label.
	pub fn new(id: String, label: String) -> Item {
		Item { always_show: false, description: None, detail: None, label, id }
	}

	/// Set to show item regardless of whether what user typed matches the item.
	pub fn always_show(mut self) -> Self {
		self.always_show = true;
		self
	}

	/// Set description, displayed in lighter font beside the label.
	pub fn description(mut self, x: String) -> Self {
		self.description = Some(x);
		self
	}

	/// Set detail, displayed in smaller and lighter font below the label.
	pub fn detail(mut self, x: String) -> Self {
		self.detail = Some(x);
		self
	}
}

/// Builder for configuring quick picks. Use [`QuickPick::new`] to create.
#[must_use]
pub struct Builder<'a, I: Iterator<Item=Item>> {
	ignore_focus_out: bool,
	match_on_description: bool,
	match_on_detail: bool,
	placeholder: Option<&'a str>,
	items: I,
}
impl<'a, I: Iterator<Item=Item>> Builder<'a, I> {
	/// Do not make the quick pick disappear when user breaks focus.
	pub fn ignore_focus_out(mut self) -> Self {
		self.ignore_focus_out = true;
		self
	}

	/// When user types a filter, match it against the description as well as the label.
	pub fn match_on_description(mut self) -> Self {
		self.match_on_description = true;
		self
	}

	/// When user types a filter, match it against the detail as well as the label.
	pub fn match_on_detail(mut self) -> Self {
		self.match_on_detail = true;
		self
	}

	/// Set a placeholder.
	pub fn placeholder(mut self, x: &'a str) -> Self {
		self.placeholder = Some(x);
		self
	}

	/// When user types a filter, match it against the description and the detail as well as the label.
	pub fn match_on_all(mut self) -> Self {
		self.match_on_description = true;
		self.match_on_detail = true;
		self
	}

	/// Add an item to the selection.
	pub fn item(self, item: Item) -> Builder<'a, Chain<I, Once<Item>>> {
		Builder {
			ignore_focus_out: self.ignore_focus_out,
			match_on_description: self.match_on_description,
			match_on_detail: self.match_on_detail,
			placeholder: self.placeholder,
			items: self.items.chain(std::iter::once(item)),
		}
	}

	/// Add items to the selection.
	pub fn items<I2: IntoIterator<Item=Item>>(self, items: I2) -> Builder<'a, Chain<I, I2::IntoIter>> {
		Builder {
			ignore_focus_out: self.ignore_focus_out,
			match_on_description: self.match_on_description,
			match_on_detail: self.match_on_detail,
			placeholder: self.placeholder,
			items: self.items.chain(items.into_iter()),
		}
	}

	/// Prepare a lazy future with the quick pick.
	/// This does not spawn it yet.
	pub async fn show(self) -> Option<String> {
		let pong = Pong::new();
		send_object(json::object! {
			"tag" => "quick_pick",
			"ignoreFocusOut" => self.ignore_focus_out,
			"matchOnDescription" => self.match_on_description,
			"matchOnDetail" => self.match_on_detail,
			"placeholder" => self.placeholder,
			"items" => self.items.map(|item| json::object! {
				"label" => item.label,
				"description" => item.description,
				"detail" => item.detail,
				"alwaysShow" => item.always_show,
				"id" => item.id,
			}).collect::<Vec<_>>(),
			"aid" => pong.aid(),
		});
		pong.await.as_str().map(str::to_owned)
	}
}

/// Quick pick provided by the VS Code API.
///
/// See [module documentation](index.html) for details.
pub struct QuickPick {
	_a: (),
}

impl QuickPick {
	/// Create a new builder to configure the quick pick.
	pub fn new() -> Builder<'static, Empty<Item>> {
		Builder { ignore_focus_out: false, match_on_detail: false, match_on_description: false, placeholder: None, items: std::iter::empty() }
	}
}
