use crate::{Error, Result};
use backtrace::Backtrace;
use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize};

pub async fn from_resp<T: DeserializeOwned>(resp: Response, endpoint: &'static str) -> Result<T> {
	let resp_raw = resp.text().await?;
	serde_json::from_str(&resp_raw).map_err(|e| Error::UnexpectedJSON { endpoint, backtrace: Backtrace::new(), resp_raw, inner: Some(Box::new(e)) })
}

pub fn from_str<'d, T: Deserialize<'d>>(resp_raw: &'d str, endpoint: &'static str) -> Result<T> {
	serde_json::from_str(resp_raw).map_err(|e| Error::UnexpectedJSON {
		endpoint,
		backtrace: Backtrace::new(),
		resp_raw: resp_raw.to_owned(),
		inner: Some(Box::new(e)),
	})
}
