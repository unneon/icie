//! Conversion traits between Rust and JavaScript types.

use json::{JsonValue, Null};
use std::{collections::HashMap, path::PathBuf};

/// Trait responsible for converting values between Rust and JavaScript.
pub trait Marshal: Sized {
	/// Convert a Rust value to JavaScript.
	/// This conversion must not fail - if it can, consider using Rust's type system to enforce that condition.
	fn to_json(&self) -> JsonValue;
	/// Convert a JavaScript value to a Rust value.
	fn from_json(raw: JsonValue) -> Result<Self, String>
	where
		Self: std::marker::Sized;
}

macro_rules! impl_number {
	($t:ty, $method:ident) => {
		impl Marshal for $t {
			fn to_json(&self) -> JsonValue {
				json::from(*self)
			}

			fn from_json(raw: JsonValue) -> Result<Self, String> {
				raw.$method().ok_or_else(|| type_error(stringify!($t), &raw))
			}
		}
	};
}

impl_number!(i8, as_i8);
impl_number!(i16, as_i16);
impl_number!(i32, as_i32);
impl_number!(i64, as_i64);
impl_number!(isize, as_isize);
impl_number!(u8, as_u8);
impl_number!(u16, as_u16);
impl_number!(u32, as_u32);
impl_number!(u64, as_u64);
impl_number!(usize, as_usize);
impl_number!(f32, as_f32);
impl_number!(f64, as_f64);
impl Marshal for bool {
	fn to_json(&self) -> JsonValue {
		JsonValue::from(*self)
	}

	fn from_json(raw: JsonValue) -> Result<Self, String> {
		raw.as_bool().ok_or_else(|| type_error("bool", &raw))
	}
}
impl Marshal for String {
	fn to_json(&self) -> JsonValue {
		json::from(self.as_str())
	}

	fn from_json(mut raw: JsonValue) -> Result<Self, String> {
		raw.take_string().ok_or_else(|| type_error("string", &raw))
	}
}
impl Marshal for PathBuf {
	fn to_json(&self) -> JsonValue {
		json::from(self.to_str().unwrap())
	}

	fn from_json(raw: JsonValue) -> Result<Self, String> {
		Ok(PathBuf::from(&*shellexpand::tilde(&String::from_json(raw)?)))
	}
}
impl<T: Marshal> Marshal for Option<T> {
	fn to_json(&self) -> JsonValue {
		self.as_ref().map_or(Null, Marshal::to_json)
	}

	fn from_json(raw: JsonValue) -> Result<Self, String> {
		if raw.is_null() { Ok(None) } else { Ok(Some(T::from_json(raw)?)) }
	}
}
impl<T: Marshal> Marshal for Vec<T> {
	fn to_json(&self) -> JsonValue {
		JsonValue::Array(self.iter().map(Marshal::to_json).collect())
	}

	fn from_json(mut raw: JsonValue) -> Result<Self, String> {
		if raw.is_array() {
			Ok(raw.members_mut().map(|x| T::from_json(x.take())).collect::<Result<Self, String>>()?)
		} else {
			Err(type_error("array", &raw))
		}
	}
}
impl<T: Marshal, S: std::hash::BuildHasher+Default> Marshal for HashMap<String, T, S> {
	fn to_json(&self) -> JsonValue {
		let mut obj = json::object::Object::with_capacity(self.len());
		for (k, v) in self {
			obj.insert(k, v.to_json());
		}
		JsonValue::Object(obj)
	}

	fn from_json(mut raw: JsonValue) -> Result<Self, String> {
		if raw.is_object() {
			Ok(raw.entries_mut().map(|(k, v)| Ok((k.to_owned(), T::from_json(v.take())?))).collect::<Result<Self, String>>()?)
		} else {
			Err(type_error("object", &raw))
		}
	}
}

fn type_error(expected: &'static str, raw: &JsonValue) -> String {
	format!("expected {}, found `{}`", expected, json_type(raw))
}
fn json_type(raw: &JsonValue) -> &'static str {
	match raw {
		JsonValue::Null => "null",
		JsonValue::Short(_) => "string",
		JsonValue::String(_) => "string",
		JsonValue::Number(_) => "number",
		JsonValue::Boolean(_) => "boolean",
		JsonValue::Object(_) => "object",
		JsonValue::Array(_) => "array",
	}
}
