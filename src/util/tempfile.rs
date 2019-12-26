use crate::util::{fs, path::Path, time_now};
use evscode::R;
use std::time::UNIX_EPOCH;

pub struct Tempfile {
	path: Path,
}

impl Tempfile {
	pub async fn new(uniq_name: &str, data: impl AsRef<[u8]>) -> R<Tempfile> {
		let id = time_now().duration_since(UNIX_EPOCH).unwrap().as_micros() % 1_000_000;
		let path =
			Path::from_native(node_sys::os::tmpdir()).join(format!("icie_{}_{}", uniq_name, id));
		fs::write(&path, data.as_ref()).await?;
		Ok(Tempfile { path })
	}

	pub fn path(&self) -> &Path {
		&self.path
	}
}

impl Drop for Tempfile {
	fn drop(&mut self) {
		fs::remove_file_sync(&self.path).unwrap();
	}
}
