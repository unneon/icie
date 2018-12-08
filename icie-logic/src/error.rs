#[derive(Debug)]
pub enum Error {
	ManualDescription(String),
}
impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
		write!(f, "{:?}", self)
	}
}
impl std::error::Error for Error {}

pub type R<T> = Result<T, Error>;

macro_rules! er {
	($args:tt) => {
		return Err(crate::error::Error::ManualDescription(format!($args)));
	};
}
#[allow(unused)]
macro_rules! eo {
	($args:tt) => {
		crate::error::Error::ManualDescription(format!($args))
	};
}
