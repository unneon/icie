//! Input boxes displayed at the top of the editor.

use crate::{future::Pong, internal::executor::send_object};

/// Builder for configurating input boxes. Use [`InputBox::new`] to create.
#[must_use]
pub struct Builder<'a> {
	ignore_focus_out: bool,
	password: bool,
	placeholder: Option<&'a str>,
	prompt: Option<&'a str>,
	value: Option<&'a str>,
	value_selection: Option<(usize, usize)>,
}
impl<'a> Builder<'a> {
	/// Do not make the input box disappear when user breaks focus.
	pub fn ignore_focus_out(mut self) -> Self {
		self.ignore_focus_out = true;
		self
	}

	/// Replace displayed characters with password placeholder characters.
	/// Consider also using [`Builder::ignore_focus_out`] because users might need to pull a password from their password manager.
	pub fn password(mut self) -> Self {
		self.password = true;
		self
	}

	/// Set a placeholder value that will be displayed with low opacity if the input box is empty.
	pub fn placeholder(mut self, x: &'a str) -> Self {
		self.placeholder = Some(x);
		self
	}

	/// Set a prompt text that tells the user what to do.
	/// VS Code will append a text that says to press Enter to continue or Escape to cancel.
	pub fn prompt(mut self, x: &'a str) -> Self {
		self.prompt = Some(x);
		self
	}

	/// Set default value in the input box.
	pub fn value(mut self, x: &'a str) -> Self {
		self.value = Some(x);
		self
	}

	/// Set which part of the default value will be selected by default.
	/// The indices are 0-based and closed-open(e.g. `.value("Hello, world!").value_selection(2, 6)` will select `llo,`.
	pub fn value_selection(mut self, l: usize, r: usize) -> Self {
		self.value_selection = Some((l, r));
		self
	}

	/// Display the input box.
	pub async fn show(self) -> Option<String> {
		let pong = Pong::new();
		send_object(json::object! {
			"tag" => "input_box",
			"prompt" => self.prompt,
			"placeHolder" => self.placeholder,
			"password" => self.password,
			"ignoreFocusOut" => self.ignore_focus_out,
			"value" => self.value,
			"valueSelection" => self.value_selection.map(|(l, r)| json::array! [l, r]),
			"aid" => pong.aid(),
		});
		pong.await.as_str().map(str::to_owned)
	}
}

/// Input box provided by the VS Code API.
///
/// See [module documentation](index.html) for details.
pub struct InputBox {
	_a: (),
}

impl InputBox {
	/// Create a new builder to configure the input box.
	pub fn new() -> Builder<'static> {
		Builder { ignore_focus_out: false, password: false, placeholder: None, prompt: None, value: None, value_selection: None }
	}
}
