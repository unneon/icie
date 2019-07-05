//! Selecting one or some of the given options.

use crate::{internal::executor::send_object, LazyFuture};

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
	pub fn new(id: impl AsRef<str>, label: impl AsRef<str>) -> Item {
		Item {
			always_show: false,
			description: None,
			detail: None,
			label: label.as_ref().to_owned(),
			id: id.as_ref().to_owned(),
		}
	}

	/// Set to show item regardless of whether what user typed matches the item.
	pub fn always_show(mut self) -> Self {
		self.always_show = true;
		self
	}

	/// Set description, displayed in lighter font beside the label.
	pub fn description(mut self, x: impl AsRef<str>) -> Self {
		self.description = Some(x.as_ref().to_owned());
		self
	}

	/// Set detail, displayed in smaller and lighter font below the label.
	pub fn detail(mut self, x: impl AsRef<str>) -> Self {
		self.detail = Some(x.as_ref().to_owned());
		self
	}
}

/// Builder for configuring quick picks. Use [`QuickPick::new`] to create.
#[must_use]
pub struct Builder {
	ignore_focus_out: bool,
	match_on_description: bool,
	match_on_detail: bool,
	placeholder: Option<String>,
	items: Vec<Item>,
}
impl Builder {
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
	pub fn placeholder(mut self, x: impl AsRef<str>) -> Self {
		self.placeholder = Some(x.as_ref().to_owned());
		self
	}

	/// When user types a filter, match it against the description and the detail as well as the label.
	pub fn match_on_all(mut self) -> Self {
		self.match_on_description = true;
		self.match_on_detail = true;
		self
	}

	/// Add an item to the selection.
	pub fn item(mut self, item: Item) -> Self {
		self.items.push(item);
		self
	}

	/// Add items to the selection.
	pub fn items(mut self, items: impl IntoIterator<Item=Item>) -> Self {
		for item in items.into_iter() {
			self = self.item(item);
		}
		self
	}

	/// Prepare a lazy future with the quick pick.
	/// This does not spawn it yet.
	pub fn build(self) -> LazyFuture<Option<String>> {
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "quick_pick",
					"ignoreFocusOut" => self.ignore_focus_out,
					"matchOnDescription" => self.match_on_description,
					"matchOnDetail" => self.match_on_detail,
					"placeholder" => self.placeholder,
					"items" => self.items.into_iter().map(|item| json::object! {
						"label" => item.label,
						"description" => item.description,
						"detail" => item.detail,
						"alwaysShow" => item.always_show,
						"id" => item.id,
					}).collect::<Vec<_>>(),
					"aid" => aid,
				})
			},
			|raw| raw.as_str().map(String::from),
		)
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
	pub fn new() -> Builder {
		Builder {
			ignore_focus_out: false,
			match_on_detail: false,
			match_on_description: false,
			placeholder: None,
			items: Vec::new(),
		}
	}
}
