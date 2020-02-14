use std::{
	cmp::min, fmt::{self, Write}, str::FromStr
};
use wasm_bindgen::JsValue;

const SPACE_CHARACTERS: &[char] = &['/', '-', '_', '+'];
const BLANK_CHARACTERS: &[char] = &[':', '(', ')', '[', ']', ',', '!', '\'', '"', '.', '#'];

pub trait VariableSet: FromStr<Err=String>+Clone+fmt::Display {
	type Map;
	fn expand(&self, map: &Self::Map) -> Option<String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Case {
	None,
	Camel,  // camelCase
	Pascal, // PascalCase
	Snake,  // snake_case
	Kebab,  // kebab-case
	Upper,  // UPPER_CASE
}

impl Case {
	fn apply(&self, s: &str) -> String {
		if let Case::None = self {
			return s.to_owned();
		}
		let parts: Vec<String> = s.split(' ').filter(|p| !p.is_empty()).map(String::from).collect();
		let (casing, cap_range, joiner): (fn(&str) -> String, _, _) = match self {
			Case::None => (str::to_owned, 0..0, ""),
			Case::Camel => (str::to_lowercase, 1..parts.len(), ""),
			Case::Pascal => (str::to_lowercase, 0..parts.len(), ""),
			Case::Snake => (str::to_lowercase, 0..0, "_"),
			Case::Kebab => (str::to_lowercase, 0..0, "-"),
			Case::Upper => (str::to_uppercase, 0..0, "_"),
		};
		parts
			.into_iter()
			.enumerate()
			.map(|(i, mut part)| {
				part = casing(&part);
				if cap_range.contains(&i) {
					part = capitalize(&part);
				}
				part
			})
			.collect::<Vec<_>>()
			.join(joiner)
	}
}

fn capitalize(s: &str) -> String {
	let mut cs = s.chars();
	let c1 = cs.next().map(|c1| c1.to_uppercase());
	c1.into_iter().flatten().chain(cs).collect()
}

impl fmt::Display for Case {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Case::None => Ok(()),
			Case::Camel => f.write_str(" case.camel"),
			Case::Pascal => f.write_str(" case.pascal"),
			Case::Snake => f.write_str(" case.snake"),
			Case::Kebab => f.write_str(" case.kebab"),
			Case::Upper => f.write_str(" case.upper"),
		}
	}
}

impl FromStr for Case {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"case.camel" => Ok(Case::Camel),
			"case.pascal" => Ok(Case::Pascal),
			"case.snake" => Ok(Case::Snake),
			"case.kebab" => Ok(Case::Kebab),
			"case.upper" => Ok(Case::Upper),
			_ => Err(format!(
				"unrecognized case type {:?}, choose one of: \"case.camel\", \"case.pascal\", \
				 \"case.snake\", \"case.kebab\", \"case.upper\"",
				s
			)),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Segment<V: VariableSet> {
	Literal(String),
	Substitution { variable: V, case: Case },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interpolation<V: VariableSet> {
	segments: Vec<Segment<V>>,
}

impl<V: VariableSet> fmt::Display for Interpolation<V> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		for segment in &self.segments {
			match segment {
				Segment::Literal(literal) => {
					for ch in literal.chars() {
						match ch {
							'{' => f.write_str("{{")?,
							'}' => f.write_str("}}")?,
							_ => f.write_char(ch)?,
						}
					}
				},
				Segment::Substitution { variable, case } => {
					f.write_char('{')?;
					fmt::Display::fmt(variable, f)?;
					fmt::Display::fmt(case, f)?;
					f.write_char('}')?;
				},
			}
		}
		Ok(())
	}
}

impl<V: VariableSet> Interpolation<V> {
	pub fn interpolate(&self, map: &V::Map) -> (String, bool) {
		let mut r = String::new();
		let mut all_good = true;
		for segment in &self.segments {
			match segment {
				Segment::Literal(literal) => r += literal,
				Segment::Substitution { variable, case } => {
					let mut variable = match variable.expand(map) {
						Some(v) => v,
						None => {
							all_good = false;
							String::new()
						},
					};
					for c in SPACE_CHARACTERS {
						variable = variable.replace(*c, " ");
					}
					for c in BLANK_CHARACTERS {
						variable = variable.replace(*c, "");
					}
					r += &case.apply(&variable);
				},
			}
		}
		(r, all_good)
	}
}

impl<V: VariableSet> FromStr for Interpolation<V> {
	type Err = String;

	fn from_str(mut s: &str) -> Result<Self, Self::Err> {
		let mut segments = Vec::new();
		while !s.is_empty() {
			if s.starts_with("{{") {
				segments.push(Segment::Literal("{".to_owned()));
				s = &s[2..];
			} else if s.starts_with("}}") {
				segments.push(Segment::Literal("}".to_owned()));
				s = &s[2..];
			} else if s.starts_with('{') {
				let i2 = s.find('}').ok_or("unterminated variable block")?;
				let inner = &s[1..i2];
				let mut parts = inner.split(' ');
				let variable = parts.next().ok_or("variable block has no content")?;
				let variable = variable.parse()?;
				let case = parts.next().map(|case| case.parse()).unwrap_or(Ok(Case::None))?;
				segments.push(Segment::Substitution { variable, case });
				s = &s[i2 + 1..];
			} else if s.starts_with('}') {
				return Err("variable block end } without a matching start {".to_owned());
			} else {
				let i = match (s.find('{'), s.find('}')) {
					(Some(i1), Some(i2)) => min(i1, i2),
					(Some(i), None) => i,
					(None, Some(i)) => i,
					(None, None) => s.len(),
				};
				segments.push(Segment::Literal(s[..i].to_owned()));
				s = &s[i..];
			}
		}
		Ok(Interpolation { segments })
	}
}

impl<V: VariableSet> evscode::marshal::Marshal for Interpolation<V> {
	fn to_js(&self) -> JsValue {
		self.to_string().into()
	}

	fn from_js(raw: JsValue) -> Result<Self, String> {
		<String as evscode::marshal::Marshal>::from_js(raw)?.parse()
	}
}

impl<V: VariableSet> evscode::Configurable for Interpolation<V> {
	fn to_json(&self) -> serde_json::Value {
		self.to_string().into()
	}

	fn schema(default: Option<&Self>) -> serde_json::Value {
		<String as evscode::Configurable>::schema(
			default.map(std::string::ToString::to_string).as_ref(),
		)
	}
}
