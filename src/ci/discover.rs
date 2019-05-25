use crate::ci::test::Outcome;

#[derive(Debug)]
pub struct Row {
	pub number: usize,
	pub solution: Outcome,
	pub fitness: i64,
	pub input: String,
}
