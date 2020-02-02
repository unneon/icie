//! Progress bars, both finite and infinite.

use futures::{channel::mpsc, FutureExt, StreamExt};
use js_sys::Promise;
use std::{future::Future, sync::Mutex};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

/// Builder for configuring progress bars. Use [`Progress::new`] to create.
#[must_use]
pub struct Builder {
	title: Option<String>,
	location: vscode_sys::window::ProgressLocation,
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
		self.location = vscode_sys::window::ProgressLocation::SourceControl;
		self
	}

	/// Change the progress bar location to the entire window(instead of a message).
	pub fn in_window(mut self) -> Self {
		self.location = vscode_sys::window::ProgressLocation::Window;
		self
	}

	/// Enable a cancel button that stops the progress.
	pub fn cancellable(mut self) -> Self {
		self.cancellable = true;
		self
	}

	/// Display the progress bar.
	pub fn show(self) -> (Progress, impl Future<Output=()>) {
		let (tx, mut rx) = mpsc::unbounded::<(Option<f64>, Option<String>)>();
		let (cancel_tx, cancel_rx) = futures::channel::oneshot::channel();
		let cancel_callback = move |_: JsValue| {
			let _ = cancel_tx.send(());
		};
		let progress_loop = move |progress: vscode_sys::ProgressProgress,
		                          cancel_token: vscode_sys::CancellationToken|
		      -> Promise {
			js_sys::Reflect::apply(
				&cancel_token.on_cancellation_requested().unchecked_into(),
				&JsValue::undefined(),
				&js_sys::Array::of1(&Closure::once_into_js(cancel_callback)),
			)
			.unwrap();
			wasm_bindgen_futures::future_to_promise(async move {
				while let Some(update) = rx.next().await {
					progress.report(vscode_sys::ProgressProgressValue {
						increment: update.0,
						message: update.1.as_deref(),
					})
				}
				Ok(JsValue::undefined())
			})
		};
		vscode_sys::window::with_progress(
			vscode_sys::window::ProgressOptions {
				cancellable: self.cancellable,
				location: self.location,
				title: self.title.as_deref(),
			},
			Closure::once_into_js(progress_loop),
		);
		(Progress { tx, value: Mutex::new(0.0) }, cancel_rx.map(Result::unwrap))
	}
}

/// Progress bar provided by the VS Code API.
pub struct Progress {
	tx: mpsc::UnboundedSender<(Option<f64>, Option<String>)>,
	value: Mutex<f64>,
}
impl Progress {
	/// Create a new builder to configure the progress bar.
	pub fn new() -> Builder {
		Builder {
			title: None,
			location: vscode_sys::window::ProgressLocation::Notification,
			cancellable: false,
		}
	}

	/// Increment and set message on the progress bar, see [`Progress::increment`] and
	/// [`Progress::message`].
	pub fn update_inc(&self, inc: f64, msg: impl AsRef<str>) {
		self.partial_update(Some(inc), Some(msg.as_ref()));
	}

	/// Set value and message on the progress bar, see [`Progress::increment`] and
	/// [`Progress::message`].
	pub fn update_set(&self, val: f64, msg: impl AsRef<str>) {
		let old_val = *self.value.lock().unwrap();
		self.partial_update(Some(val - old_val), Some(msg.as_ref()));
	}

	/// Increment the progress bar by the given percentage.
	pub fn increment(&self, inc: f64) {
		self.partial_update(Some(inc), None);
	}

	/// Set the progress bar to the given percentage.
	pub fn set(&self, val: f64) {
		let old_val = *self.value.lock().unwrap();
		self.increment(val - old_val)
	}

	/// Change the progress bar message to a specified value.
	/// This message will be displayed beside the title.
	pub fn message(&self, msg: impl AsRef<str>) {
		self.partial_update(None, Some(msg.as_ref()));
	}

	/// Update each components of the progress bar if given, see [`Progress::increment`] and
	/// [`Progress::message`]. This will panic if the progress exceeds 110%.
	pub fn partial_update(&self, inc: Option<f64>, msg: Option<&str>) {
		if let Some(inc) = inc {
			*self.value.lock().unwrap() += inc;
			assert!(*self.value.lock().unwrap() <= 110.0);
		}
		let _ = self.tx.unbounded_send((inc, msg.map(str::to_owned)));
	}

	/// Close the progress bar.
	pub fn end(self) {
	}
}
