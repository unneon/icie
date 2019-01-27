use crate::{Reaction, ICIE};

pub struct StatusStack {
	stack: Vec<String>,
}

impl StatusStack {
	pub fn new() -> StatusStack {
		StatusStack { stack: Vec::new() }
	}

	pub fn push(&mut self, message: &str) -> Reaction {
		let text = StatusStack::text_from_message(message);
		self.stack.push(text.clone());
		Reaction::Status { message: Some(text) }
	}

	pub fn pop(&mut self) -> Reaction {
		self.stack.pop();
		Reaction::Status {
			message: self.stack.last().map(String::clone),
		}
	}

	fn text_from_message(message: &str) -> String {
		format!("❄️ {}", message)
	}
}

pub(crate) struct Status<'a> {
	icie: &'a ICIE,
}

impl<'a> Status<'a> {
	pub fn new<'b>(msg: &'b str, icie: &'a ICIE) -> Status<'a> {
		let mut stack = icie.status_stack.lock().unwrap();
		icie.send(stack.push(msg));
		Status { icie }
	}
}
impl<'a> Drop for Status<'a> {
	fn drop(&mut self) {
		let mut stack = self.icie.status_stack.lock().unwrap();
		self.icie.send(stack.pop());
	}
}
