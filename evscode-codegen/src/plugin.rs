use crate::{command::COMMAND_INVOKELIST, config::CONFIG_INVOKELIST};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parser, punctuated::Punctuated, token::Comma, FieldValue};

pub fn generate(input: TokenStream) -> TokenStream {
	let fields = Punctuated::<FieldValue, Comma>::parse_terminated.parse(input).unwrap();
	let payload_name = quote! { evscode::meta::Command };
	let commands = COMMAND_INVOKELIST.payloads();
	let base_defs = COMMAND_INVOKELIST.base_definitions(payload_name);
	let config_name = quote! { evscode::meta::ConfigEntry };
	let config = CONFIG_INVOKELIST.payloads();
	let base_defs2 = CONFIG_INVOKELIST.base_definitions(config_name);
	TokenStream::from(quote! {
		#base_defs
		#base_defs2

		#[wasm_bindgen::prelude::wasm_bindgen(js_name = internal_generate_package_json)]
		pub fn __evscode_generate_package_json(path: &str) {
			evscode::macros::generate_package_json(path, __evscode_metadata());
		}

		#[wasm_bindgen::prelude::wasm_bindgen(js_name = activate)]
		pub fn __evscode_activate(ctx: &evscode::macros::ExtensionContext) {
			evscode::macros::activate(ctx, __evscode_metadata());
		}

		#[wasm_bindgen::prelude::wasm_bindgen(js_name = deactivate)]
		pub async fn __evscode_deactivate() {
			evscode::macros::deactivate().await;
		}

		fn __evscode_metadata() -> evscode::meta::Package  {
			evscode::meta::Package {
				identifier: env!("CARGO_PKG_NAME"),
				version: env!("CARGO_PKG_VERSION"),
				commands: #commands,
				configuration: #config,
				#fields
			}
		}
	})
}
