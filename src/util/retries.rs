use crate::{util::sleep, STATUS};
use std::time::Duration;

pub struct Retries {
	left: usize,
	delay: Duration,
}

impl Retries {
	pub fn new(left: usize, delay: Duration) -> Retries {
		Retries { left, delay }
	}

	pub async fn wait(&mut self) -> bool {
		if self.left > 0 {
			let _status = STATUS.push("Retrying...");
			self.left -= 1;
			sleep(self.delay).await;
			true
		} else {
			false
		}
	}
}
