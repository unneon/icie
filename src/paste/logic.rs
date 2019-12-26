use crate::util::path::Path;
use async_trait::async_trait;
use evscode::{E, R};
use std::{collections::HashMap, time::SystemTime};

#[derive(Debug)]
pub struct Library {
	pub directory: Path,
	pub pieces: HashMap<String, Piece>,
}

#[derive(Debug, Clone)]
pub struct Piece {
	pub name: String,
	pub description: Option<String>,
	pub detail: Option<String>,
	pub code: String,
	pub guarantee: String,
	pub dependencies: Vec<String>,
	pub parent: Option<String>,
	pub modified: SystemTime,
}

impl Library {
	pub fn new_empty() -> Library {
		Library { directory: Path::from_native(String::new()), pieces: HashMap::new() }
	}

	pub fn verify(&self) -> R<()> {
		for (id, piece) in &self.pieces {
			if let Some(parent) = &piece.parent {
				if !self.pieces.contains_key(parent) {
					return Err(E::error(format!(
						"parent of {:?} is {:?}, which does not exist",
						id,
						Some(parent)
					))
					.context("malformed library"));
				}
				if self.pieces[parent].parent.is_some() {
					return Err(E::error("doubly nested library pieces are not supported yet")
						.context("malformed library"));
				}
			}
			for dep in &piece.dependencies {
				if !self.pieces.contains_key(dep) {
					return Err(E::error("dependency does not exist").context("malformed library"));
				}
			}
		}
		let (dg, t1, _) = self.build_dependency_graph();
		let og = self.build_ordering_graph(&dg, &t1);
		if og.toposort().is_none() {
			return Err(
				E::error("dependency/parenting cycle detected").context("malformed library")
			);
		}
		Ok(())
	}

	pub async fn walk_graph(&self, piece_id: &str, mut context: impl PasteContext) -> R<()> {
		let (dg, t1, t2) = self.build_dependency_graph();
		let og = self.build_ordering_graph(&dg, &t1);
		let mut missing = dg.vmasked_bfs(t1[piece_id], |v| !context.has(&t2[v]));
		let ord = og.toposort().unwrap();
		let mut pos = vec![og.len(); og.len()];
		for i in 0..ord.len() {
			pos[ord[i]] = i;
		}
		missing.sort_by_key(|v| pos[*v]);
		for v in missing {
			context.paste(t2[v]).await?;
		}
		Ok(())
	}

	/// Builds a graph where every piece has outgoings edges to all of its' dependencies.
	fn build_dependency_graph(&self) -> (Graph, HashMap<&str, usize>, Vec<&str>) {
		let t1: HashMap<&str, usize> =
			self.pieces.iter().enumerate().map(|(v, (id, _))| (id.as_str(), v)).collect();
		let mut t2 = t1.iter().collect::<Vec<_>>();
		t2.sort_by_key(|(_, v)| **v);
		let t2 = t2.into_iter().map(|(id, _)| *id).collect();
		let mut g = Graph::new(self.pieces.len());
		for (id, data) in &self.pieces {
			let v = t1[id.as_str()];
			for id2 in &data.dependencies {
				let u = t1[id2.as_str()];
				g.add_edge_1(v, u);
			}
		}
		(g, t1, t2)
	}

	/// Builds a graph where every piece has outgoing edges to all of its' (in)direct dependants.
	fn build_ordering_graph(&self, dg: &Graph, t1: &HashMap<&str, usize>) -> Graph {
		let mut og = dg.transpose();
		for data in self.pieces.values() {
			if let Some(parent) = &data.parent {
				let p = t1[parent.as_str()];
				for dep in &data.dependencies {
					let u = t1[dep.as_str()];
					if u != p && data.parent != self.pieces[dep].parent {
						og.add_edge_1(u, p);
					}
				}
			}
		}
		og
	}

