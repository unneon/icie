use evscode::{error::ResultExt, goodies::DevToolsLogger, E, R};
use log::{LevelFilter, Metadata, Record};

const LOG_LEVELS: &[(&str, LevelFilter)] = &[
	("cookie_store", log::LevelFilter::Info),
	("html5ever", log::LevelFilter::Info),
	("selectors", log::LevelFilter::Info),
];

pub fn initialize() -> R<()> {
	log::set_boxed_logger(Box::new(Logger { dev_tools: DevToolsLogger }))
		.wrap("logging system initialization failed")?;
	log::set_max_level(LevelFilter::Trace);
	Ok(())
}

pub async fn on_error(error: E) {
	error.backtrace.0.set_name("ICIEError");
	error.backtrace.0.set_message(&error.human_detailed());
	if error.should_auto_report() {
		evscode::telemetry_exception(
			&error,
			&[("severity", format!("{:?}", error.severity).as_str())],
			&[],
		);
	}
	error.emit();
}

struct Logger {
	dev_tools: DevToolsLogger,
}

impl log::Log for Logger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		LOG_LEVELS.iter().all(|(source, filter)| {
			metadata.level() <= *filter || !metadata.target().starts_with(source)
		})
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
