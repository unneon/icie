#![feature(specialization, impl_trait_in_bindings)]

mod build;
mod debug;
mod dir;
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
	repository: "https://github.com/matcegla/icie"
}
