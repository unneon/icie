//! Logger that logs every passed event to Developer Tools console in VS Code.

use crate::glue::PACKAGE;
use log::{Log, Metadata, Record};

/// Formats the message in the same way it will be displayed in Developer Tools.
pub fn format_message(record: &Record) -> String {
	format!(
		"[{}.{}] {}",
		PACKAGE.get().unwrap().publisher,
		PACKAGE.get().unwrap().identifier,
		record.args()
	)
}

/// Logger that logs every passed event to Developer Tools console in VS Code.
pub struct DevToolsLogger;

impl Log for DevToolsLogger {
	fn enabled(&self, _: &Metadata) -> bool {
		true
	}

	fn log(&self, record: &Record) {
		crate::console::log(record.level().into(), &format_message(record));
	}

	fn flush(&self) {
	}
}
