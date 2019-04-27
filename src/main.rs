#![feature(specialization)]

#[evscode::command(title = "Hello world!", key = "alt+0")]
fn hello_world() {
	evscode::InfoMessage::new("Hello, world!").spawn();
}

evscode::plugin! {
	name: "icie",
	version: "0.5.0",
	publisher: "pustaczek",
	display_name: "ICIE",
	description: "Competitive programming IDE-as-a-VS-Code-plugin",
	categories: &["Other"],
	keywords: &["competitive", "ide", "codeforces", "olympiad", "informatics"],
	repository: "https://github.com/matcegla/icie"
}
