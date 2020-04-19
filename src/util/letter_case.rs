#[derive(Debug)]
pub enum Case {
	Kebab, // kebab-case
	Upper, // UPPER_CASE
}

impl Case {
	pub fn apply(&self, text: &str) -> String {
		let (casing, joiner): (fn(&str) -> String, _) = match self {
			Case::Kebab => (str::to_lowercase, "-"),
			Case::Upper => (str::to_uppercase, "_"),
		};
		let words = text.split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty());
		words.map(casing).collect::<Vec<_>>().join(joiner)
	}
}
