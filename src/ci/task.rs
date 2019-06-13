use crate::ci::exec::Environment;

pub trait Checker {
	fn judge(&self, input: &str, desired: &str, out: &str) -> bool;
}

pub struct Task {
	pub checker: Box<dyn Checker+Send>,
	pub environment: Environment,
}

pub struct FreeWhitespaceChecker;
impl Checker for FreeWhitespaceChecker {
	fn judge(&self, _input: &str, desired: &str, out: &str) -> bool {
		self.equal_bew(desired, out)
	}
}
impl FreeWhitespaceChecker {
	fn equal_bew(&self, a: &str, b: &str) -> bool {
		let mut i = a.chars().peekable();
		let mut j = b.chars().peekable();
		while i.peek().is_some() && j.peek().is_some() {
			if i.peek().unwrap().is_whitespace() && j.peek().unwrap().is_whitespace() {
				while i.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
					i.next();
				}
				while j.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
					j.next();
				}
			} else {
				if i.peek() != j.peek() {
					return false;
				}
				i.next();
				j.next();
			}
		}
		for c in i {
			if !c.is_whitespace() {
				return false;
			}
		}
		for c in j {
			if !c.is_whitespace() {
				return false;
			}
		}
		true
	}
}
