use log::{Log, Metadata, Record};

pub struct VSCodeLoger {
	pub blacklist: radix_trie::Trie<&'static str, log::LevelFilter>,
}

impl Log for VSCodeLoger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		let target = metadata.target();
		let target_crate = &target[..target.find(':').unwrap_or_else(|| target.len())];
		if let Some(filter) = self.blacklist.get(target_crate) {
			metadata.level() <= *filter
		} else {
			true
		}
	}

	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) {
			crate::console::log(record.level().into(), &format!("{}", record.args()));
		}
	}

	fn flush(&self) {
	}
}
