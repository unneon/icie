#![feature(const_fn, exhaustive_patterns, never_type, specialization, try_blocks)]
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
mod logger;
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
	on_activate: Some(Box::new(|| Box::pin(launch::activate()))),
	on_deactivate: Some(Box::new(|| Box::pin(launch::deactivate()))),
	on_error: Some(Box::new(|e| Box::pin(logger::on_error(e)))),
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
	telemetry_key: "d131172a-874d-4c0a-b02f-dbf4c951de3c",
}
