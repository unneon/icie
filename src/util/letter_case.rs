const SPACE_CHARACTERS: &[char] = &['/', '-', '_', '+'];
const BLANK_CHARACTERS: &[char] = &[':', '(', ')', '[', ']', ',', '!', '\'', '"', '.', '#'];

#[derive(Debug)]
pub enum Case {
	Kebab, // kebab-case
	Upper, // UPPER_CASE
}

impl Case {
	pub fn apply(&self, text: &str) -> String {
		let mut text = text.to_owned();
		for c in SPACE_CHARACTERS {
			text = text.replace(*c, " ");
		}
		for c in BLANK_CHARACTERS {
			text = text.replace(*c, "");
		}
		let (casing, joiner): (fn(&str) -> String, _) = match self {
			Case::Kebab => (str::to_lowercase, "-"),
			Case::Upper => (str::to_uppercase, "_"),
		};
		text.split(' ').filter(|p| !p.is_empty()).map(casing).collect::<Vec<_>>().join(joiner)
	}
}
