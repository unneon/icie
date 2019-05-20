pub trait Fitness {
	fn evaluate(&self, input: &str) -> i64;
}

pub struct ByteLength;
impl Fitness for ByteLength {
	fn evaluate(&self, input: &str) -> i64 {
		-(input.len() as i64)
	}
}
