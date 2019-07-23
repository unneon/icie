use proc_macro2::TokenStream;
use quote::quote;
use std::{convert::AsRef, path};

pub fn js_path(modpath: &[String], id: impl AsRef<str>) -> String {
	let mut buf = String::new();
	for part in modpath {
		buf += &format!("{}.", part);
	}
	buf += id.as_ref();
	buf
}

pub fn get_modpath(input: &proc_macro::TokenStream) -> Vec<String> {
	let source_path = input.clone().into_iter().last().expect("evscode_codegen::get_modpath empty TokenStream").span().source_file().path();
	let mut parts = source_path
		.components()
		.map(|comp| match comp {
			path::Component::Normal(part) => part.to_str().expect("evscode_cogen::get_modpath non-utf8 path").to_owned(),
			_ => panic!("badly behaved source path"),
		})
		.collect::<Vec<_>>();
	if parts.first().map(|s| s.as_str()) != Some("src") {
		parts.remove(0); // remove crate name
	}
	parts.remove(0); // remove src/
	if parts.last().map(|s| s.as_str()) == Some("main.rs") {
		parts.pop();
	}
	if parts.last().map(|s| s.as_str()) == Some("mod.rs") {
		parts.pop();
	}
	if let Some(part) = parts.last_mut() {
		if part.ends_with(".rs") {
			part.pop();
			part.pop();
			part.pop();
		}
	}
	parts
}

pub fn caps_to_camel(s: impl AsRef<str>) -> String {
	let mut buf = String::new();
	for (i, word) in s.as_ref().split('_').enumerate() {
		for (j, chr) in word.chars().enumerate() {
			if i != 0 && j == 0 {
				buf += &chr.to_uppercase().to_string();
			} else {
				buf += &chr.to_lowercase().to_string();
			}
		}
	}
	buf
}

pub fn option_lit(x: Option<impl quote::ToTokens>) -> TokenStream {
	match x {
		Some(x) => quote! { Some(#x) },
		None => quote! { None },
	}
}
