mod library;
mod logic;
mod piece_parse;

use crate::{
	dir, paste::logic::{Library, Piece}, telemetry::TELEMETRY, util::time_now
};
use async_trait::async_trait;
use evscode::{error::ResultExt, E, R};

pub struct VscodePaste<'a> {
	solution: String,
	text: String,
	library: &'a Library,
}

#[evscode::command(title = "ICIE Quick Paste", key = "alt+[")]
async fn quick() -> R<()> {
	let _status = crate::STATUS.push("Copy-pasting");
	TELEMETRY.paste_quick.spark();
	let library = library::CACHED_LIBRARY.update().await?;
	let piece_id = select_piece(&library).await?;
	let context = query_context(&library).await?;
	TELEMETRY.paste_quick_ok.spark();
	library.walk_graph(&piece_id, context).await?;
	Ok(())
}

async fn select_piece(library: &Library) -> R<String> {
	let mut pieces = library.pieces.iter().collect::<Vec<_>>();
	pieces.sort_by_key(|piece| &piece.1.name);
	Ok(evscode::QuickPick::new()
		.match_on_all()
		.items(pieces.into_iter().map(|(id, piece)| {
			let mut item = evscode::quick_pick::Item::new(id.clone(), piece.name.clone());
			if let Some(description) = &piece.description {
				item = item.description(description.clone());
			}
			if let Some(detail) = &piece.detail {
				item = item.detail(detail.clone());
			}
			item
		}))
		.show()
		.await
		.ok_or_else(E::cancel)?)
}

#[evscode::command(title = "ICIE Quick input struct", key = "alt+i")]
async fn qistruct() -> R<()> {
	let _status = crate::STATUS.push("Qistructing");
	TELEMETRY.paste_qistruct.spark();
	let name =
		evscode::InputBox::new().prompt("Qistruct name").placeholder("Person").show().await.ok_or_else(E::cancel)?;
	let members = input_members().await?;
	let code = generate_cpp_qistruct(&name, &members);
	let piece = make_hidden_struct_piece(&name, code);
	paste_standalone(piece).await?;
	Ok(())
}

async fn input_members() -> R<Vec<(String, String)>> {
	let mut members = Vec::new();
	loop {
		let prompt = format!("Qistruct member {}", members.len() + 1);
		let answer = evscode::InputBox::new().prompt(&prompt).placeholder("int age").show().await;
		let member = match answer {
			Some(member) if !member.trim().is_empty() => member,
			_ => break,
		};
		let i = member.rfind(' ').wrap("incorrect member syntax, should be e.g., int age")?;
		let member_type = &member[..i];
		let member_name = &member[i + 1..];
		members.push((member_type.to_string(), member_name.to_string()));
	}
	Ok(members)
}

fn generate_cpp_qistruct(name: &str, members: &[(String, String)]) -> String {
	let mut code = format!("struct {} {{\n", name);
	for member in members {
		code += &format!("\t{} {};\n", member.0, member.1);
	}
	code += &format!("\tfriend istream& operator>>(istream& in, {}& x) {{ return in", name);
	for member in members {
		code += &format!(" >> x.{}", member.1);
	}
	code += "; }\n};";
	code
}

fn make_hidden_struct_piece(name: &str, code: String) -> Piece {
	Piece {
		name: String::new(),
		description: None,
		detail: None,
		code,
		guarantee: format!("struct {} {{", name),
		dependencies: Vec::new(),
		parent: None,
		modified: time_now(),
	}
}

async fn paste_standalone(piece: Piece) -> R<()> {
	let mut library = Library::new_empty();
	library.pieces.insert(String::from("__fake_id"), piece);
	let context = query_context(&library).await?;
	library.walk_graph("__fake_id", context).await?;
	Ok(())
}

async fn query_context(library: &Library) -> R<VscodePaste<'_>> {
	let solution = dir::solution()?;
	let text = evscode::query_document_text(&solution).await?;
	let context = VscodePaste { solution: solution.into_string(), text, library };
	Ok(context)
}

#[async_trait(?Send)]
impl logic::PasteContext for VscodePaste<'_> {
	fn has(&mut self, piece_id: &str) -> bool {
		let piece = &self.library.pieces[piece_id];
		self.text.contains(&piece.guarantee)
	}

	async fn paste(&mut self, piece_id: &str) -> R<()> {
		let (position, snippet) = self.library.place(piece_id, &self.text);
		evscode::edit_paste(&self.solution, &snippet, position).await?;
		self.text = evscode::query_document_text(&self.solution).await?;
		Ok(())
	}
}

fn qpaste_doc_error(e: E) -> E {
	e.action("How to use quickpasting?", async {
		evscode::open_external("https://github.com/pustaczek/icie/blob/master/docs/QUICKPASTE.md").await
	})
}
