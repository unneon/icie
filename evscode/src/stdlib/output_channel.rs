//! Readonly text containers, displayed at the bottom of the screen.
//!
//! Typically used for displaying compilation output, logs from spawned processes, and other
//! user-facing information that take too much space to display elsewhere and are a part of normal
//! workflow.

/// Handle to an output channel.
pub struct OutputChannel {
	native: vscode_sys::OutputChannel,
}

impl OutputChannel {
	/// Create a new output channel with a given name.
	pub fn new(name: &str) -> OutputChannel {
		OutputChannel { native: vscode_sys::window::create_output_channel(name) }
	}

	/// Appends text to the output. Newline is not added automatically.
	pub fn append(&self, text: &str) {
		self.native.append(text)
	}

	/// Removes all text from the output channel.
	pub fn clear(&self) {
		self.native.clear()
	}

	/// Reveals the output channel in the UI.
	pub fn show(&self, preserve_focus: bool) {
		self.native.show(preserve_focus)
	}
}
