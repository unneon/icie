#![feature(specialization, impl_trait_in_bindings)]

mod auth;
mod build;
mod debug;
mod dir;
mod discover;
mod init;
mod manifest;
mod paste;
mod template;
mod term;
mod test;
mod util;

lazy_static::lazy_static! {
	pub static ref STATUS: evscode::StackedStatus = evscode::StackedStatus::new("❄️ ");
}

evscode::plugin! {
	name: "icie",
	version: "0.5.0",
	publisher: "pustaczek",
	display_name: "ICIE",
	description: "Competitive programming IDE-as-a-VS-Code-plugin",
	categories: &["Other"],
	keywords: &["competitive", "ide", "codeforces", "olympiad", "informatics"],
	license: "UNLICENSED",
	repository: "https://github.com/matcegla/icie",
	log_bounds: &[
		("html5ever", log::LevelFilter::Error),
		("tokio_reactor", log::LevelFilter::Warn),
		("hyper", log::LevelFilter::Warn),
		("mio", log::LevelFilter::Warn),
		("want", log::LevelFilter::Warn),
	]
}
