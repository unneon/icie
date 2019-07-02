use crate::paste::{
	logic::{Library, Piece}, qpaste_doc_error
};
use evscode::{E, R};
use std::{
	collections::HashMap, ffi::OsStr, path::PathBuf, sync::{Mutex, MutexGuard}
};

lazy_static::lazy_static! {
	pub static ref CACHED_LIBRARY: LibraryCache = LibraryCache::new();
}

#[evscode::config(
	description = "Path to your competitive programming library for use with the Alt+[ quickpasting feature. Press Alt+[ with this not set to see how to set up this \
	               functionality."
)]
static PATH: evscode::Config<String> = "";

pub struct LibraryCache {
	lock: Mutex<Library>,
}

impl LibraryCache {
	pub fn new() -> LibraryCache {
		LibraryCache {
			lock: Mutex::new(Library {
				directory: PathBuf::new(),
				pieces: HashMap::new(),
			}),
		}
	}

	pub fn update(&'static self) -> R<MutexGuard<Library>> {
		let mut lib = self.lock.lock().unwrap();
		let directory = self.get_directory()?;
		if directory != lib.directory {
			lib.pieces = HashMap::new();
		}
		let mut new_pieces = HashMap::new();
		for entry in directory
			.read_dir()
			.map_err(|e| E::from_std(e).context(format!("error when reading {} directory", directory.display())))?
		{
			let entry = entry.map_err(E::from_std)?;
			let path = entry.path();
			let id = crate::util::without_extension(&path)
				.strip_prefix(&directory)
				.map_err(E::from_std)?
				.to_str()
				.unwrap()
				.to_owned();
			if path.extension() == Some(OsStr::new("cpp")) {
				let piece = self.maybe_load_piece(path, &id, &mut lib.pieces)?;
				new_pieces.insert(id, dbg!(piece));
			}
		}
		lib.directory = directory;
		lib.pieces = new_pieces;
		if lib.pieces.is_empty() {
			return Err(E::error(qpaste_doc_error("your competitive programming library is empty")));
		}
		lib.verify()?;
		Ok(lib)
	}

	fn maybe_load_piece(&self, path: PathBuf, id: &str, cached_pieces: &mut HashMap<String, Piece>) -> R<Piece> {
		let modified = path.metadata().map_err(E::from_std)?.modified().map_err(E::from_std)?;
		let cached = if let Some(cached) = cached_pieces.remove(id) {
			if cached.modified == modified {
				Some(cached)
			} else {
				None
			}
		} else {
			None
		};
		let piece = if let Some(cached) = cached {
			cached
		} else {
			let code = crate::util::fs_read_to_string(path)?;
			Piece::parse(&code, id.to_owned(), modified)?
		};
		Ok(piece)
	}

	fn get_directory(&self) -> R<PathBuf> {
		let dir = PATH.get();
		if dir.trim().is_empty() {
			return Err(E::error(qpaste_doc_error("no competitive programming library found")));
		}
		let dir = PathBuf::from(shellexpand::tilde(&*dir).into_owned());
		if !dir.exists() {
			return Err(E::error(format!("directory {} does not exist", dir.display())));
		}
		Ok(dir)
	}
}