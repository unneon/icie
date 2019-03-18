use crate::{
	error::{self, R}, vscode
};
use std::{collections::HashMap, fs::File, path::Path};

#[derive(Debug, Deserialize)]
pub struct Library {
	pub pieces: HashMap<String, Piece>,
}
#[derive(Debug, Deserialize)]
pub struct Piece {
	pub name: String,
	pub description: Option<String>,
	pub detail: Option<String>,
	pub code: String,
	pub request: String,
	pub guarantee: String,
	pub dependencies: Vec<String>,
}

impl Library {
	pub fn load(path: &Path) -> R<Library> {
		let file = File::open(path)?;
		let library: Library = serde_json::from_reader(file)?;
		library.verify()?;
		Ok(library)
	}

	fn verify(&self) -> R<()> {
		for (_, piece) in &self.pieces {
			for dep in &piece.dependencies {
				if !self.pieces.contains_key(dep) {
					return Err(error::Category::MalformedLibrary {
						detail: "dependency does not exist",
					}
					.err())?;
				}
			}
		}
		// TODO check for dependency cycles
		Ok(())
	}

	pub fn place(&self, piece: &Piece, source: &str) -> R<(vscode::Position, String)> {
		let index = self.place_index(piece, source)?;
		let position = index_to_position(index, source);
		Ok((position, format!("{}\n", piece.code.clone())))
	}

	fn place_index(&self, piece: &Piece, source: &str) -> R<usize> {
		let mut pos = source.find("using namespace std").map(|i| i + 1).unwrap_or(0);
		for dep_id in &piece.dependencies {
			let dep = &self.pieces[dep_id];
			pos += source[pos..].find(&dep.guarantee).map(|i| i + 1).unwrap_or(0);
		}
		pos += source[pos..].find('\n').map(|i| i + 1).unwrap_or(0);
		Ok(pos)
	}
}

fn index_to_position(index: usize, source: &str) -> vscode::Position {
	let line = source[..index].chars().filter(|c| *c == '\n').count();
	vscode::Position { line: line as i64, character: 0 }
}
