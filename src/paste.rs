mod library;
mod logic;
mod piece_parse;

use crate::dir;
use evscode::{E, R};
use logic::{Library, Piece};
use std::{path::PathBuf, time::SystemTime};

#[evscode::command(title = "ICIE Quick Paste", key = "alt+[")]
fn quick() -> R<()> {
	let _status = crate::STATUS.push("Copy-pasting");
	let library = library::CACHED_LIBRARY.update()?;
	let piece_id = evscode::QuickPick::new()
		.match_on_all()
		.items(library.pieces.iter().map(|(id, piece)| {
			let mut item = evscode::quick_pick::Item::new(id, &piece.name);
			if let Some(description) = &piece.description {
				item = item.description(description);
			}
			if let Some(detail) = &piece.detail {
				item = item.detail(detail);
			}
			item
		}))
		.build()
		.wait()
		.ok_or_else(E::cancel)?;
	let context = query_context(&library)?;
	library.walk_graph(&piece_id, context)?;
	Ok(())
}

#[evscode::command(title = "ICIE Quick input struct", key = "alt+i")]
fn qistruct() -> R<()> {
	let _status = crate::STATUS.push("Qistructing");
	let name = evscode::InputBox::new().prompt("Qistruct name").placeholder("Person").build().wait().ok_or_else(E::cancel)?;
	let mut members = Vec::new();
	while let Some(member) = evscode::InputBox::new().prompt(format!("Qistruct member {}", members.len() + 1)).placeholder("int age").build().wait() {
		if member.trim().is_empty() {
			break;
		}
		let i = member.rfind(' ').ok_or_else(|| E::error("incorrect member syntax, should be e.g., int age"))?;
		let typ = &member[..i];
		let ide = &member[i + 1..];
		members.push((typ.to_string(), ide.to_string()));
	}
	let mut code = format!("struct {} {{\n", name);
	for (typ, ide) in &members {
		code += &format!("\t{} {};\n", typ, ide);
	}
	code += &format!("\tfriend istream& operator>>(istream& in, {}& x) {{ return in", name);
	for (_, ide) in &members {
		code += &format!(" >> x.{}", ide);
	}
	code += "; }\n};";
	let piece = Piece {
		name: String::new(),
		description: None,
		detail: None,
		code,
		guarantee: format!("struct {} {{", name),
		dependencies: Vec::new(),
		parent: None,
		modified: SystemTime::now(),
	};
	let mut library = Library::new_empty();
	library.pieces.insert(String::from("__qistruct"), piece);
	let context = query_context(&library)?;
	library.walk_graph("__qistruct", context)?;
	Ok(())
}

fn query_context(library: &Library) -> R<VscodePaste> {
	let solution = dir::solution()?;
	let text = evscode::query_document_text(solution.clone()).wait();
	let context = VscodePaste { solution, text, library };
	Ok(context)
}

pub struct VscodePaste<'a> {
	solution: PathBuf,
	text: String,
	library: &'a Library,
}
impl logic::PasteContext for VscodePaste<'_> {
	fn has(&mut self, piece_id: &str) -> bool {
		let piece = &self.library.pieces[piece_id];
		self.text.contains(&piece.guarantee)
	}

	fn paste(&mut self, piece_id: &str) -> R<()> {
		let (position, snippet) = self.library.place(piece_id, &self.text);
		evscode::edit_paste(self.solution.clone(), snippet, position).wait();
		self.text = evscode::query_document_text(self.solution.clone()).wait();
		Ok(())
	}
}

fn qpaste_doc_error(s: impl AsRef<str>) -> String {
	format!("{}; see [quickpasting docs](https://github.com/pustaczek/icie#quickpasting-setup)", s.as_ref())
}
