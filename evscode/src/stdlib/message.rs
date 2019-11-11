//! Information messages with optional actions.
//!
//! The messages will ignore the newlines inside the string and display all text on one line.
//! At the bottom of the message, the extension name will be displayed.
//! [Markdown links](https://github.com/adam-p/markdown-here/wiki/Markdown-Cheatsheet#links) will be parsed and displayed as links.

use serde::{Deserialize, Serialize};
use std::{
	future::Future, iter::{Chain, Empty, Once}
};
use wasm_bindgen::JsValue;

/// Action button that will appear on a message.
pub struct Action<T> {
	/// Identifier that will be returned if the action in selected.
	pub id: T,
	/// Title of the button.
	pub title: String,
	/// Whether the action will be selected as default if the message is closed.
	/// There can be only one item with this equal to `true`.
	/// This option only works for modal messages and is otherwise ignored.
	pub is_close_affordance: bool,
}

/// Builder for configuring messages. Use [`Message::new`] to create.
#[must_use]
pub struct Builder<T, A: Iterator<Item=Action<T>>> {
	message: String,
	kind: fn(&str, &JsValue, Vec<JsValue>) -> vscode_sys::Thenable<JsValue>,
	modal: bool,
	items: A,
}
impl<T: Serialize+for<'d> Deserialize<'d>, A: Iterator<Item=Action<T>>> Builder<T, A> {
	/// Use a orange warning icon
	pub fn warning(mut self) -> Self {
		self.kind = vscode_sys::window::show_warning_message;
		self
	}

	/// Use a red error icon
	pub fn error(mut self) -> Self {
		self.kind = vscode_sys::window::show_error_message;
		self
	}

	/// Make message modal.
	/// Instead of displaying the message in the bottom right corner, VS Code will display it as a popup and block the rest of the editor until user
	/// responds. Probably only use this for messages which require urgent user interaction.
	pub fn modal(mut self) -> Self {
		self.modal = true;
		self
	}

	/// Add action buttons to the message.
	pub fn items<A2: IntoIterator<Item=Action<T>>>(self, items: A2) -> Builder<T, Chain<A, A2::IntoIter>> {
		Builder { message: self.message, kind: self.kind, modal: self.modal, items: self.items.chain(items.into_iter()) }
	}

	/// Add an action button to the message.
	/// See [`Action`] for the meaning of the arguments.
	pub fn item(self, id: T, title: &str, is_close_affordance: bool) -> Builder<T, Chain<A, Once<Action<T>>>> {
		Builder {
			message: self.message,
			kind: self.kind,
			modal: self.modal,
			items: self.items.chain(std::iter::once(Action { id, title: title.to_owned(), is_close_affordance })),
		}
	}

	/// Display the message, with a lazy future.
	/// Returns the id of the selected action, if any.
	pub async fn show(self) -> Option<T> {
		self.show_eager().await
	}

	/// Display the message, and only then return a future with the result.
	/// Returns the id of the selected action, if any.
	pub fn show_eager(self) -> impl Future<Output=Option<T>>+'static {
		let options = JsValue::from_serde(&vscode_sys::window::ShowMessageOptions { modal: self.modal }).unwrap();
		let items: Vec<_> = self
			.items
			.map(|item| {
				JsValue::from_serde(&vscode_sys::window::ShowMessageItem {
					is_close_affordance: item.is_close_affordance,
					title: &item.title,
					id: item.id,
				})
				.unwrap()
			})
			.collect();
		let promise = (self.kind)(&self.message, &options, items);
		async move {
			let resp: Result<vscode_sys::ItemRet<T>, _> = promise.await.into_serde();
			match resp {
				Ok(resp) => Some(resp.id),
				Err(_) => None,
			}
		}
	}
}

/// Info message provided by the VS Code API
///
/// See [module documentation](index.html) for details.
pub struct Message {
	_a: (),
}

impl Message {
	/// Create a new builder to configure the message.
	pub fn new<T>(message: &str) -> Builder<T, Empty<Action<T>>> {
		Builder { message: message.to_owned(), kind: vscode_sys::window::show_information_message, modal: false, items: std::iter::empty() }
	}
}
