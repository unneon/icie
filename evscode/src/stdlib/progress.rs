//! Progress bars, both finite and infinite.

use crate::{
	internal::executor::{send_object, HANDLE_FACTORY}, LazyFuture
};
use std::sync::atomic::{AtomicBool, Ordering};

/// Builder for configuring progress bars. Use [`Progress::new`] to create.
#[must_use]
pub struct Builder {
	title: Option<String>,
	location: &'static str,
	cancellable: bool,
}
impl Builder {
	/// Set the progress bar title.
	pub fn title(mut self, x: impl AsRef<str>) -> Self {
		self.title = Some(x.as_ref().to_owned());
		self
	}

	/// Change the progress bar location to the source control tab.
	pub fn in_source_control(mut self) -> Self {
		self.location = "source_control";
		self
	}

	/// Change the progress bar location to the entire window(instead of a message).
	pub fn in_window(mut self) -> Self {
		self.location = "window";
		self
	}

	/// Enable a cancel button that stops the progress.
	pub fn cancellable(mut self) -> Self {
		self.cancellable = true;
		self
	}

	/// Display the progress bar.
	pub fn show(self) -> Progress {
		let hid = HANDLE_FACTORY.generate();
		send_object(json::object! {
			"tag" => "progress_start",
			"hid" => hid.to_string(),
			"title" => self.title,
			"location" => self.location,
			"cancellable" => self.cancellable,
		});
		Progress { hid, canceler_spawned: AtomicBool::new(false) }
	}
}
/// Progress bar provided by the VS Code API.
pub struct Progress {
	hid: u64,
	canceler_spawned: AtomicBool,
}
impl Progress {
	/// Create a new builder to configure the progress bar.
	pub fn new() -> Builder {
		Builder { title: None, location: "notification", cancellable: false }
	}

	/// Update both components of the progress bar, see [`Progress::increment`] and [`Progress::message`].
	pub fn update(&self, inc: f64, msg: impl AsRef<str>) {
		self.partial_update(Some(inc), Some(msg.as_ref()));
	}

	/// Increment the progress bar by the given percentage.
	pub fn increment(&self, inc: f64) {
		self.partial_update(Some(inc), None);
	}

	/// Change the progress bar message to a specified value.
	/// This message will be displayed beside the title.
	pub fn message(&self, msg: impl AsRef<str>) {
		self.partial_update(None, Some(msg.as_ref()));
	}

	/// Update each components of the progress bar if given, see [`Progress::increment`] and [`Progress::message`].
	pub fn partial_update(&self, inc: Option<f64>, msg: Option<&str>) {
		send_object(json::object! {
			"tag" => "progress_update",
			"hid" => self.hid.to_string(),
			"increment" => inc,
			"message" => msg,
		});
	}

	/// Returns a lazy future that will yield () if user presses the cancel button.
	/// For this to ever happen, [`Builder::cancellable`] must be called when building the progress bar.
	/// This function can only be called once.
	pub fn canceler(&self) -> LazyFuture<()> {
		assert!(!self.canceler_spawned.fetch_or(true, Ordering::SeqCst));
		let hid = self.hid;
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "progress_register_cancel",
					"hid" => hid.to_string(),
					"aid" => aid,
				})
			},
			|_| (),
		)
	}

	/// Close the progress bar.
	pub fn end(self) {
	}
}

/// Dropping the object closes the progress bar
impl Drop for Progress {
	fn drop(&mut self) {
		send_object(json::object! {
			"tag" => "progress_end",
			"hid" => self.hid.to_string(),
		});
	}
}
