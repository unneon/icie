use crate::paste::{logic::Piece, qpaste_doc_error};
use evscode::{error::ResultExt, E, R};
use regex::Regex;
use std::{collections::HashMap, time::SystemTime};

impl Piece {
	pub fn parse(code: &str, id: String, modified: SystemTime) -> R<Piece> {
		let headers = code
			.lines()
			.filter(|line| line.starts_with("///"))
			.map(|line| {
				let m: regex::Captures = HEADER_REGEX
					.captures(line)
					.wrap(qpaste_doc_error("invalid header in qpaste piece"))?;
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
			.split(',')
			.filter(|s| !s.trim().is_empty())
			.map(|s| s.trim().to_owned())
			.collect::<Vec<_>>();
		let parent = headers.optfield("Parent").map(String::from);
		let code =
			code.lines().filter(|line| !line.starts_with("///")).collect::<Vec<_>>().join("\n");
		Ok(Piece { name, description, detail, code, guarantee, dependencies, parent, modified })
	}
}

struct Headers<'a> {
	id: &'a str,
	headers: HashMap<String, String>,
}
impl Headers<'_> {
	fn field(&self, key: &str) -> R<&str> {
		match self.headers.get(key).map(|value| value.trim()) {
			Some("") | None => {
				Err(E::error(format!("piece {:?} does not have the {:?} header", self.id, key)))
			},
			Some(value) => Ok(value),
		}
	}

	fn optfield(&self, key: &str) -> Option<&str> {
		self.headers
			.get(key)
			.and_then(|value| if value.trim().is_empty() { None } else { Some(value.trim()) })
	}
}

lazy_static::lazy_static! {
	static ref HEADER_REGEX: Regex = Regex::new("///\\s*(\\w+):\\s*(.*)").unwrap();
}

#[test]
fn test_simple() {
	let code = r#"/// Name: FU
/// Description: Find & Union
/// Detail: Disjoint sets in O(α n) proven by Tarjan(1975)
/// Guarantee: struct FU {
struct FU {
	FU(int n):link(n,-1),rank(n,0){}
	int find(int i) const { return link[i] == -1 ? i : (link[i] = find(link[i])); }
	bool tryUnion(int a, int b) {
		a = find(a), b = find(b);
		if (a == b) return false;
		if (rank[a] < rank[b]) swap(a, b);
		if (rank[a] == rank[b]) ++rank[a];
		link[b] = a;
		return true;
	}
	mutable vector<int> link;
	vector<int> rank;
};
"#;
	let modified = SystemTime::now();
	let piece = Piece::parse(code, "__".to_owned(), modified).unwrap();
	assert_eq!(piece.name, "FU");
	assert_eq!(piece.description, Some("Find & Union".to_owned()));
	assert_eq!(piece.detail, Some("Disjoint sets in O(α n) proven by Tarjan(1975)".to_owned()));
	assert_eq!(
		piece.code,
		r#"struct FU {
	FU(int n):link(n,-1),rank(n,0){}
	int find(int i) const { return link[i] == -1 ? i : (link[i] = find(link[i])); }
	bool tryUnion(int a, int b) {
		a = find(a), b = find(b);
		if (a == b) return false;
		if (rank[a] < rank[b]) swap(a, b);
		if (rank[a] == rank[b]) ++rank[a];
		link[b] = a;
		return true;
	}
	mutable vector<int> link;
	vector<int> rank;
};"#
	);
	assert_eq!(piece.guarantee, "struct FU {");
	assert_eq!(piece.dependencies, Vec::<String>::new());
	assert_eq!(piece.parent, None);
	assert_eq!(piece.modified, modified);
}

#[test]
fn test_complex() {
	let code = r#"/// Name: DFS
/// Description: Depth First Search
/// Detail: An algorithm for traversing or searching tree or graph data structures with backtracking.
/// Guarantee: void DFS(
/// Dependencies: graph, dummyf, dfs-impl
/// Parent: graph
template <typename Pre=Dummyf, typename Post=Dummyf, typename PreE=Dummyf, typename PostE=Dummyf, typename FailE=Dummyf> void DFS(int source, Pre pre, Post post, PreE pree, PostE poste, FailE faile) const {
	auto visit = vector<bool>(size(), false);
	implDFS(source, visit, pre, post, pree, poste, faile);
}
"#;
	let modified = SystemTime::now();
	let piece = Piece::parse(code, "__".to_owned(), modified).unwrap();
	assert_eq!(piece.name, "DFS");
	assert_eq!(piece.description, Some("Depth First Search".to_owned()));
	assert_eq!(
		piece.detail,
		Some(
			"An algorithm for traversing or searching tree or graph data structures with \
			 backtracking."
				.to_owned()
		)
	);
	assert_eq!(
		piece.code,
		r#"template <typename Pre=Dummyf, typename Post=Dummyf, typename PreE=Dummyf, typename PostE=Dummyf, typename FailE=Dummyf> void DFS(int source, Pre pre, Post post, PreE pree, PostE poste, FailE faile) const {
	auto visit = vector<bool>(size(), false);
	implDFS(source, visit, pre, post, pree, poste, faile);
}"#
	);
	assert_eq!(piece.guarantee, "void DFS(");
	assert_eq!(piece.dependencies, &["graph", "dummyf", "dfs-impl"]);
	assert_eq!(piece.parent, Some("graph".to_owned()));
	assert_eq!(piece.modified, modified);
}
