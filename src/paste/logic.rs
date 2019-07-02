use evscode::{E, R};
use std::{collections::HashMap, path::PathBuf, time::SystemTime};

#[derive(Debug)]
pub struct Library {
	pub directory: PathBuf,
	pub pieces: HashMap<String, Piece>,
}

#[derive(Debug)]
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
		Library {
			directory: PathBuf::new(),
			pieces: HashMap::new(),
		}
	}

	pub fn verify(&self) -> R<()> {
		for (id, piece) in &self.pieces {
			if let Some(parent) = &piece.parent {
				if !self.pieces.contains_key(parent) {
					return Err(E::error(format!("parent of {:?} is {:?}, which does not exist", id, Some(parent))).context("malformed library"));
				}
				if self.pieces[parent].parent.is_some() {
					return Err(E::error("doubly nested library pieces are not supported yet").context("malformed library"));
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
			return Err(E::error("dependency/parenting cycle detected").context("malformed library"));
		}
		Ok(())
	}

	pub fn walk_graph(&self, piece_id: &str, mut context: impl PasteContext) -> R<()> {
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
			context.paste(t2[v])?;
		}
		Ok(())
	}

	fn build_dependency_graph(&self) -> (Graph, HashMap<&str, usize>, Vec<&str>) {
		let t1: HashMap<&str, usize> = self.pieces.iter().enumerate().map(|(v, (id, _))| (id.as_str(), v)).collect();
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

	pub fn place(&self, piece_id: &str, source: &str) -> R<((usize, usize), String)> {
		let index = self.place_index(piece_id, source)?;
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
		Ok((position, format!("{}{}{}", pref, code, suf)))
	}

	fn place_index(&self, piece_id: &str, source: &str) -> R<usize> {
		let piece = &self.pieces[piece_id];
		if let Some(parent) = &piece.parent {
			let parent = &self.pieces[parent];
			let mut pos = source.find(&parent.guarantee).unwrap();
			pos += source[pos..].find('{').unwrap();
			pos += source[pos..].find('\n').unwrap();
			pos += 1;
			Ok(pos)
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
			Ok(pos)
		}
	}
}

fn index_to_position(index: usize, source: &str) -> (usize, usize) {
	let line = source[..index].chars().filter(|c| *c == '\n').count();
	(line, 0)
}

fn skip_to_toplevel(mut pos: usize, source: &str) -> usize {
	loop {
		pos += source[pos..].find('\n').unwrap_or(source.len());
		if source[pos..].starts_with("\n}") {
			pos += 1;
			pos += source[pos..].find('\n').unwrap_or_else(|| source[pos..].len());
			break pos + 1;
		} else if source[pos..].starts_with("\n\n") || source[pos..].starts_with("\n ") || source[pos..].starts_with("\n\t") {
			pos += 1;
		} else {
			break pos + 1;
		}
	}
}

pub trait PasteContext {
	fn has(&mut self, piece: &str) -> bool;
	fn paste(&mut self, piece: &str) -> R<()>;
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
		if que.len() == self.len() {
			Some(que)
		} else {
			None
		}
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

	#[test]
	fn dependency_order() {
		let lib = example_library();
		let orders = paste_iter(&lib, "dfs");
		assert!((orders[0] == "dummyf" && orders[1] == "graph") || (orders[0] == "graph" && orders[1] == "dummyf"));
		assert_eq!(orders[2], "dfs-impl");
		assert_eq!(orders[3], "dfs");
		assert_eq!(orders.len(), 4);
	}

	fn paste_iter(lib: &Library, piece_id: &str) -> Vec<String> {
		let mut buf = Vec::new();
		let ctx = MockContext { buf: &mut buf };
		lib.walk_graph(piece_id, ctx).unwrap();
		buf
	}

	struct MockContext<'a> {
		buf: &'a mut Vec<String>,
	}
	impl PasteContext for MockContext<'_> {
		fn has(&mut self, piece_id: &str) -> bool {
			self.buf.contains(&piece_id.to_owned())
		}

		fn paste(&mut self, piece_id: &str) -> Result<(), E> {
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
		lib.pieces["dummyf"] = mock_piece("dummyf", &[], None);
		lib.pieces["graph"] = mock_piece("graph", &[], None);
		lib.pieces["dfs"] = mock_piece("dfs", &["graph", "dfs-impl", "dummyf"], Some("graph"));
		lib.pieces["dfs-impl"] = mock_piece("dfs-impl", &["graph", "dummyf"], Some("graph"));
		lib
	}
}
