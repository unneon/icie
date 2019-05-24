mod logic;

use crate::dir;
use logic::{Library, Piece};
use std::path::{Path, PathBuf};

#[evscode::config(description = "Where to find a .json with library piece definitions")]
static LIBRARY_PATH: evscode::Config<String> = "";

#[evscode::command(title = "ICIE Quick Paste", key = "alt+[")]
fn quick() -> evscode::R<()> {
	let _status = crate::STATUS.push("Copy-pasting");
	let library = load_library()?;
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
		.ok_or_else(evscode::E::cancel)?;
	let context = query_context(&library)?;
	library.walk_graph(&piece_id, context)?;
	Ok(())
}

#[evscode::command(title = "ICIE Quick input struct", key = "alt+i")]
fn qistruct() -> evscode::R<()> {
	let _status = crate::STATUS.push("Qistructing");
	let name = evscode::InputBox::new()
		.prompt("Qistruct name")
		.placeholder("Person")
		.build()
		.wait()
		.ok_or_else(evscode::E::cancel)?;
	let mut members = Vec::new();
	loop {
		let member = match evscode::InputBox::new()
			.prompt(format!("Qistruct member {}", members.len() + 1))
			.placeholder("int age")
			.build()
			.wait()
		{
			Some(ref member) if member.trim() == "" => break,
			Some(member) => member,
			None => break,
		};
		let i = member.rfind(' ').ok_or_else(|| evscode::E::error("incorrect member syntax, should be e.g., int age"))?;
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
	let piece = logic::Piece {
		name: String::new(),
		description: None,
		detail: None,
		code,
		request: String::new(),
		guarantee: format!("struct {} {{", name),
		dependencies: Vec::new(),
		parent: None,
	};
	let mut library = load_library()?;
	library.pieces.insert(String::from("__qistruct"), piece);
	let context = query_context(&library)?;
	library.walk_graph("__qistruct", context)?;
	Ok(())
}

fn load_library() -> evscode::R<Library> {
	let path = LIBRARY_PATH.get();
	if path.trim().is_empty() {
		return Err(evscode::E::error("Library path not set, change it in settings(Ctrl+,) under Icie Paste Library Path"));
	}
	let library = Library::load(Path::new(&*shellexpand::tilde(&*path)))?;
	Ok(library)
}

fn query_context<'a>(library: &'a Library) -> evscode::R<VscodePaste<'a>> {
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
	fn has(&mut self, piece: &Piece) -> bool {
		self.text.contains(&piece.guarantee)
	}

	fn paste(&mut self, piece_id: &str) -> evscode::R<()> {
		let piece = &self.library.pieces[piece_id];
		log::info!("Wanna paste: {}", piece.name);
		let (position, snippet) = self.library.place(piece_id, &self.text)?;
		evscode::edit_paste(self.solution.clone(), snippet, position).wait();
		self.text = evscode::query_document_text(self.solution.clone()).wait();
		Ok(())
	}
}
