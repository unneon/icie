//! Configuration system based on VS Code API, with strong typing and auto-reload built in.
//!
//! To add a configuration entry, write this in global scope of any file of an extension:
//! ```ignore
//! /// Fooification time limit, expressed in milliseconds
//! #[evscode::config]
//! static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);
//! ```
//! The entry will be automatically added to the extension manifest and registered for updates.
//! To read the values, first call [`Config::get`] to get an [`std::sync::Arc`] with the current
//! value. The [`Config`] will receive configuration updates and return up-to-date values, but the
//! returned pointer will not.
//!
//! The system supports all types that implement the [`Configurable`](trait.Configurable.html)
//! trait. If the conversion provided via the [`Marshal`](../marshal/trait.Marshal.html) trait
//! fails, the default value will be used.

use crate::{marshal::Marshal, meta::Identifier};
use std::collections::HashMap;
use wasm_bindgen::JsValue;

macro_rules! optobject_impl {
	($obj:ident, ) => {};
	($obj:ident, optional $key:expr => $value:expr, $($rest:tt)*) => {
		if let Some(value) = $value {
			$obj[$key] = serde_json::Value::from(value);
		}
		optobject_impl!($obj, $($rest)*);
	};
	($obj:ident, $key:expr => $value:expr, $($rest:tt)*) => {
		$obj[$key] = serde_json::Value::from($value);
		optobject_impl!($obj, $($rest)*);
	};
}
macro_rules! optobject {
	{ $($token:tt)* } => {
		let mut obj = serde_json::Value::Object(Default::default());
		optobject_impl!(obj, $($token)*);
		obj
	};
}

/// Wrapper object for an automatically updated configuration entry.
///
/// To get the current value, call the [`Config::get`] method which will return an
/// [`std::sync::Arc`]. Do not store the Arc for extended periods of time, so that your extension is
/// responsive to configuration updates.
#[derive(Debug)]
pub struct Config<T: Configurable> {
	id: Identifier,
	default: T,
}
impl<T: Configurable> Config<T> {
	#[doc(hidden)]
	pub fn placeholder(default: T, id: Identifier) -> Config<T> {
		Config { id, default }
	}

	/// Return a reference-counted pointer to the current configuration values.
	pub fn get(&self) -> T {
		let mut tree = vscode_sys::workspace::get_configuration(&self.id.extension_id());
		for key in self.id.inner_path().split('.') {
			tree = js_sys::Reflect::get(&tree, &JsValue::from_str(key)).unwrap();
		}
		match T::from_js(tree) {
			Ok(value) => value,
			Err(_) => self.default.clone(),
		}
	}
}

/// A trait that allows a type to be used as a configuration values.
///
/// This trait can be [automatically derived](../../evscode_codegen/derive.Configurable.html) for
/// enums that hold no data. ```ignore
/// #[derive(evscode::Configurable)]
/// enum AnimalBackend {
///     #[evscode(name = "Doggo")]
///     Dog,
///     #[evscode(name = "Kitty")]
///     Cat,
/// }
/// ```
/// There does not exist a simple way to implement it for any custom types, because the VS Code
/// [documentation of config API][1] is lacking.
///
/// [1]: (https://code.visualstudio.com/api/references/contribution-points#contributes.configuration)
pub trait Configurable: Marshal+Clone {
	#[doc(hidden)]
	fn to_json(&self) -> serde_json::Value;
	#[doc(hidden)]
	fn schema(default: Option<&Self>) -> serde_json::Value;
}

macro_rules! simple_configurable {
	($rust:ty, $json:expr) => {
		impl Configurable for $rust {
			fn to_json(&self) -> serde_json::Value {
				self.clone().into()
			}

			fn schema(default: Option<&Self>) -> serde_json::Value {
				optobject! {
					"type" => $json,
					optional "default" => default.map(Self::to_json),
				}
			}
		}
	};
}

simple_configurable!(bool, "boolean");
simple_configurable!(String, "string");
simple_configurable!(i8, "number");
simple_configurable!(i16, "number");
simple_configurable!(i32, "number");
simple_configurable!(i64, "number");
simple_configurable!(isize, "number");
simple_configurable!(u8, "number");
simple_configurable!(u16, "number");
simple_configurable!(u32, "number");
simple_configurable!(u64, "number");
simple_configurable!(usize, "number");

impl<T: Configurable> Configurable for Option<T> {
	fn to_json(&self) -> serde_json::Value {
		self.as_ref().map_or(serde_json::Value::Null, T::to_json)
	}

	fn schema(default: Option<&Option<T>>) -> serde_json::Value {
		let mut obj = T::schema(default.and_then(|default| default.as_ref()));
		if let serde_json::Value::Array(type_array) = &mut obj["type"] {
			type_array.push(serde_json::Value::Null);
		} else {
			obj["type"] = serde_json::json!(["null", obj["type"].as_str().unwrap()]);
		}
		if let Some(None) = default {
			obj["default"] = serde_json::Value::Null;
		}
		obj
	}
}
/// This implementation is not editable in VS Code setting UI.
/// I am not sure why, because VS Code has builtin configuration entries that have the same manifest
/// entry, but are editable. Naturally, the [documentation](https://code.visualstudio.com/api/references/contribution-points#contributes.configuration) of this behaviour does not exist.
impl<T: Configurable, S: std::hash::BuildHasher+Default+Clone> Configurable
	for HashMap<String, T, S>
{
	fn to_json(&self) -> serde_json::Value {
		serde_json::Value::Object(
			self.iter().map(|(key, value)| (key.clone(), value.to_json())).collect(),
		)
	}

	fn schema(default: Option<&Self>) -> serde_json::Value {
		optobject! {
			"type" => "object",
			optional "default" => default.map(Self::to_json),
			"additionalProperties" => serde_json::json!({
				"anyOf": serde_json::json!([T::schema(None)])
			}),
		}
	}
}

#[doc(hidden)]
pub trait ErasedConfig: Send+Sync {
	fn is_default(&self) -> bool;
}
impl<T: PartialEq+Eq+Configurable+Send+Sync> ErasedConfig for Config<T> {
	fn is_default(&self) -> bool {
		self.get() == self.default
	}
}
