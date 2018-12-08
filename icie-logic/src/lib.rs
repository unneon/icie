#[macro_use]
mod error;
mod handle;
mod vscode;

pub use self::handle::Handle;
use self::{error::R, vscode::*};
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum Impulse {
	Ping,
	QuickPick { response: Option<String> },
	InputBox { response: Option<String> },
}
pub enum Reaction {
	Status { message: Option<String> },
	InfoMessage { message: String },
	ErrorMessage { message: String },
	QuickPick { items: Vec<QuickPickItem> },
	InputBox { options: InputBoxOptions },
}

struct ICIE {
	input: Receiver<Impulse>,
	output: Sender<Reaction>,
}
impl ICIE {
	fn main_loop(&mut self) {
		loop {
			match self.process() {
				Ok(()) => (),
				Err(err) => self
					.output
					.send(Reaction::ErrorMessage {
						message: format!("ICIE Error ❄ {}", err),
					})
					.unwrap(),
			}
		}
	}

	fn process(&mut self) -> R<()> {
		Ok(())
	}
}
