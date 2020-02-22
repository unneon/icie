use crate::{error::ErrorCode, Error, Result};
use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize};
use wasm_backtrace::Backtrace;

pub async fn from_resp<T: DeserializeOwned>(resp: Response) -> Result<T> {
	let resp_raw = resp.text().await?;
	from_str(&resp_raw)
}

pub fn from_str<'d, T: Deserialize<'d>>(resp_raw: &'d str) -> Result<T> {
	serde_json::from_str(resp_raw).map_err(|e| Error {
		code: ErrorCode::AlienInvasion,
		cause: Some(Box::new(e)),
		backtrace: Backtrace::new(),
	})
}
