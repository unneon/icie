use crate::util::expand_path;
use evscode::{
	marshal::{type_error2, Marshal}, Configurable
};
use serde::{Deserializer, Serializer};
use std::{fmt, ops};
use wasm_bindgen::JsValue;

#[derive(Clone, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct PathBuf {
	buf: String,
}

pub type PathRef<'a> = &'a PathBuf;

impl PathBuf {
	/// Converts a native-encoded string received from JS to a [`PathBuf`].
	/// Passing a non-native string will result in various issues with string operations.
	pub fn from_native(buf: String) -> PathBuf {
		PathBuf { buf }
	}

	pub fn as_ref(&self) -> PathRef {
		&self
	}

	pub fn to_str(&self) -> Option<&str> {
		Some(&self.buf)
	}

	pub fn extension(&self) -> Option<String> {
		let raw = node_sys::path::extname(&self.buf);
		if raw.is_empty() { None } else { Some(raw[1..].to_owned()) }
	}

	pub fn file_name(&self) -> String {
		node_sys::path::basename(&self.buf)
	}

	pub fn file_stem(&self) -> String {
		let ext = node_sys::path::extname(&self.buf);
		node_sys::path::basename_with_ext(&self.buf, &ext)
	}

	pub fn join(&self, key: impl AsRef<str>) -> PathBuf {
		PathBuf::from_native(node_sys::path::join(&self.buf, key.as_ref()))
	}

	pub fn parent(&self) -> PathBuf {
		PathBuf::from_native(node_sys::path::dirname(&self.buf))
	}

	pub fn strip_prefix(&self, to: &PathBuf) -> Result<PathBuf, std::path::StripPrefixError> {
		Ok(PathBuf { buf: node_sys::path::relative(&self.buf, &to.buf) })
	}

	pub fn to_owned(&self) -> PathBuf {
		PathBuf { buf: self.buf.to_owned() }
	}

	pub fn with_extension(&self, new_ext: &str) -> PathBuf {
		let old_ext_len = match self.extension() {
			Some(old_ext) => old_ext.len() + 1,
			None => 0,
		};
		PathBuf::from_native(format!("{}.{}", &self.buf[..self.buf.len() - old_ext_len], new_ext))
	}
}

impl From<&'static str> for PathBuf {
	fn from(s: &'static str) -> Self {
		expand_path(s)
	}
}

impl fmt::Debug for PathBuf {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<String as fmt::Debug>::fmt(&self.buf, f)
	}
}

impl fmt::Display for PathBuf {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<String as fmt::Display>::fmt(&self.buf, f)
	}
}

impl serde::Serialize for PathBuf {
	fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
	where
		S: Serializer,
	{
		self.buf.serialize(serializer)
	}
}
impl<'de> serde::Deserialize<'de> for PathBuf {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		Ok(PathBuf::from_native(<String as serde::Deserialize>::deserialize(deserializer)?))
	}
}

impl ops::Deref for PathBuf {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.buf
	}
}

impl Marshal for PathBuf {
	fn to_js(&self) -> JsValue {
		JsValue::from_str(self.to_str().unwrap())
	}

	fn from_js(raw: JsValue) -> Result<Self, String> {
		Ok(expand_path(&raw.as_string().ok_or_else(|| type_error2("path", &raw))?))
	}
}

impl Configurable for PathBuf {
	fn to_json(&self) -> serde_json::Value {
		self.to_str().unwrap().into()
	}

	fn schema(default: Option<&Self>) -> serde_json::Value {
		<String as Configurable>::schema(default.map(|p| p.to_str().unwrap().to_owned()).as_ref())
	}
}
