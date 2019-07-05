#![feature(proc_macro_span, proc_macro_diagnostic)]
#![recursion_limit = "128"]

extern crate proc_macro;

mod invoke_list;
mod params;
mod util;

use proc_macro::{Diagnostic, Level, TokenStream};
use quote::quote;
use syn::{export::Span, parse::Parser, parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Comma, FieldValue, ItemEnum, ItemFn, ItemStatic, LitStr, ReturnType};

static COMMAND_INVOKELIST: invoke_list::InvocationList = invoke_list::InvocationList::new("Command");
static CONFIG_INVOKELIST: invoke_list::InvocationList = invoke_list::InvocationList::new("Config");

/// Register the function as a callable command with the given title and [keyboard shortcut](https://code.visualstudio.com/docs/getstarted/keybindings#_accepted-keys).
///
/// The shortcut is optional and can be omitted.
/// The macro also works on function that return `()` instead of `evscode::R<()>`.
/// Invoking this macro will automatically register the command within the VS Code event system.
/// ```ignore
/// #[evscode::command(title = "Example Evscode Extension - Hello World", key = "ctrl+alt+5")]
/// fn spawn() -> evscode::R<()> {
/// 	evscode::Message::new("Hello, world!").build().spawn();
/// 	Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn command(params: TokenStream, item: TokenStream) -> TokenStream {
	let modpath = util::get_modpath(&item);
	let params: params::Command = syn::parse_macro_input!(params);
	let item: ItemFn = syn::parse_macro_input!(item);
	let inner_id = util::js_path(&modpath, item.ident.to_string());
	let title = LitStr::new(&params.title, Span::call_site());
	let key = util::option_lit(params.key.map(|key| LitStr::new(&key, Span::call_site())));
	let raw_name = &item.ident;
	let trigger = match &item.decl.output {
		ReturnType::Default => quote! { (|| Ok(#raw_name())) },
		_ => quote! { #raw_name },
	};
	let machinery = COMMAND_INVOKELIST.invoke(quote! {
		evscode::meta::Command {
			inner_id: #inner_id,
			title: #title,
			key: #key,
			trigger: #trigger
		}
	});
	TokenStream::from(quote! {
		#item
		#machinery
	})
}

/// Specify all of the plugin metadata.
///
/// See [`evscode::meta::Package`](../evscode/meta/struct.Package.html) for a description and types of all available options.
/// This macro will generate a main function and should only be invoked once, from the main.rs filed.
/// ```ignore
/// evscode::plugin! {
/// 	name: "Example Evscode Extension",
/// 	publisher: "", // fill in your Marketplace publisher username.
/// 	description: "An example extension developed using Evscode",
/// 	keywords: &["test"],
/// 	categories: &["Other"],
/// 	license: "", // fill in an SPDX 2.0 identifier of your extension's license
/// 	repository: "", // fill in an URL of your extension repository.
/// 	on_activate: None,
/// 	extra_activations: &[],
/// 	log_filters: &[],
/// }
/// ```
#[proc_macro]
pub fn plugin(input: TokenStream) -> TokenStream {
	let fields = Punctuated::<FieldValue, Comma>::parse_terminated.parse(input).unwrap();
	let payload_name = quote! { evscode::meta::Command };
	let commands = COMMAND_INVOKELIST.payloads();
	let base_defs = COMMAND_INVOKELIST.base_definitions(payload_name);
	let config_name = quote! { evscode::meta::ConfigEntry };
	let config = CONFIG_INVOKELIST.payloads();
	let base_defs2 = CONFIG_INVOKELIST.base_definitions(config_name);
	let r = quote! {
		#base_defs
		#base_defs2
		fn main() {
			let manifest_dir = env!("CARGO_MANIFEST_DIR");
			let package = evscode::meta::Package {
				identifier: env!("CARGO_PKG_NAME"),
				version: env!("CARGO_PKG_VERSION"),
				commands: #commands,
				configuration: #config,
				#fields
			};
			evscode::internal::cli::run_main(&package, manifest_dir).expect("running failed");
		}
	};
	TokenStream::from(r)
}

/// Create a strongly-typed and automatically updated [config](../evscode/config/index.html) entry.
///
/// Use at any point in global scope, but insides of inline modules can cause problems.
/// The entry id will be derived from the current module path.
/// ```ignore
/// #[evscode::config(description = "Fooification time limit, expressed in milliseconds")]
/// static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);
/// ```
#[proc_macro_attribute]
pub fn config(params: TokenStream, item: TokenStream) -> TokenStream {
	let modpath = util::get_modpath(&item);
	let params: params::Config = syn::parse_macro_input!(params);
	let item: ItemStatic = syn::parse_macro_input!(item);
	let rust_type = item.ty.clone();
	let rust_inner_type = match *rust_type {
		syn::Type::Path(path) => match &path.path.segments[1].arguments {
			syn::PathArguments::AngleBracketed(args) => args.args[0].clone(),
			_ => panic!("expected evscode::Config<...>"),
		},
		_ => panic!("expected evscode::Config<...>"),
	};
	let default = item.expr.clone();
	let id = util::js_path(&modpath, util::caps_to_camel(item.ident.to_string()));
	let description = params.description;
	let reference = item.ident.clone();
	let visibility = item.vis.clone();
	let machinery = CONFIG_INVOKELIST.invoke(quote! {
		evscode::meta::ConfigEntry {
			id: #id,
			schema: |description| <#rust_inner_type as evscode::Configurable>::schema(
				Some(description),
				Some(&<#rust_inner_type as From<_>>::from(#default)),
			),
			description: #description,
			reference: std::ops::Deref::deref(&#reference),
		}
	});
	TokenStream::from(quote! {
		evscode::internal::macros::lazy_static! {
			#visibility static ref #reference: evscode::Config<#rust_inner_type> = evscode::Config::new(<#rust_inner_type as From<_>>::from(#default));
		}
		#machinery
	})
}

/// Derive Configurable trait for dataless enums, allowing them to be used in configs.
/// ```ignore
/// #[derive(evscode::Configurable)]
/// enum AnimalBackend {
/// 	#[evscode(name = "Doggo")]
/// 	Dog,
/// 	#[evscode(name = "Kitty")]
/// 	Cat,
/// }
/// ```
#[proc_macro_derive(Configurable, attributes(evscode))]
pub fn derive_configurable(input: TokenStream) -> TokenStream {
	let item: ItemEnum = parse_macro_input!(input);
	let enum_name = &item.ident;
	let vars = match collect_configurable_variants(&item) {
		Some(vars) => vars,
		None => return derive_unreachable_configurable(enum_name).into(),
	};
	let to_js: proc_macro2::TokenStream = vars
		.iter()
		.map(|var| {
			let pat = &var.ident;
			let name = &var.name;
			quote! {
				#enum_name::#pat => #name,
			}
		})
		.collect();
	let from_js: proc_macro2::TokenStream = vars
		.iter()
		.map(|var| {
			let name = &var.name;
			let out = &var.ident;
			quote! {
				#name => Ok(#enum_name::#out),
			}
		})
		.collect();
	let variants: proc_macro2::TokenStream = vars
		.iter()
		.map(|var| {
			let name = &var.name;
			quote! {
				#name,
			}
		})
		.collect();
	let expected = vars.iter().map(|var| format!("{:?}", var.name)).collect::<Vec<_>>().join(", ");
	TokenStream::from(quote! {
		impl evscode::marshal::Marshal for #enum_name {
			fn to_json(&self) -> evscode::json::JsonValue {
				evscode::json::from(match self {
					#to_js
				})
			}

			fn from_json(obj: evscode::json::JsonValue) -> Result<Self, String> {
				match obj.as_str().unwrap() {
					#from_js
					got => Err(format!("expected one of [{}], found `{:?}`", #expected, got)),
				}
			}
		}
		impl evscode::Configurable for #enum_name {
			fn schema(description: Option<&str>, default: Option<&Self>) -> evscode::json::JsonValue {
				let mut obj = evscode::json::object! {
					"type" => "string",
					"enum" => vec! [ #variants ],
				};
				if let Some(description) = description {
					obj["description"] = json::from(description);
				}
				if let Some(default) = default {
					obj["default"] = evscode::marshal::Marshal::to_json(default);
				}
				obj
			}
		}
	})
}

fn derive_unreachable_configurable(name: &proc_macro2::Ident) -> proc_macro2::TokenStream {
	quote! {
		impl evscode::Marshal for #name {
			fn from_json(_: &evscode::json::JsonValue) -> Self {
				unreachable!()
			}
			fn to_json(&self) -> evscode::json::JsonValue {
				unreachable!()
			}
		}
		impl evscode::Configurable for #name {
			fn schema(_: Option<&str>, _: Option<&Self>) -> evscode::json::JsonValue {
				unreachable!()
			}
		}
	}
}

#[derive(Debug)]
struct ConfVar {
	ident: syn::Ident,
	name: syn::LitStr,
}
fn collect_configurable_variants(item: &ItemEnum) -> Option<Vec<ConfVar>> {
	let mut conf_vars = Vec::new();
	for variant in &item.variants {
		let mut found = false;
		for attr in &variant.attrs {
			let attr = attr.parse_meta().expect("syn::Attribute::parse_meta failed");
			match attr {
				syn::Meta::List(ml) => {
					let stuff = ml.nested.into_iter().collect::<Vec<_>>();
					match stuff.as_slice() {
						[syn::NestedMeta::Meta(syn::Meta::NameValue(mnv))] => match &mnv.lit {
							syn::Lit::Str(s) => {
								conf_vars.push(ConfVar {
									ident: variant.ident.clone(),
									name: s.clone(),
								});
								if !found {
									found = true;
								} else {
									panic!("derive configuration received too many attributes");
								}
							},
							_ => panic!("derive_configurable bad input #4"),
						},
						_ => panic!("derive_configurable bad input #3"),
					}
				},
				_ => panic!("syn::Attribute::parse_meta has not returned a syn::Meta::List"),
			}
		}
		if !found {
			Diagnostic::spanned(
				vec![proc_macro::Span::call_site(), item.ident.span().unwrap()],
				Level::Error,
				"some variants do not have names",
			)
			.span_help(variant.span().unwrap(), "add a name attribute to this variant, e.g. #[evscode(name = \"Do nothing\")]")
			.emit();
			return None;
		}
	}
	Some(conf_vars)
}
