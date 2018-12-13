use super::{Directory, Impulse, Reaction, ICIE};
use std::{
	sync::{
		mpsc::{self, Receiver, Sender}, Mutex
	}, thread
};

pub struct Handle {
	input: Mutex<Sender<Impulse>>,
	output: Mutex<Receiver<Reaction>>,
}
impl Handle {
	pub fn spawn() -> Handle {
		let (es, er) = mpsc::channel();
		let (is, ir) = mpsc::channel();
		thread::spawn(move || {
			ICIE {
				input: er,
				output: is,
				directory: Directory::new_empty(),
			}
			.main_loop()
		});
		Handle {
			input: Mutex::new(es),
			output: Mutex::new(ir),
		}
	}

	pub fn send(&self, message: Impulse) {
		self.input.lock().unwrap().send(message).unwrap()
	}

	pub fn recv(&self) -> Reaction {
		self.output.lock().unwrap().recv().unwrap()
	}
}
