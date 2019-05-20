use crate::ci::test::Outcome;

#[derive(Clone, Debug)]
pub struct Row {
	pub number: i64,
	pub solution: Outcome,
	pub fitness: i64,
	pub input: String,
}
