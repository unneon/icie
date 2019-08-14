//! Configuration system based on VS Code API, with strong typing and auto-reload built in.
//!
//! To add a configuration entry, write this in global scope of any file of an extension:
//! ```ignore
//! /// Fooification time limit, expressed in milliseconds
//! #[evscode::config]
//! static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);
//! ```
//! The entry will be automatically added to the extension manifest and registered for updates.
//! To read the values, first call [`Config::get`] to get an [`std::sync::Arc`] with the current value.
//! The [`Config`] will receive configuration updates and return up-to-date values, but the returned pointer will not.
//!
//! The system supports all types that implement the [`Configurable`](trait.Configurable.html) trait.
//! If the conversion provided via the [`Marshal`](../marshal/trait.Marshal.html) trait fails, the default value will be used.

use crate::marshal::Marshal;
use arc_swap::ArcSwapOption;
use json::JsonValue;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

macro_rules! optobject_impl {
	($obj:ident, ) => {};
	($obj:ident, optional $key:expr => $value:expr, $($rest:tt)*) => {
		if let Some(value) = $value {
			$obj[$key] = json::from(value);
		}
		optobject_impl!($obj, $($rest)*);
	};
	($obj:ident, $key:expr => $value:expr, $($rest:tt)*) => {
		$obj[$key] = json::from($value);
		optobject_impl!($obj, $($rest)*);
	};
}
macro_rules! optobject {
	{ $($token:tt)* } => {
		let mut obj = json::JsonValue::new_object();
		optobject_impl!(obj, $($token)*);
		obj
	};
}

/// Wrapper object for an automatically updated configuration entry.
///
/// To get the current value, call the [`Config::get`] method which will return an [`std::sync::Arc`].
/// Do not store the Arc for extended periods of time, so that your extension is responsive to configuration updates.
#[derive(Debug)]
pub struct Config<T: Configurable+Sized> {
	arc: ArcSwapOption<T>,
}
impl<T: Configurable> Config<T> {
	#[doc(hidden)]
	pub fn placeholder() -> Config<T> {
		Config { arc: ArcSwapOption::new(None) }
	}

	/// Return a reference-counted pointer to the current configuration values.
	pub fn get(&self) -> Arc<T> {
		self.arc.load_full().expect("evscode::Config::get config not set")
	}
}

/// A trait that allows a type to be used as a configuration values.
///
/// This trait can be [automatically derived](../../evscode_codegen/derive.Configurable.html) for enums that hold no data.
/// ```ignore
/// #[derive(evscode::Configurable)]
/// enum AnimalBackend {
/// 	#[evscode(name = "Doggo")]
/// 	Dog,
/// 	#[evscode(name = "Kitty")]
/// 	Cat,
/// }
/// ```
/// There does not exist a simple way to implement it for any custom types, because the VS Code [documentation of config API](https://code.visualstudio.com/api/references/contribution-points#contributes.configuration) is lacking.
pub trait Configurable: Marshal {
	#[doc(hidden)]
	fn schema(default: Option<&Self>) -> JsonValue;
}

macro_rules! simple_configurable {
	($rust:ty, $json:expr) => {
		impl Configurable for $rust {
			fn schema(default: Option<&Self>) -> JsonValue {
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
simple_configurable!(PathBuf, "string");
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
	fn schema(default: Option<&Option<T>>) -> JsonValue {
		let mut obj = T::schema(default.and_then(|default| default.as_ref()));
		if obj["type"].is_array() {
			obj["type"].push("null").unwrap();
		} else {
			obj["type"] = json::array!["null", obj["type"].as_str().unwrap()];
		}
		if let Some(None) = default {
			obj["default"] = json::Null;
		}
		obj
	}
}
/// This implementation is not editable in VS Code setting UI.
/// I am not sure why, because VS Code has builtin configuration entries that have the same manifest entry, but are editable.
/// Naturally, the [documentation](https://code.visualstudio.com/api/references/contribution-points#contributes.configuration) of this behaviour does not exist.
impl<T: Configurable, S: std::hash::BuildHasher+Default> Configurable for HashMap<String, T, S> {
	fn schema(default: Option<&Self>) -> JsonValue {
		optobject! {
			"type" => "object",
			optional "default" => default.map(Self::to_json),
			"additionalProperties" => json::object! {
				"anyOf" => json::array! [T::schema(None)],
			},
		}
	}
}

#[doc(hidden)]
pub trait ErasedConfig {
	fn update(&self, raw: JsonValue) -> Result<(), String>;
}
impl<T: Configurable> ErasedConfig for Config<T> {
	fn update(&self, raw: JsonValue) -> Result<(), String> {
		match T::from_json(raw) {
			Ok(obj) => {
				let arc = Some(Arc::new(obj));
				self.arc.swap(arc);
				Ok(())
			},
			Err(e) => {
				self.arc.swap(None);
				Err(e)
			},
		}
	}
}
