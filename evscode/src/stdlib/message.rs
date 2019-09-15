//! Information messages with optional actions.
//!
//! The messages will ignore the newlines inside the string and display all text on one line.
//! At the bottom of the message, the extension name will be displayed.
//! [Markdown links](https://github.com/adam-p/markdown-here/wiki/Markdown-Cheatsheet#links) will be parsed and displayed as links.

use crate::{future::Pong, internal::executor::send_object};
use std::iter::{Chain, Empty, Once};

/// Action button that will appear on a message.
pub struct Action<'a> {
	/// Identifier that will be returned if the action in selected.
	pub id: String,
	/// Title of the button.
	pub title: &'a str,
	/// Whether the action will be selected as default if the message is closed.
	/// There can be only one item with this equal to `true`.
	pub is_close_affordance: bool,
}

/// Builder for configuring messages. Use [`Message::new`] to create.
#[must_use]
pub struct Builder<'a, A: Iterator<Item=Action<'a>>> {
	message: &'a str,
	kind: &'static str,
	modal: bool,
	items: A,
}
impl<'a, A: Iterator<Item=Action<'a>>> Builder<'a, A> {
	/// Use a orange warning icon
	pub fn warning(mut self) -> Self {
		self.kind = "warning";
		self
	}

	/// Use a red error icon
	pub fn error(mut self) -> Self {
		self.kind = "error";
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
	pub fn items<A2: IntoIterator<Item=Action<'a>>>(self, items: A2) -> Builder<'a, Chain<A, A2::IntoIter>> {
		Builder { message: self.message, kind: self.kind, modal: self.modal, items: self.items.chain(items.into_iter()) }
	}

	/// Add an action button to the message.
	/// See [`Action`] for the meaning of the arguments.
	pub fn item(self, id: String, title: &'a str, is_close_affordance: bool) -> Builder<'a, Chain<A, Once<Action<'a>>>> {
		Builder {
			message: self.message,
			kind: self.kind,
			modal: self.modal,
			items: self.items.chain(std::iter::once(Action { id, title, is_close_affordance })),
		}
	}

	/// Display the message.
	/// Returns the id of the selected action, if any.
	pub async fn show(self) -> Option<String> {
		self.start_show().await.as_str().map(str::to_owned)
	}

	/// Display the message without waiting for the user to close it.
	pub fn show_detach(self) {
		self.start_show();
	}

	fn start_show(self) -> Pong {
		let pong = Pong::new();
		send_object(json::object! {
			"tag" => "message",
			"message" => self.message,
			"kind" => self.kind,
			"items" => self.items.map(|item| json::object! {
				"id" => item.id,
				"title" => item.title,
				"isCloseAffordance" => item.is_close_affordance,
			}).collect::<Vec<_>>(),
			"modal" => self.modal,
			"aid" => pong.aid(),
		});
		pong
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
	pub fn new<'a>(message: &'a str) -> Builder<'a, Empty<Action>> {
		Builder { message, kind: "info", modal: false, items: std::iter::empty() }
	}
}
