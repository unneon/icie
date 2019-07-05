//! Container for a singleton webview used by many threads.

use crate::{goodies::WebviewHandle, Webview, R};
use std::sync::{Arc, Mutex};

/// Function that creates the webview.
pub type Creator = fn() -> R<Webview>;
/// Worker function that handles the messages received from the webview.
pub type Manager = fn(WebviewHandle) -> R<()>;

/// State of the webview.
pub struct WebviewSingleton {
	create: Creator,
	manage: Manager,
	container: Mutex<Option<Arc<Mutex<Webview>>>>,
}

impl WebviewSingleton {
	/// Create a new instance of the webview
	pub fn new(create: Creator, manage: Manager) -> WebviewSingleton {
		WebviewSingleton {
			create,
			manage,
			container: Mutex::new(None),
		}
	}

	/// Get a webview handle, creating it if it does not exist or was closed.
	pub fn handle(&'static self) -> R<Arc<Mutex<Webview>>> {
		let mut container_lock = self.container.lock().unwrap();
		let view = if let Some(view) = &*container_lock {
			view.clone()
		} else {
			let view = (self.create)()?;
			let handle = Arc::new(Mutex::new(view));
			let handle2 = handle.clone();
			crate::runtime::spawn(move || {
				(self.manage)(handle2)?;
				let mut container_lock = self.container.lock().unwrap();
				*container_lock = None;
				Ok(())
			});
			*container_lock = Some(handle.clone());
			handle
		};
		Ok(view)
	}
}
