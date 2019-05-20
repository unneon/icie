use std::{
	io::Read, time::{Duration, Instant}
};

pub type R<T> = Result<T, std::io::Error>;

pub fn time_fn<T, F: FnOnce() -> T>(f: F) -> (Duration, T) {
	let t1 = Instant::now();
	let r = f();
	let t2 = Instant::now();
	let dt = t2 - t1;
	(dt, r)
}

pub fn io_read<T: Read>(mut f: T) -> std::io::Result<Vec<u8>> {
	let mut buf = Vec::new();
	f.read_to_end(&mut buf)?;
	Ok(buf)
}
