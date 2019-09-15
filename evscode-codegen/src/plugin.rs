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
		fn main() {
			const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
			let package = Box::leak(Box::new(evscode::meta::Package {
				identifier: env!("CARGO_PKG_NAME"),
				version: env!("CARGO_PKG_VERSION"),
				commands: #commands,
				configuration: #config,
				#fields
			}));
			evscode::internal::cli::run_main(package, MANIFEST_DIR).expect("running failed");
		}
	})
}
