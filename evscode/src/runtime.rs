//! Runtime used by Evscode to manage communicating with VS Code

use crate::R;

/// Spawn a thread. If the function fails, the error returned from the function will be displayed to the user.
pub fn spawn(f: impl FnOnce() -> R<()>+Send+'static) {
	crate::internal::executor::spawn(f)
}
