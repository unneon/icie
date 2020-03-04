use evscode::{
	error::ResultExt, goodies::{dev_tools_logger, DevToolsLogger}, E, R
};
use log::{LevelFilter, Metadata, Record};
use once_cell::sync::Lazy;
use std::{collections::VecDeque, ops::Deref, sync::Mutex};

const LOG_LEVELS: &[(&str, LevelFilter)] = &[
	("cookie_store", log::LevelFilter::Info),
	("html5ever", log::LevelFilter::Info),
	("selectors", log::LevelFilter::Info),
];

const LOG_HISTORY_SIZE: usize = 2048;

static LOGGER: Lazy<Logger> = Lazy::new(|| Logger {
	dev_tools: DevToolsLogger,
	log_history: Mutex::new(VecDeque::with_capacity(LOG_HISTORY_SIZE)),
});

pub fn initialize() -> R<()> {
	log::set_logger(LOGGER.deref()).wrap("logging system initialization failed")?;
	log::set_max_level(LevelFilter::Trace);
	Ok(())
}

pub async fn on_error(error: E) {
	error.backtrace.0.set_name("ICIEError");
	error.backtrace.0.set_message(&error.human_detailed());
	if error.should_auto_report() {
		let log_history = LOGGER.log_history.lock().unwrap();
		let log_history = log_history.iter().map(String::as_str).collect::<Vec<_>>();
		let log_history = log_history.join("\n");
		evscode::telemetry_exception(
			&error,
			&[
				("severity", format!("{:?}", error.severity).as_str()),
				("log_history", &log_history),
			],
			&[],
		);
	}
	error.emit();
}

struct Logger {
	dev_tools: DevToolsLogger,
	log_history: Mutex<VecDeque<String>>,
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
			let mut log_history = self.log_history.lock().unwrap();
			if log_history.len() == LOG_HISTORY_SIZE {
				log_history.pop_front();
			}
			log_history.push_back(dev_tools_logger::format_message(record));
		}
	}

	fn flush(&self) {
		self.dev_tools.flush()
	}
}
