use crate::{
	paste::{
		logic::{Library, Piece}, qpaste_doc_error
	}, util::{fs, path::Path}
};
use evscode::{error::ResultExt, E, R};
use futures::lock::{Mutex, MutexGuard};
use std::collections::HashMap;

lazy_static::lazy_static! {
	pub static ref CACHED_LIBRARY: LibraryCache = LibraryCache::new();
}

// TODO: Refactor to Option<Path>
/// Path to your competitive programming library for use with the Alt+[ quickpasting feature. Press
/// Alt+[ with this not set to see how to set up this functionality.
#[evscode::config]
static PATH: evscode::Config<Path> = "";

pub struct LibraryCache {
	lock: Mutex<Library>,
}

impl LibraryCache {
	pub fn new() -> LibraryCache {
		LibraryCache {
			lock: Mutex::new(Library {
				directory: Path::from_native(String::new()),
				pieces: HashMap::new(),
			}),
		}
	}

	#[allow(clippy::extra_unused_lifetimes)]
	pub async fn update(&'static self) -> R<MutexGuard<'_, Library>> {
		let mut lib = self.lock.lock().await;
		let directory = self.get_directory().await?;
		if directory != lib.directory {
			lib.pieces = HashMap::new();
		}
		let mut new_pieces = HashMap::new();
		for path in fs::read_dir(&directory).await? {
			let id = crate::util::without_extension(&path)
				.strip_prefix(&directory)
				.wrap("piece outside the piece collection directory")?
				.to_str()
				.unwrap()
				.to_owned();
			if path.extension() == Some("cpp".to_owned()) {
				let piece = self.maybe_load_piece(path, &id, &mut lib.pieces).await?;
				new_pieces.insert(id, piece);
			}
		}
		lib.directory = directory;
		lib.pieces = new_pieces;
		if lib.pieces.is_empty() {
			return Err(E::error(qpaste_doc_error(
				"your competitive programming library is empty",
			)));
		}
		lib.verify()?;
		Ok(lib)
	}

	async fn maybe_load_piece(
		&self,
		path: Path,
		id: &str,
		cached_pieces: &mut HashMap<String, Piece>,
	) -> R<Piece>
	{
		let modified = fs::metadata(&path).await?.modified;
		let cached = if let Some(cached) = cached_pieces.remove(id) {
			if cached.modified == modified { Some(cached) } else { None }
		} else {
			None
		};
		let piece = if let Some(cached) = cached {
			cached
		} else {
			let code = fs::read_to_string(&path).await?;
			Piece::parse(&code, id.to_owned(), modified)?
		};
		Ok(piece)
	}

	async fn get_directory(&self) -> R<Path> {
		let dir = PATH.get();
		if dir.to_str().unwrap() == "" {
			return Err(E::error(qpaste_doc_error("no competitive programming library found")));
		}
		if !fs::exists(&dir).await? {
			return Err(E::error(format!("directory {} does not exist", dir)));
		}
		Ok(dir)
	}
}
