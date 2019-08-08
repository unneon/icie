#![feature(never_type, proc_macro_hygiene, exhaustive_patterns, specialization, try_blocks, duration_float, bind_by_move_pattern_guards)]

mod auth;
mod build;
mod checker;
mod ci;
mod debug;
mod dir;
mod discover;
mod init;
mod interpolation;
mod launch;
mod manifest;
mod net;
mod newsletter;
mod paste;
mod submit;
mod template;
mod term;
mod test;
mod tutorial;
mod util;

lazy_static::lazy_static! {
	pub static ref STATUS: evscode::StackedStatus = evscode::StackedStatus::new("❄️ ");
}

evscode::plugin! {
	name: "ICIE",
	publisher: "pustaczek",
	description: "Competitive programming IDE-as-a-VS-Code-plugin",
	keywords: &["competitive", "contest", "codeforces", "atcoder", "spoj"],
	categories: &["Other"],
	license: "GPL-3.0-only",
	repository: "https://github.com/pustaczek/icie",
	on_activate: Some(launch::activate),
	extra_activations: &[
		evscode::meta::Activation::WorkspaceContains { selector: ".icie" },
	],
	log_filters: &[
		("cookie_store", log::LevelFilter::Info),
		("html5ever", log::LevelFilter::Error),
		("hyper", log::LevelFilter::Info),
		("mio", log::LevelFilter::Info),
		("reqwest", log::LevelFilter::Info),
		("rustls", log::LevelFilter::Info),
		("selectors", log::LevelFilter::Info),
		("tokio_reactor", log::LevelFilter::Info),
		("want", log::LevelFilter::Info),
	],
}
