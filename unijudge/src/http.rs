use crate::{Error, Result};
use http::{header::HeaderName, HeaderValue};
use reqwest::RequestBuilder;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, ops::Deref};
use url::Url;

#[derive(Debug)]
pub struct Client {
	inner: reqwest::Client,
}

impl Client {
	pub fn new(user_agent: &'static str) -> Result<Client> {
		Ok(Client {
			inner: reqwest::Client::builder()
				.default_headers([(HeaderName::from_static("user-agent"), HeaderValue::from_static(user_agent))].iter().cloned().collect())
				.cookie_store(true)
				.build()
				.map_err(Error::NoTLS)?,
		})
	}

	pub fn cookie_set(&self, cookie: Cookie, url: &str) -> Result<()> {
		let mut cookies = self.inner.cookies().unwrap().write().map_err(|_| Error::StateCorruption)?;
		cookies.0.insert_raw(&cookie.cookie, &url.parse()?).map_err(|_| Error::WrongData)?;
		Ok(())
	}

	pub fn cookie_get(&self, key: &str) -> Result<Option<Cookie>> {
		self.cookie_get_if(|name| name == key)
	}

	pub fn cookie_get_if(&self, mut key: impl FnMut(&str) -> bool) -> Result<Option<Cookie>> {
		let cookies = self.inner.cookies().unwrap().read().map_err(|_| Error::StateCorruption)?;
		let cookie = match cookies.0.iter_unexpired().find(|cookie| key(cookie.name())) {
			Some(cookie) => Cookie { cookie: cookie.deref().clone().into_owned() },
			None => panic!("must find!"),
		};
		Ok(Some(cookie))
	}

	pub fn get(&self, url: Url) -> RequestBuilder {
		self.inner.get(url)
	}

	pub fn post(&self, url: Url) -> RequestBuilder {
		self.inner.post(url)
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cookie {
	#[serde(serialize_with = "serialize_raw_cookie", deserialize_with = "deserialize_raw_cookie")]
	cookie: cookie::Cookie<'static>,
}

impl Cookie {
	pub fn value(&self) -> &str {
		self.cookie.value()
	}
}

fn serialize_raw_cookie<S: Serializer>(cookie: &cookie::Cookie<'static>, serializer: S) -> std::result::Result<S::Ok, S::Error> {
	serializer.serialize_str(&cookie.to_string())
}

fn deserialize_raw_cookie<'d, D: Deserializer<'d>>(deserializer: D) -> std::result::Result<cookie::Cookie<'static>, D::Error> {
	deserializer.deserialize_str(RawCookieVisitor)
}
struct RawCookieVisitor;
impl<'d> de::Visitor<'d> for RawCookieVisitor {
	type Value = cookie::Cookie<'static>;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "cookie")
	}

	fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
		v.parse().map_err(de::Error::custom)
	}
}
