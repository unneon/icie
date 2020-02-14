//! Logger that logs every passed event to Developer Tools console in VS Code.

use crate::{glue::PACKAGE, meta::Package};
use log::{Log, Metadata, Record};

/// Logger that logs every passed event to Developer Tools console in VS Code.
pub struct DevToolsLogger {
	package: &'static Package,
}

impl DevToolsLogger {
	/// Initialize the logger, so it may cache some common prefixes between logging calls.
	pub fn new() -> DevToolsLogger {
		DevToolsLogger { package: PACKAGE.get().unwrap() }
	}
}

impl Default for DevToolsLogger {
	fn default() -> Self {
		Self::new()
	}
}

impl Log for DevToolsLogger {
	fn enabled(&self, _: &Metadata) -> bool {
		true
	}

	fn log(&self, record: &Record) {
		let message =
			format!("[{}.{}] {}", self.package.publisher, self.package.identifier, record.args());
		crate::console::log(record.level().into(), &message);
	}

	fn flush(&self) {
	}
}
