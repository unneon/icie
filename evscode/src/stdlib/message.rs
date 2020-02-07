//! Information messages with optional actions.
//!
//! The messages will ignore the newlines inside the string and display all text on one line.
//! At the bottom of the message, the extension name will be displayed.
//! [Markdown links](https://github.com/adam-p/markdown-here/wiki/Markdown-Cheatsheet#links) will be parsed and displayed as links.

use serde::{Deserialize, Serialize};
use std::{
	future::Future, iter::{Chain, Empty, Once}, marker::PhantomData, pin::Pin, task::{Context, Poll}
};
use wasm_bindgen::JsValue;

/// Action button that will appear on a message.
pub struct Action<'a, T> {
	/// Identifier that will be returned if the action in selected.
	pub id: T,
	/// Title of the button.
	pub title: &'a str,
	/// Whether the action will be selected as default if the message is closed.
	/// There can be only one item with this equal to `true`.
	/// This option only works for modal messages and is otherwise ignored.
	pub is_close_affordance: bool,
}

/// Builder for configuring messages. Use [`Message::new`] to create.
#[must_use]
pub struct Builder<'a, T, A: Iterator<Item=Action<'a, T>>> {
	message: &'a str,
	kind: fn(&str, &JsValue, Vec<JsValue>) -> vscode_sys::Thenable<JsValue>,
	modal: bool,
	items: A,
}
impl<'a, T: Unpin+Serialize+for<'d> Deserialize<'d>, A: Iterator<Item=Action<'a, T>>>
	Builder<'a, T, A>
{
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
	/// Instead of displaying the message in the bottom right corner, VS Code will display it as a
	/// popup and block the rest of the editor until user responds. Probably only use this for
	/// messages which require urgent user interaction.
	pub fn modal(mut self) -> Self {
		self.modal = true;
		self
	}

	/// Add action buttons to the message.
	pub fn items<A2: IntoIterator<Item=Action<'a, T>>>(
		self,
		items: A2,
	) -> Builder<'a, T, Chain<A, A2::IntoIter>>
	{
		Builder {
			message: self.message,
			kind: self.kind,
			modal: self.modal,
			items: self.items.chain(items.into_iter()),
		}
	}

	/// Add an action button to the message.
	/// See [`Action`] for the meaning of the arguments.
	pub fn item(
		self,
		id: T,
		title: &'a str,
		is_close_affordance: bool,
	) -> Builder<'a, T, Chain<A, Once<Action<'a, T>>>>
	{
		Builder {
			message: self.message,
			kind: self.kind,
			modal: self.modal,
			items: self.items.chain(std::iter::once(Action { id, title, is_close_affordance })),
		}
	}

	/// Display the message, with a lazy future.
	/// Returns the id of the selected action, if any.
	pub async fn show(self) -> Option<T> {
		self.show_eager().await
	}

	/// Display the message, and only then return a future with the result.
	/// Returns the id of the selected action, if any.
	pub fn show_eager(self) -> ShownMessage<T> {
		let options =
			JsValue::from_serde(&vscode_sys::window::ShowMessageOptions { modal: self.modal })
				.unwrap();
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
		ShownMessage(promise, PhantomData)
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
	pub fn new<'a, T>(message: &'a str) -> Builder<'a, T, Empty<Action<'a, T>>> {
		Builder {
			message,
			kind: vscode_sys::window::show_information_message,
			modal: false,
			items: std::iter::empty(),
		}
	}
}

/// A future returned after displaying a message eagerly. See [`Message::show`] and
/// [`Message::show_eager`].
pub struct ShownMessage<T>(vscode_sys::Thenable<JsValue>, PhantomData<T>);

impl<T: for<'d> Deserialize<'d>+Unpin> Future for ShownMessage<T> {
	type Output = Option<T>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
		Pin::new(&mut self.get_mut().0).poll(cx).map(|ret| {
			let resp: Result<vscode_sys::ItemRet<T>, _> = ret.into_serde();
			match resp {
				Ok(resp) => Some(resp.id),
				Err(_) => None,
			}
		})
	}
}
