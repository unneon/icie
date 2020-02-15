use evscode::{
	error::{ResultExt, Severity}, goodies::{dev_tools_logger, DevToolsLogger}, E, R
};
use log::{LevelFilter, Metadata, Record};
use once_cell::sync::Lazy;
use std::{collections::VecDeque, sync::Mutex};

const LOG_LEVELS: &[(&str, LevelFilter)] = &[
	("cookie_store", log::LevelFilter::Info),
	("html5ever", log::LevelFilter::Info),
	("selectors", log::LevelFilter::Info),
];

const LOG_HISTORY_SIZE: usize = 256;

pub fn initialize() -> R<()> {
	log::set_boxed_logger(Box::new(Logger { dev_tools: DevToolsLogger }))
		.wrap("logging system initialization failed")?;
	log::set_max_level(LevelFilter::Trace);
	Ok(())
}

pub async fn on_error(error: E) {
	error.backtrace.0.set_name("ICIEError");
	error.backtrace.0.set_message(&error.human_detailed());
	if error.severity == Severity::Error {
		let log_history =
			LOG_HISTORY.lock().unwrap().iter().map(String::as_str).collect::<Vec<_>>().join("\n");
		evscode::telemetry_exception(&error, &[("log_history", log_history.as_str())], &[]);
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
			let mut log_history = LOG_HISTORY.lock().unwrap();
			if log_history.len() == LOG_HISTORY_SIZE {
				log_history.pop_front();
			}
			log_history.push_front(dev_tools_logger::format_message(record));
		}
	}

	fn flush(&self) {
		self.dev_tools.flush()
	}
}

static LOG_HISTORY: Lazy<Mutex<VecDeque<String>>> =
	Lazy::new(|| Mutex::new(VecDeque::with_capacity(LOG_HISTORY_SIZE)));
