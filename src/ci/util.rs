use std::io::Read;

pub fn io_read<T: Read>(mut f: T) -> std::io::Result<Vec<u8>> {
	let mut buf = Vec::new();
	f.read_to_end(&mut buf)?;
	Ok(buf)
}
