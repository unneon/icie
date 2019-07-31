use crate::internal::executor::send_object;
use json::object;
use log::{Level, Log, Metadata, Record};

pub struct VSCodeLoger {
	pub blacklist: radix_trie::Trie<&'static str, log::LevelFilter>,
}

pub static mut LOGGER_SLOT: Option<VSCodeLoger> = None;

impl Log for VSCodeLoger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		let target = metadata.target();
		let target_crate = &target[..target.find(':').unwrap_or(target.len())];
		if let Some(filter) = self.blacklist.get(target_crate) { metadata.level() <= *filter } else { true }
	}

	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) {
			send_object(object! {
				"tag" => "console",
				"level" => match record.level() {
					Level::Error => "error",
					Level::Warn => "warn",
					Level::Info => "info",
					Level::Debug => "log",
					Level::Trace => "debug",
				},
				"message" => format!("{}", record.args()),
			});
		}
	}

	fn flush(&self) {
	}
}
