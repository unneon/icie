use crate::{Error, Result};
use backtrace::Backtrace;
use reqwest::Response;
use serde::de::DeserializeOwned;

pub fn from_resp<T: DeserializeOwned>(mut resp: Response, endpoint: &'static str) -> Result<T> {
	let resp_raw = resp.text()?;
	serde_json::from_str(&resp_raw).map_err(|e| Error::UnexpectedJSON { endpoint, backtrace: Backtrace::new(), resp_raw, inner: Some(Box::new(e)) })
}