	pub fn place(&self, piece_id: &str, source: &str) -> ((usize, usize), String) {
		let index = self.place_index(piece_id, source);
		let position = index_to_position(index, source);
		let (pref, suf) = if self.pieces[piece_id].parent.is_some() {
			("", "\n")
		} else {
			(
				if source[..index].ends_with("\n\n") { "" } else { "\n" },
				if source[index..].starts_with('\n') { "\n" } else { "\n\n" },
			)
		};
		let code = if self.pieces[piece_id].parent.is_some() {
			let mut buf = String::new();
			for line in self.pieces[piece_id].code.lines() {
				buf += "\t";
				buf += line;
				buf += "\n";
			}
			buf.trim_end().to_owned()
		} else {
			self.pieces[piece_id].code.trim_end().to_owned()
		};
		(position, format!("{}{}{}", pref, code, suf))
	}

	fn place_index(&self, piece_id: &str, source: &str) -> usize {
		let piece = &self.pieces[piece_id];
		if let Some(parent) = &piece.parent {
			// the piece will be placed at the end of the struct definition
			// while c++ allows declarations in structs to be out of order, it doesn't do so always
			// specifically, templates and nested structs seem to break it for some reason
			let parent = &self.pieces[parent];
			let mut pos = source.find(&parent.guarantee).unwrap();
			pos += source[pos..].find("\n}").unwrap();
			pos += 1;
			pos
		} else {
			let (dg, t1, t2) = self.build_dependency_graph();
			let og = self.build_ordering_graph(&dg, &t1);
			let og2 = og.transpose();
			let mut pos = source.find("using namespace std").map(|i| i + 1).unwrap_or(0);
			for v in &og2.edges[t1[piece_id]] {
				let dep_id = t2[*v];
				let dep = &self.pieces[dep_id];
				pos += source[pos..].find(&dep.guarantee).map(|i| i + 1).unwrap_or(0);
			}
			pos = skip_to_toplevel(pos, source);
			if pos > source.len() && !source.ends_with('\n') {
				pos -= 1;
			}
			pos
		}
	}
}

fn index_to_position(index: usize, source: &str) -> (usize, usize) {
	let line = source[..index].chars().filter(|c| *c == '\n').count();
	(line, 0)
}

fn skip_to_toplevel(mut pos: usize, source: &str) -> usize {
	loop {
		pos += source[pos..].find('\n').unwrap_or_else(|| source.len());
		if source[pos..].starts_with("\n}") {
			pos += 1;
			pos += source[pos..].find('\n').unwrap_or_else(|| source[pos..].len());
			break pos + 1;
		} else if source[pos..].starts_with("\n\n")
			|| source[pos..].starts_with("\n ")
			|| source[pos..].starts_with("\n\t")
		{
			pos += 1;
		} else {
			break pos + 1;
		}
	}
}

#[async_trait(?Send)]
pub trait PasteContext {
	fn has(&mut self, piece: &str) -> bool;
	async fn paste(&mut self, piece: &str) -> R<()>;
}

struct Graph {
	edges: Vec<Vec<usize>>,
}
impl Graph {
	fn new(n: usize) -> Graph {
		Graph { edges: vec![vec![]; n] }
	}

	fn add_edge_1(&mut self, v: usize, u: usize) {
		self.edges[v].push(u);
	}

	fn len(&self) -> usize {
		self.edges.len()
	}

	fn toposort(&self) -> Option<Vec<usize>> {
		let mut deg = vec![0; self.len()];
		for v in 0..self.len() {
			for u in &self.edges[v] {
				deg[*u] += 1;
			}
		}
		let mut que = Vec::new();
		for (v, d) in deg.iter().enumerate() {
			if *d == 0 {
				que.push(v);
			}
		}
		for i in 0.. {
			if i >= que.len() {
				break;
			}
			let v = que[i];
			for u in &self.edges[v] {
				deg[*u] -= 1;
				if deg[*u] == 0 {
					que.push(*u);
				}
			}
		}
		if que.len() == self.len() { Some(que) } else { None }
	}

