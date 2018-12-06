use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

pub struct QuickPickItem {
	pub label: String,
	pub description: Option<String>,
	pub detail: Option<String>,
	pub id: String,
}

pub enum Impulse {
	Ping,
	QuickPick { response: Option<String> },
}
pub enum Reaction {
	Status { message: Option<String> },
	InfoMessage { message: String },
	ErrorMessage { message: String },
	QuickPick { items: Vec<QuickPickItem> },
}
use self::Impulse::*;
use std::sync::Mutex;

enum State {
	Active,
	Disabled,
}

fn main_loop(input: Receiver<Impulse>, output: Sender<Reaction>) {
	let mut state = State::Disabled;
	for impulse in input {
		match impulse {
			Ping => match state {
				State::Active => {
					state = State::Disabled;
					output.send(Reaction::Status { message: None }).unwrap();
				},
				State::Disabled => {
					state = State::Active;
					output.send(Reaction::Status { message: Some(String::from("❄️ ---")) }).unwrap();
					output.send(Reaction::QuickPick {
						items: vec![
							QuickPickItem { label: "Strategy A".to_owned(), description: Some("6 hours".to_owned()), detail: None, id: "strategy_a".to_owned() },
							QuickPickItem { label: "Strategy B".to_owned(), description: Some("7 hours".to_owned()), detail: None, id: "strategy_b".to_owned() },
							QuickPickItem { label: "Analyzing whether strategy A or B is more efficient".to_owned(), description: Some("56 hours".to_owned()), detail: None, id: "analyzing".to_owned() },
						]
					}).unwrap();
				},
			},
			QuickPick { response } => match state {
				State::Active => match response {
					Some(response) => {
						let label = match response.as_str() {
							"strategy_a" => "Strategy A",
							"strategy_b" => "Strategy B",
							"analyzing" => "Analyzing whether strategy A or B is more efficient",
							_ => {
								state = State::Disabled;
								output.send(Reaction::Status { message: None }).unwrap();
								output.send(Reaction::ErrorMessage { message: format!("Unrecognized quick pick response: {:?}", response) }).unwrap();
								continue;
							}
						};
						output.send(Reaction::Status { message: Some(format!("❄️ {}", label)) }).unwrap();
					},
					None => {
						state = State::Disabled;
						output.send(Reaction::Status { message: None }).unwrap();
					},
				},
				State::Disabled => output.send(Reaction::ErrorMessage { message: "Unexpected quick pick response".to_owned() }).unwrap(),
			},
		}
	}
}

pub struct ICIE {
	input: Mutex<Sender<Impulse>>,
	output: Mutex<Receiver<Reaction>>,
}
impl ICIE {
	pub fn spawn() -> ICIE {
		let (es, er) = mpsc::channel();
		let (is, ir) = mpsc::channel();
		thread::spawn(move || main_loop(er, is));
		ICIE { input: Mutex::new(es), output: Mutex::new(ir) }
	}
	pub fn send(&self, message: Impulse) {
		self.input.lock().unwrap().send(message).unwrap()
	}
	pub fn recv(&self) -> Reaction {
		self.output.lock().unwrap().recv().unwrap()
	}
}

