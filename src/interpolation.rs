use json::JsonValue;
use std::{
	cmp::min, fmt::{self, Write}, str::FromStr
};

pub trait VariableSet: FromStr<Err=String>+fmt::Display {
	type Map;
	fn expand(&self, map: &Self::Map) -> Option<String>;
}

enum Case {
	None,
	CamelCase,  // camelCase
	PascalCase, // PascalCase
	SnakeCase,  // snake_case
	KebabCase,  // kebab-case
	UpperCase,  // UPPER_CASE
}

impl Case {
	fn apply(&self, s: &str) -> String {
		if let Case::None = self {
			return s.to_owned();
		}
		let mut parts: Vec<String> = s.split(' ').filter(|p| !p.is_empty()).map(String::from).collect();
		let casing = match self {
			Case::None => (|s: &str| s.to_owned()) as fn(&str) -> String,
			Case::CamelCase | Case::PascalCase | Case::KebabCase | Case::SnakeCase => (|s: &str| s.to_lowercase()),
			Case::UpperCase => |s: &str| s.to_uppercase(),
		};
		for part in &mut parts {
			*part = casing(&*part);
		}
		match self {
			Case::CamelCase => {
				for i in 1..parts.len() {
					parts[i] = capitalize(&parts[i]);
				}
			},
			Case::PascalCase => {
				for part in &mut parts {
					*part = capitalize(&*part);
				}
			},
			Case::None | Case::SnakeCase | Case::KebabCase | Case::UpperCase => (),
		}
		let joiner = match self {
			Case::None | Case::CamelCase | Case::PascalCase => "",
			Case::SnakeCase | Case::UpperCase => "_",
			Case::KebabCase => "-",
		};
		parts.join(joiner)
	}
}

fn capitalize(s: &str) -> String {
	let mut cs = s.chars();
	let c1 = cs.next().map(|c1| c1.to_uppercase().into_iter());
	c1.into_iter().flatten().chain(cs).collect()
}

impl fmt::Display for Case {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Case::None => Ok(()),
			Case::CamelCase => f.write_str(" case.camel"),
			Case::PascalCase => f.write_str(" case.pascal"),
			Case::SnakeCase => f.write_str(" case.snake"),
			Case::KebabCase => f.write_str(" case.kebab"),
			Case::UpperCase => f.write_str(" case.upper"),
		}
	}
}

impl FromStr for Case {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"case.camel" => Ok(Case::CamelCase),
			"case.pascal" => Ok(Case::PascalCase),
			"case.snake" => Ok(Case::SnakeCase),
			"case.kebab" => Ok(Case::KebabCase),
			"case.upper" => Ok(Case::UpperCase),
			_ => Err(format!(
				"unrecognized case type {:?}, choose one of: \"case.camel\", \"case.pascal\", \"case.snake\", \"case.kebab\", \"case.upper\"",
				s
			)),
		}
	}
}

enum Segment<V: VariableSet> {
	Literal(String),
	Substitution { variable: V, case: Case },
}

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
					let variable = match variable.expand(map) {
						Some(v) => v,
						None => {
							all_good = false;
							String::new()
						},
					};
					;
					r += &case.apply(&variable.replace('/', " ").replace('-', " ").replace('_', " "));
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
			} else if s.starts_with("{") {
				let i2 = s.find("}").ok_or("unterminated variable block")?;
				let inner = &s[1..i2];
				let mut parts = inner.split(' ');
				let variable = parts.next().ok_or("variable block has no content")?;
				let variable = variable.parse()?;
				let case = parts.next().map(|case| case.parse()).unwrap_or(Ok(Case::None))?;
				segments.push(Segment::Substitution { variable, case });
				s = &s[i2 + 1..];
			} else if s.starts_with("}") {
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
	fn to_json(&self) -> JsonValue {
		self.to_string().to_json()
	}

	fn from_json(raw: JsonValue) -> Result<Self, String> {
		<String as evscode::marshal::Marshal>::from_json(raw)?.parse()
	}
}

impl<V: VariableSet> evscode::Configurable for Interpolation<V> {
	fn schema(description: Option<&str>, default: Option<&Self>) -> JsonValue {
		<String as evscode::Configurable>::schema(description, default.map(std::string::ToString::to_string).as_ref())
	}
}
