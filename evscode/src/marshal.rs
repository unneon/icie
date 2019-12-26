//! Conversion traits between Rust and JavaScript types.

use std::{collections::HashMap, fmt};
use wasm_bindgen::{prelude::*, JsCast};

/// Trait responsible for converting values between Rust and JavaScript.
pub trait Marshal: Sized {
	/// Convert a Rust value to a JavaScript value.
	/// This conversion must not fail - if it can, consider using Rust's type system to enforce that
	/// condition.
	fn to_js(&self) -> JsValue;
	/// Convert a JavaScript value to a Rust value.
	fn from_js(raw: JsValue) -> Result<Self, String>;
}

macro_rules! impl_number {
	($t:ty, $method:ident) => {
		impl Marshal for $t {
			fn to_js(self: &Self) -> JsValue {
				JsValue::from_f64(*self as f64)
			}

			fn from_js(raw: JsValue) -> Result<Self, String> {
				raw.as_f64().ok_or_else(|| type_error2(stringify!($t), &raw)).map(|f| f as $t)
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
	fn to_js(&self) -> JsValue {
		JsValue::from_bool(*self)
	}

	fn from_js(raw: JsValue) -> Result<Self, String> {
		raw.as_bool().ok_or_else(|| type_error2("bool", &raw))
	}
}
impl Marshal for String {
	fn to_js(&self) -> JsValue {
		self.into()
	}

	fn from_js(raw: JsValue) -> Result<Self, String> {
		raw.as_string().ok_or_else(|| type_error2("string", &raw))
	}
}
impl<T: Marshal> Marshal for Option<T> {
	fn to_js(&self) -> JsValue {
		self.as_ref().map_or(JsValue::undefined(), T::to_js)
	}

	fn from_js(raw: JsValue) -> Result<Self, String> {
		if raw.is_undefined() || raw.is_null() { Ok(None) } else { Ok(Some(T::from_js(raw)?)) }
	}
}
impl<T: Marshal> Marshal for Vec<T> {
	fn to_js(&self) -> JsValue {
		let arr = js_sys::Array::new();
		for value in self {
			arr.push(&value.to_js());
		}
		JsValue::from(arr)
	}

	fn from_js(raw: JsValue) -> Result<Self, String> {
		match raw.dyn_into::<js_sys::Array>() {
			Ok(raw) => Ok(raw
				.values()
				.into_iter()
				.map(|raw| T::from_js(raw.unwrap()))
				.collect::<Result<_, _>>()?),
			Err(raw) => Err(type_error2("array", &raw)),
		}
	}
}
impl<T: Marshal, S: std::hash::BuildHasher+Default> Marshal for HashMap<String, T, S> {
	fn to_js(&self) -> JsValue {
		let obj = js_sys::Object::new();
		for (key, value) in self {
			js_sys::Reflect::set(&obj, &JsValue::from_str(&key), &value.to_js()).unwrap();
		}
		obj.into()
	}

	fn from_js(raw: JsValue) -> Result<Self, String> {
		let obj = raw.dyn_into::<js_sys::Object>().expect("not a js_sys.Object");
		Ok(js_sys::Object::entries(&obj)
			.values()
			.into_iter()
			.map(|kv| {
				let kv: Vec<_> = kv
					.expect("object iteration failed")
					.dyn_into::<js_sys::Array>()
					.unwrap()
					.values()
					.into_iter()
					.map(Result::unwrap)
					.collect();
				match kv.as_slice() {
					[key, value] => {
						let key = key.as_string().unwrap();
						let value = T::from_js(value.clone()).unwrap();
						(key, value)
					},
					_ => unreachable!(),
				}
			})
			.collect())
	}
}

/// Returns a string describing a type error when casting from JS.
pub fn type_error2(expected: &'static str, raw: &JsValue) -> String {
	format!("expected {}, found `{:?}`", expected, raw)
}

pub(crate) fn camel_case(s: &str, f: &mut fmt::Formatter) -> fmt::Result {
	let mut rest = s.split('_');
	let lead = rest.next();
	if let Some(lead) = lead {
		for c in lead.chars() {
			write!(f, "{}", c.to_lowercase())?;
		}
		for word in rest {
			let mut rest = word.chars();
			let lead = rest.next();
			if let Some(lead) = lead {
				write!(f, "{}", lead.to_uppercase())?;
			}
			for c in rest {
				write!(f, "{}", c.to_lowercase())?;
			}
		}
	}
	Ok(())
}
