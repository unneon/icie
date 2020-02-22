#[derive(Debug)]
pub struct Backtrace(pub js_sys::Error);

impl Backtrace {
	pub fn new() -> Backtrace {
		Backtrace(js_sys::Error::new(""))
	}
}

impl Default for Backtrace {
	fn default() -> Backtrace {
		Backtrace::new()
	}
}
