//! Status bar element that supports nested status setting across multiple threads.
//!
//! First, create the element in your main.rs file.
//! The passed string is the prefix added before specific status messages.
//! ```
//! lazy_static::lazy_static! {
//! 		static ref STATUS: evscode::StackedStatus = evscode::StackedStatus::new("EEE ");
//! }
//! ```
//! Then, to set the status use the [`StackedStatus::push`] function and save the returned guard for the duration of the operation.
//! ```
//! # let STATUS = evscode::StackedStatus::new("EEE ");
//! # fn compile() {}
//! # fn parse_compilation_errors() {}
//! let _status = STATUS.push("Building"); // "EEE Building"
//! compile();
//! {
//! 	let _status = STATUS.push("Parsing compilation errors"); // "EEE Parsing compilation errors"
//! 	parse_compilation_errors();
//! }
//! // "EEE Building"
//! // (disappears)
//! ```
//!
//! If multiple threads call [`StackedStatus::push`] simultaneously, it will separate the messages with a comma.

use std::{
	collections::HashMap, sync::{Mutex, MutexGuard}, thread::ThreadId, time::Instant
};

type ThreadStack = (Instant, Vec<Option<String>>);

/// A structure that holds stacked status state. See [module documentation](index.html) for details.
pub struct StackedStatus {
	prefix: &'static str,
	stacks: Mutex<HashMap<ThreadId, ThreadStack>>,
}
impl StackedStatus {
	/// Create a state instance with a given message prefix
	pub fn new(prefix: &'static str) -> StackedStatus {
		StackedStatus { prefix, stacks: Mutex::new(HashMap::new()) }
	}

	/// Set the current thread status message and return a guard object that will control its lifetime
	pub fn push(&self, msg: impl AsRef<str>) -> Guard<'_> {
		self.push_impl(Some(msg.as_ref().to_owned()))
	}

	/// Hide the message from the current thread, indicating it is waiting for other threads to progress.
	pub fn push_silence(&self) -> Guard<'_> {
		self.push_impl(None)
	}

	fn push_impl(&self, value: Option<String>) -> Guard<'_> {
		let tid = std::thread::current().id();
		let mut lck = self.stacks.lock().expect("evscode::StackedStatus::push_impl stacks PoisonError");
		lck.entry(tid).or_insert_with(|| (Instant::now(), Vec::new())).1.push(value);
		self.update(lck);
		Guard { stacked: self, tid }
	}

	fn update(&self, lck: MutexGuard<HashMap<ThreadId, ThreadStack>>) {
		let mut entries = lck.values().map(|(t, s)| (*t, s.last().expect("evscode::StackedStatus::update empty stack"))).collect::<Vec<_>>();
		entries.sort();
		let words = entries.iter().filter_map(|(_, word)| word.as_ref()).map(|word| word.as_str()).collect::<Vec<_>>();
		let buf = format!("{}{}", self.prefix, words.join(", "));
		if !words.is_empty() {
			crate::stdlib::status(Some(&buf));
		} else {
			crate::stdlib::status(None);
		}
	}
}

/// Guard object that will remove its associated status message when dropped
pub struct Guard<'a> {
	stacked: &'a StackedStatus,
	tid: ThreadId,
}
impl<'a> Drop for Guard<'a> {
	fn drop(&mut self) {
		let mut lck = self.stacked.stacks.lock().expect("evscode::StackedStatusGuard::drop stacks PoisonError");
		let should_remove = {
			let stack = lck.get_mut(&self.tid).expect("evscode::StackedStatusGuard::drop stack not found");
			stack.1.pop();
			stack.1.is_empty()
		};
		if should_remove {
			lck.remove(&self.tid);
		}
		self.stacked.update(lck);
	}
}