	fn vmasked_bfs(&self, source: usize, mut vmask: impl FnMut(usize) -> bool) -> Vec<usize> {
		let mut visit = vec![false; self.len()];
		let mut que = Vec::new();
		visit[source] = true;
		que.push(source);
		for i in 0.. {
			if i >= que.len() {
				break;
			}
			let v = que[i];
			if vmask(v) {
				for u in &self.edges[v] {
					if !visit[*u] {
						visit[*u] = true;
						que.push(*u);
					}
				}
			} else {
				que[i] = self.len();
			}
		}
		que.into_iter().filter(|v| *v != self.len()).collect()
	}

	fn transpose(&self) -> Graph {
		let mut g = Graph::new(self.len());
		for v in 0..self.len() {
			for u in &self.edges[v] {
				g.add_edge_1(*u, v);
			}
		}
		g
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn dependency_order() {
		let lib = example_library();
		let orders = paste_iter(&lib, "dfs").await;
		assert!(
			(orders[0] == "dummyf" && orders[1] == "graph")
				|| (orders[0] == "graph" && orders[1] == "dummyf")
		);
		assert_eq!(orders[2], "dfs-impl");
		assert_eq!(orders[3], "dfs");
		assert_eq!(orders.len(), 4);
	}

	#[test]
	fn nonexistent_parent() {
		let mut lib = Library::new_empty();
		lib.pieces.insert("b".to_owned(), mock_piece("b", &["a"], Some("a")));
		assert!(lib.verify().is_err());
	}

	#[test]
	fn nonexisting_dependency() {
		let mut lib = Library::new_empty();
		lib.pieces.insert("b".to_owned(), mock_piece("b", &["a"], None));
		assert!(lib.verify().is_err());
	}

	#[test]
	fn doubly_nested() {
		let mut lib = Library::new_empty();
		lib.pieces.insert("a".to_owned(), mock_piece("a", &[], None));
		lib.pieces.insert("b".to_owned(), mock_piece("b", &["a"], Some("a")));
		assert!(lib.verify().is_ok());
		lib.pieces.insert("c".to_owned(), mock_piece("c", &["b", "a"], Some("b")));
		assert!(lib.verify().is_err());
	}

	#[test]
	fn dependency_cycle() {
		let mut lib = Library::new_empty();
		lib.pieces.insert("a".to_owned(), mock_piece("a", &["b"], None));
		lib.pieces.insert("b".to_owned(), mock_piece("b", &["c"], None));
		lib.pieces.insert("c".to_owned(), mock_piece("c", &["a"], None));
		assert!(lib.verify().is_err());
	}

	#[tokio::test]
	async fn placing_basic() {
		let lib = linear_library();
		assert_eq!(
			replace(
				&lib,
				"ntt",
				r#"#include <bits/stdc++.h>
using namespace std;

int main() {

}
"#
			)
			.await,
			r#"#include <bits/stdc++.h>
using namespace std;

{{qpow}}

{{mint}}

{{ntt}}

int main() {

}
"#
		);
	}

	#[tokio::test]
	async fn placing_partial() {
		let lib = linear_library();
		assert_eq!(
			replace(
				&lib,
				"ntt",
				r#"#include <bits/stdc++.h>
using namespace std;

{{qpow}}

int main() {

}
"#
			)
			.await,
			r#"#include <bits/stdc++.h>
using namespace std;

{{qpow}}

{{mint}}

{{ntt}}

int main() {

}
"#
		);
	}

	#[tokio::test]
	async fn in_group_ordering() {
		let mut lib1 = Library::new_empty();
		lib1.pieces.insert("graph".to_owned(), Piece {
			name: "Graph".to_owned(),
			description: None,
			detail: None,
			code: "struct Graph {\n};".to_owned(),
			guarantee: "struct Graph {".to_owned(),
			dependencies: Vec::new(),
			parent: None,
			modified: SystemTime::now(),
		});
		lib1.pieces.insert("lca".to_owned(), mock_piece("lca", &["graph"], Some("graph")));
		lib1.pieces.insert(
			"dominator".to_owned(),
			mock_piece("dominator", &["graph", "lca"], Some("graph")),
		);
		let mut lib2 = Library::new_empty();
		lib2.pieces.insert("graph".to_owned(), lib1.pieces.get("graph").unwrap().clone());
		lib2.pieces.insert("dominator".to_owned(), lib1.pieces.get("dominator").unwrap().clone());
		lib2.pieces.insert("lca".to_owned(), lib1.pieces.get("lca").unwrap().clone());
		let desired = r#"
struct Graph {
	{{lca}}
	{{dominator}}
};

"#;
		assert_eq!(replace(&lib1, "dominator", "").await, desired);
		assert_eq!(replace(&lib2, "dominator", "").await, desired);
	}

	async fn replace(lib: &Library, piece: &str, code: &str) -> String {
		let mut buf = code.to_owned();
		lib.walk_graph(piece, PlaceMockContext { lib: &lib, buf: &mut buf }).await.unwrap();
		buf
	}

	struct PlaceMockContext<'a> {
		lib: &'a Library,
		buf: &'a mut String,
	}

