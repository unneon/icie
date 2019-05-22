#![feature(specialization)]

mod auth;
mod build;
mod ci;
mod debug;
mod dir;
mod discover;
mod init;
mod launch;
mod manifest;
mod net;
mod paste;
mod submit;
mod template;
mod term;
mod test;
mod util;

lazy_static::lazy_static! {
	pub static ref STATUS: evscode::StackedStatus = evscode::StackedStatus::new("❄️ ");
}

evscode::plugin! {
	name: "ICIE",
	publisher: "pustaczek",
	description: "Competitive programming IDE-as-a-VS-Code-plugin",
	keywords: &["competitive", "ide", "codeforces", "olympiad", "informatics"],
	categories: &["Other"],
	license: "GPL-3.0-only",
	repository: "https://github.com/pustaczek/icie",
	on_activate: Some(launch::activate),
	extra_activation_events: &[
		evscode::ActivationEvent::WorkspaceContains { selector: ".icie" },
	],
	log_filters: &[
		("html5ever", log::LevelFilter::Error),
		("tokio_reactor", log::LevelFilter::Warn),
		("hyper", log::LevelFilter::Warn),
		("mio", log::LevelFilter::Warn),
		("want", log::LevelFilter::Warn),
	],
}
