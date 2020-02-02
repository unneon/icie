//! Status bar element that supports adding items to a single status object.
//!
//! First, create the element in your main.rs file.
//! The passed string is the prefix added before specific status messages.
//! ```
//! lazy_static::lazy_static! {
//!     static ref STATUS: evscode::MultiStatus = evscode::MultiStatus::new("EEE ");
//! }
//! ```
//! Then, to set the status use the [`StackedStatus::push`] function and save the returned guard for
//! the duration of the operation. ```
//! # let STATUS = evscode::StackedStatus::new("EEE ");
//! # fn compile() {}
//! # fn parse_compilation_errors() {}
//! let _status = STATUS.push("Building"); // "EEE Building"
//! compile();
//! {
//!     let _status = STATUS.push("Parsing compilation errors"); // "EEE Building, Parsing
//! compilation errors"     parse_compilation_errors();
//! }
//! // "EEE Building"
//! // (disappears)
//! ```
//! If multiple [`StackedStatus::push`] operations are active simultaneously, the messages will be
//! separated with a comma.

use std::sync::{Mutex, MutexGuard};

/// A structure that holds stacked status state. See [module documentation](index.html) for details.
pub struct MultiStatus {
	prefix: &'static str,
	stacks: Mutex<Vec<String>>,
}
impl MultiStatus {
	/// Create a state instance with a given message prefix
	pub fn new(prefix: &'static str) -> MultiStatus {
		MultiStatus { prefix, stacks: Mutex::new(Vec::new()) }
	}

	/// Set the current thread status message and return a guard object that will control its
	/// lifetime
	pub fn push(&self, msg: impl AsRef<str>) -> Guard {
		let msg = msg.as_ref().to_owned();
		let mut lck = self.obtain_lock();
		lck.push(msg.clone());
		self.update(lck);
		Guard { stacked: self, msg }
	}

	fn update(&self, mut words: MutexGuard<Vec<String>>) {
		words.sort();
		let msg = if !words.is_empty() {
			Some(format!("{} {}", self.prefix, words.join(", ")))
		} else {
			None
		};
		crate::stdlib::status(msg.as_deref());
	}

	fn obtain_lock(&self) -> MutexGuard<Vec<String>> {
		self.stacks.try_lock().expect("evscode.MultiStatus.push_stacks failed to lock mutex")
	}
}

/// Guard object that will remove its associated status message when dropped
pub struct Guard<'a> {
	stacked: &'a MultiStatus,
	msg: String,
}
impl<'a> Drop for Guard<'a> {
	fn drop(&mut self) {
		let mut lck = self.stacked.obtain_lock();
		lck.remove_item(&self.msg);
		self.stacked.update(lck);
	}
}
