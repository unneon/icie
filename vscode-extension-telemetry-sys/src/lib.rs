use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "vscode-extension-telemetry")]
extern "C" {

	pub type default;

	#[wasm_bindgen(constructor, js_name = "default")]
	pub fn new(extension_id: &str, extension_version: &str, key: &str) -> default;

	#[wasm_bindgen(method, js_name = sendTelemetryEvent)]
	pub fn send_telemetry_event(
		this: &default,
		event_name: &str,
		properties: &JsValue,
		measurements: &JsValue,
	);

	#[wasm_bindgen(method, js_name = sendTelemetryException)]
	pub fn send_telemetry_exception(
		this: &default,
		error: &js_sys::Error,
		properties: &JsValue,
		measurements: &JsValue,
	);

}

pub type TelemetryReporter = default;
