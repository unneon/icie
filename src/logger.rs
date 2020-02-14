use evscode::{error::ResultExt, goodies::DevToolsLogger, R};
use log::{LevelFilter, Metadata, Record};

pub fn initialize() -> R<()> {
	log::set_boxed_logger(Box::new(Logger { dev_tools: DevToolsLogger::new() }))
		.wrap("logging system initialization failed")?;
	log::set_max_level(LevelFilter::Trace);
	Ok(())
}

const LOG_LEVELS: &[(&str, LevelFilter)] = &[
	("cookie_store", log::LevelFilter::Info),
	("html5ever", log::LevelFilter::Info),
	("selectors", log::LevelFilter::Info),
];

struct Logger {
	dev_tools: DevToolsLogger,
}

impl log::Log for Logger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		LOG_LEVELS.iter().all(|(source, filter)| {
			metadata.level() <= *filter || !metadata.target().starts_with(source)
		}) && self.dev_tools.enabled(metadata)
	}

	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) {
			self.dev_tools.log(record);
		}
	}

	fn flush(&self) {
		self.dev_tools.flush()
	}
}