	#[async_trait]
	impl PasteContext for PlaceMockContext<'_> {
		fn has(&mut self, piece: &str) -> bool {
			self.buf.contains(&self.lib.pieces[piece].guarantee)
		}

		async fn paste(&mut self, piece: &str) -> R<()> {
			let ((line, column), snippet) = self.lib.place(piece, &self.buf);
			*self.buf = self
				.buf
				.split('\n')
				.enumerate()
				.map(|(i, row)| {
					if i == line {
						format!("{}{}{}", &row[..column], snippet, &row[column..])
					} else {
						row.to_owned()
					}
				})
				.collect::<Vec<_>>()
				.join("\n");
			Ok(())
		}
	}

	async fn paste_iter(lib: &Library, piece_id: &str) -> Vec<String> {
		let mut buf = Vec::new();
		let ctx = MockContext { buf: &mut buf };
		lib.walk_graph(piece_id, ctx).await.unwrap();
		buf
	}

	struct MockContext<'a> {
		buf: &'a mut Vec<String>,
	}

	#[async_trait]
	impl PasteContext for MockContext<'_> {
		fn has(&mut self, piece_id: &str) -> bool {
			self.buf.contains(&piece_id.to_owned())
		}

		async fn paste(&mut self, piece_id: &str) -> R<()> {
			self.buf.push(piece_id.to_owned());
			Ok(())
		}
	}

	fn mock_piece(id: &str, dependencies: &[&str], parent: Option<&str>) -> Piece {
		Piece {
			name: id.to_owned(),
			description: None,
			detail: None,
			code: format!("{{{{{}}}}}", id),
			guarantee: format!("{{{{{}}}}}", id),
			dependencies: dependencies.iter().map(|s| (*s).to_owned()).collect(),
			parent: parent.map(|s| s.to_owned()),
			modified: SystemTime::now(),
		}
	}

	fn example_library() -> Library {
		let mut lib = Library::new_empty();
		lib.pieces.insert("dummyf".to_owned(), mock_piece("dummyf", &[], None));
		lib.pieces.insert("graph".to_owned(), mock_piece("graph", &[], None));
		lib.pieces.insert(
			"dfs".to_owned(),
			mock_piece("dfs", &["graph", "dfs-impl", "dummyf"], Some("graph")),
		);
		lib.pieces.insert(
			"dfs-impl".to_owned(),
			mock_piece("dfs-impl", &["graph", "dummyf"], Some("graph")),
		);
		lib.verify().unwrap();
		lib
	}

	fn linear_library() -> Library {
		let mut lib = Library::new_empty();
		lib.pieces.insert("qpow".to_owned(), mock_piece("qpow", &[], None));
		lib.pieces.insert("mint".to_owned(), mock_piece("mint", &["qpow"], None));
		lib.pieces.insert("ntt".to_owned(), mock_piece("ntt", &["mint"], None));
		lib.verify().unwrap();
		lib
	}
}
