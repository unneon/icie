#![feature(exhaustive_patterns, never_type, specialization, try_blocks)]
// This lint works badly in generic contexts, causing warnings inside #[async_trait] macros.
#![allow(clippy::unit_arg, clippy::unused_unit, clippy::vec_init_then_push, incomplete_features)]

use once_cell::sync::Lazy;

mod assets;
mod auth;
mod checker;
mod compile;
mod debug;
mod dir;
mod executable;
mod launch;
mod logger;
mod manifest;
mod net;
mod newsletter;
mod open;
mod paste;
mod service;
mod stress;
mod submit;
mod template;
mod terminal;
mod test;
mod tutorial;
mod util;
mod submissions;

pub static STATUS: Lazy<evscode::goodies::MultiStatus> = Lazy::new(|| evscode::goodies::MultiStatus::new("❄️"));

evscode::plugin! {
	name: "ICIE",
	publisher: "georgejsh",
	description: "Competitive programming IDE-as-a-VS-Code-plugin",
	keywords: &["competitive", "contest", "codeforces", "atcoder", "codechef"],
	categories: &["Other"],
	license: "MPL-2.0",
	repository: "https://github.com/georgejsh/icie",
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
		("node-fetch", "2.6.0"),
	],
}
