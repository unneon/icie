//! Developer tools console logging facilities

use crate::internal::executor::send_object;

/// Log a message with a given level.
/// Escape % characters with %% sequence, because advanced logging support is not ready yet.
pub fn log(level: Level, message: impl AsRef<str>) {
	send_object(json::object! {
		"tag" => "console",
		"level" => match level {
			Level::Error => "error",
			Level::Warn => "warn",
			Level::Info => "info",
			Level::Log => "log",
			Level::Debug => "debug",
		},
		"message" => message.as_ref(),
	})
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
