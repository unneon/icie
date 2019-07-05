//! Information messages with optional actions.
//!
//! The messages will ignore the newlines inside the string and display all text on one line.
//! At the bottom of the message, the extension name will be displayed.
//! [Markdown links](https://github.com/adam-p/markdown-here/wiki/Markdown-Cheatsheet#links) will be parsed and displayed as links.

use crate::{internal::executor::send_object, LazyFuture};

struct Action {
	id: String,
	title: String,
	is_close_affordance: bool,
}

/// Builder for configuring messages. Use [`Message::new`] to create.
#[must_use]
pub struct Builder {
	message: String,
	kind: &'static str,
	modal: bool,
	items: Vec<Action>,
}
impl Builder {
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
	/// Instead of displaying the message in the bottom right corner, VS Code will display it as a popup and block the rest of the editor until user responds.
	/// Probably only use this for messages which require urgent user interaction.
	pub fn modal(mut self) -> Self {
		self.modal = true;
		self
	}

	/// Add an action button to the message.
	/// The id will be returned if the item is selected.
	/// There can be at most one close affordance item, which will be selected as default if the message is closed.
	pub fn item(mut self, id: impl AsRef<str>, title: impl AsRef<str>, is_close_affordance: bool) -> Self {
		self.items.push(Action {
			id: id.as_ref().to_owned(),
			title: title.as_ref().to_owned(),
			is_close_affordance,
		});
		self
	}

	/// Prepare a lazy future with the message.
	/// This does not display it yet.
	pub fn build(self) -> LazyFuture<Option<String>> {
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "message",
					"message" => self.message.as_str(),
					"kind" => self.kind,
					"items" => self.items.iter().map(|item| json::object! {
						"id" => item.id.as_str(),
						"title" => item.title.as_str(),
						"isCloseAffordance" => item.is_close_affordance,
					}).collect::<Vec<_>>(),
					"modal" => self.modal,
					"aid" => aid,
				})
			},
			|raw| raw.as_str().map(String::from),
		)
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
	pub fn new(message: impl AsRef<str>) -> Builder {
		Builder {
			message: message.as_ref().to_owned(),
			kind: "info",
			modal: false,
			items: Vec::new(),
		}
	}
}
