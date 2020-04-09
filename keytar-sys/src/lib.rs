use js_sys::Promise;
use wasm_bindgen::prelude::*;

#[derive(serde::Deserialize)]
pub struct Credentials {
	pub account: String,
	pub password: String,
}

#[wasm_bindgen(module = keytar)]
extern "C" {

	/// Returns Option<String>.
	#[wasm_bindgen(js_name = getPassword)]
	pub fn get_password(service: &str, account: &str) -> Promise;

	/// Returns ().
	#[wasm_bindgen(js_name = setPassword)]
	pub fn set_password(service: &str, account: &str, password: &str) -> Promise;

	/// Returns bool.
	#[wasm_bindgen(js_name = deletePassword)]
	pub fn delete_password(service: &str, account: &str) -> Promise;

	/// Returns Vec<Credentials>.
	#[wasm_bindgen(js_name = findCredentials)]
	pub fn find_credentials(service: &str) -> Promise;

}
