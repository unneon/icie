//! Selecting one or some of the given options.

use js_sys::Array;
use serde::{Deserialize, Serialize};
use std::iter::{Chain, Empty, Once};
use wasm_bindgen::JsValue;

/// Builder object for an item that can be selected.
#[must_use]
pub struct Item<T> {
	always_show: bool,
	description: Option<String>,
	detail: Option<String>,
	label: String,
	id: T,
}
impl<T> Item<T> {
	/// Create a new item with the given ID and label.
	pub fn new(id: T, label: String) -> Item<T> {
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
pub struct Builder<'a, T, I: Iterator<Item=Item<T>>> {
	ignore_focus_out: bool,
	match_on_description: bool,
	match_on_detail: bool,
	placeholder: Option<&'a str>,
	items: I,
}
impl<'a, T: Serialize+for<'d> Deserialize<'d>, I: Iterator<Item=Item<T>>> Builder<'a, T, I> {
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

	/// When user types a filter, match it against the description and the detail as well as the
	/// label.
	pub fn match_on_all(mut self) -> Self {
		self.match_on_description = true;
		self.match_on_detail = true;
		self
	}

	/// Add an item to the selection.
	pub fn item(self, item: Item<T>) -> Builder<'a, T, Chain<I, Once<Item<T>>>> {
		Builder {
			ignore_focus_out: self.ignore_focus_out,
			match_on_description: self.match_on_description,
			match_on_detail: self.match_on_detail,
			placeholder: self.placeholder,
			items: self.items.chain(std::iter::once(item)),
		}
	}

	/// Add items to the selection.
	pub fn items<I2: IntoIterator<Item=Item<T>>>(
		self,
		items: I2,
	) -> Builder<'a, T, Chain<I, I2::IntoIter>>
	{
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
	pub async fn show(self) -> Option<T> {
		let items = Array::new();
		for item in self.items {
			items.push(
				&JsValue::from_serde(&vscode_sys::window::ShowQuickPickItem {
					detail: item.detail.as_deref(),
					description: item.description.as_deref(),
					always_show: item.always_show,
					label: &item.label,
					id: item.id,
					picked: false,
				})
				.unwrap(),
			);
		}
		let options = vscode_sys::window::ShowQuickPickOptions {
			can_pick_many: false,
			ignore_focus_out: self.ignore_focus_out,
			match_on_description: self.match_on_description,
			match_on_detail: self.match_on_detail,
			place_holder: self.placeholder,
		};
		let item = vscode_sys::window::show_quick_pick(&items, options).await;
		if !item.is_undefined() {
			let item: vscode_sys::ItemRet<T> = item.into_serde().unwrap();
			Some(item.id)
		} else {
			None
		}
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
	pub fn new<T>() -> Builder<'static, T, Empty<Item<T>>> {
		Builder {
			ignore_focus_out: false,
			match_on_detail: false,
			match_on_description: false,
			placeholder: None,
			items: std::iter::empty(),
		}
	}
}
