use crate::paste::{logic::Piece, qpaste_doc_error};
use evscode::{E, R};
use regex::Regex;
use std::{collections::HashMap, time::SystemTime};

impl Piece {
	pub fn parse(code: &str, id: String, modified: SystemTime) -> R<Piece> {
		let headers = code
			.lines()
			.filter(|line| line.starts_with("///"))
			.map(|line| {
				let m: regex::Captures = HEADER_REGEX.captures(line).ok_or_else(|| E::error(qpaste_doc_error("invalid header in qpaste piece")))?;
				Ok((m[1].to_string(), m[2].to_string()))
			})
			.collect::<R<HashMap<_, _>>>()?;
		let headers = Headers { id: &id, headers };
		let name = headers.field("Name")?.to_owned();
		let description = headers.optfield("Description").map(String::from);
		let detail = headers.optfield("Detail").map(String::from);
		let guarantee = headers.field("Guarantee")?.to_owned();
		let dependencies = headers
			.optfield("Dependencies")
			.unwrap_or("")
			.split(",")
			.filter(|s| !s.trim().is_empty())
			.map(|s| s.trim().to_owned())
			.collect::<Vec<_>>();
		let parent = headers.optfield("Parent").map(String::from);
		let code = code.lines().filter(|line| !line.starts_with("///")).collect::<Vec<_>>().join("\n");
		Ok(Piece {
			name,
			description,
			detail,
			code,
			guarantee,
			dependencies,
			parent,
			modified,
		})
	}
}

struct Headers<'a> {
	id: &'a str,
	headers: HashMap<String, String>,
}
impl Headers<'_> {
	fn field(&self, key: &str) -> R<&str> {
		match self.headers.get(key).map(|value| value.trim()) {
			Some("") | None => Err(E::error(format!("piece {:?} does not have the {:?} header", self.id, key))),
			Some(value) => Ok(value),
		}
	}

	fn optfield(&self, key: &str) -> Option<&str> {
		self.headers.get(key).and_then(|value| if value.trim().is_empty() { None } else { Some(value.trim()) })
	}
}

lazy_static::lazy_static! {
	static ref HEADER_REGEX: Regex = Regex::new("///\\s*(\\w+):\\s*(.*)").unwrap();
}
