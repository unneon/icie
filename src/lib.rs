#![feature(const_fn, exhaustive_patterns, specialization, try_blocks)]
// This lint works badly in generic contexts, causing warnings inside #[async_trait] macros.
#![allow(clippy::unit_arg)]

mod auth;
mod build;
mod checker;
mod debug;
mod dir;
mod discover;
mod executable;
mod init;
mod interpolation;
mod launch;
mod manifest;
mod net;
mod newsletter;
mod paste;
mod service;
mod submit;
mod telemetry;
mod template;
mod term;
mod test;
mod tutorial;
mod util;

lazy_static::lazy_static! {
	pub static ref STATUS: evscode::goodies::MultiStatus = evscode::goodies::MultiStatus::new("❄️");
}

evscode::plugin! {
	name: "ICIE",
	publisher: "pustaczek",
	description: "Competitive programming IDE-as-a-VS-Code-plugin",
	keywords: &["competitive", "contest", "codeforces", "atcoder", "codechef"],
	categories: &["Other"],
	license: "GPL-3.0-only",
	repository: "https://github.com/pustaczek/icie",
	gallery: evscode::meta::Gallery {
		color: "#6d0759",
		theme: evscode::meta::GalleryTheme::Dark,
	},
	on_activate: Some(Box::pin(launch::activate())),
	on_deactivate: Some(Box::pin(launch::deactivate())),
	extra_activations: &[
		evscode::meta::Activation::WorkspaceContains { selector: ".icie" },
		evscode::meta::Activation::WorkspaceContains { selector: ".icie-contest" },
	],
	vscode_version: "^1.33.0",
	node_dependencies: &[
		("keytar", "5.0.0-beta.3"),
		("node-fetch", "2.6.0"),
		("vscode-extension-telemetry", "0.1.2")
	],
	telemetry_key: "b05c4c82-d1e6-44f5-aa16-321230ad2475",
	log_filters: &[
		("cookie_store", log::LevelFilter::Info),
		("html5ever", log::LevelFilter::Info),
		("reqwest", log::LevelFilter::Info),
		("selectors", log::LevelFilter::Info),
	],
}
