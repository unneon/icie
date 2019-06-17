use std::io::Read;

pub type R<T> = Result<T, std::io::Error>;

pub fn io_read<T: Read>(mut f: T) -> std::io::Result<Vec<u8>> {
	let mut buf = Vec::new();
	f.read_to_end(&mut buf)?;
	Ok(buf)
}
