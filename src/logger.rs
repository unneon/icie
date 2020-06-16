use std::ops::Deref;

use log::{LevelFilter, Metadata, Record};
use once_cell::sync::Lazy;

use evscode::{error::ResultExt, goodies::DevToolsLogger, E, R};

const LOG_LEVELS: &[(&str, LevelFilter)] = &[
	("cookie_store", log::LevelFilter::Info),
	("html5ever", log::LevelFilter::Error),
	("selectors", log::LevelFilter::Info),
];

/// Whether internal application logs should be written to the Developer Console. Too see them, open Help > Toggle
/// Developer Tools, select the Console tab at the top and look for messages beginning with a pustaczek.icie tag.
#[evscode::config]
static ENABLED: evscode::Config<bool> = false;

static LOGGER: Lazy<Logger> = Lazy::new(|| Logger { dev_tools: DevToolsLogger });

pub fn initialize() -> R<()> {
	log::set_logger(LOGGER.deref()).wrap("logging system initialization failed")?;
	log::set_max_level(LevelFilter::Trace);
	Ok(())
}

pub async fn on_error(error: E) {
	error.0.backtrace.0.set_name("ICIEError");
	error.0.backtrace.0.set_message(&error.human_detailed());
	error.emit_log();
	error.emit_user();
}

struct Logger {
	dev_tools: DevToolsLogger,
}

impl log::Log for Logger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		LOG_LEVELS.iter().all(|(source, filter)| metadata.level() <= *filter || !metadata.target().starts_with(source))
	}

	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) && ENABLED.get() {
			self.dev_tools.log(record);
		}
	}

	fn flush(&self) {
		self.dev_tools.flush()
	}
}
