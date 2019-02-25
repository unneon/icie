use super::{Directory, Impulse, Reaction, ICIE};
use crate::{config::Config, status};
use std::{
	panic, sync::{
		mpsc::{self, Receiver, Sender}, Mutex
	}, thread::{self, sleep}, time::Duration
};

pub struct Handle {
	input: Mutex<Sender<Impulse>>,
	output: Mutex<Receiver<Reaction>>,
}
impl Handle {
	pub fn spawn() -> Handle {
		let (es, er) = mpsc::channel();
		let (is, ir) = mpsc::channel();
		let es2 = es.clone();
		let is2 = Mutex::new(is.clone());
		panic::set_hook(Box::new(move |info| {
			if let Ok(is2) = is2.lock() {
				let _ = is2.send(Reaction::Message {
					message: info.to_string(),
					kind: crate::vscode::MessageKind::Error,
					items: None,
					modal: None,
				});
				let mut buf = String::new();
				backtrace::trace(|frame| {
					backtrace::resolve(frame.symbol_address(), |symbol| {
						buf += &format!("{:?} {:?}:{:?}\n", symbol.name(), symbol.filename(), symbol.lineno());
					});
					true
				});
				let _ = is2.send(Reaction::ConsoleError { message: buf }).unwrap();
			}
			loop {
				sleep(Duration::from_secs(1));
			}
		}));

		thread::spawn(move || {
			let config = match Config::load_or_create() {
				Ok(config) => config,
				Err(e) => {
					is.send(Reaction::Message {
						message: format!("failed to load config: {}", e),
						kind: crate::vscode::MessageKind::Error,
						items: None,
						modal: None,
					})
					.unwrap();
					return;
				},
			};
			ICIE {
				input: er,
				output: is,
				input_sender: es2,
				config,
				directory: Directory::new_empty(),
				id_factory: Mutex::new(0),
				status_stack: Mutex::new(status::StatusStack::new()),
			}
			.main_loop()
		});
		Handle {
			input: Mutex::new(es),
			output: Mutex::new(ir),
		}
	}

	pub fn send(&self, message: Impulse) {
		// TODO maybe log failure somewhere
		let _ = self.input.lock().unwrap().send(message);
	}

	pub fn recv(&self) -> Reaction {
		self.output.lock().unwrap().recv().unwrap()
	}
}

#[no_mangle]
pub extern "C" fn __cxa_pure_virtual() {
	loop {}
}
