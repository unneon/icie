//! Developer tools console logging facilities

/// Log a message with a given level.
/// Escape % characters with %% sequence, because advanced logging support is not ready yet.
pub fn log(level: Level, message: &str) {
	let function = match level {
		Level::Error => node_sys::console::error,
		Level::Warn => node_sys::console::warn,
		Level::Info => node_sys::console::info,
		Level::Log => node_sys::console::log,
		Level::Debug => node_sys::console::debug,
	};
	function(message);
}

/// Log levels present in Console API
pub enum Level {
	/// Error level.
	/// Displayed on red background with an error icon.
	Error,
	/// Warning level.
	/// Displayed on orange background with a warning icon.
	Warn,
	/// Information level.
	/// Was once displayed with a blue information icon, but is not anymore.
	Info,
	/// Log level.
	Log,
	/// Debug level.
	/// Not displayed at all unless some options are tweaked.
	Debug,
}
impl From<log::Level> for Level {
	fn from(level: log::Level) -> Self {
		match level {
			log::Level::Error => Level::Error,
			log::Level::Warn => Level::Warn,
			log::Level::Info => Level::Info,
			log::Level::Debug => Level::Log,
			log::Level::Trace => Level::Debug,
		}
	}
}
