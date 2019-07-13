use proc_macro2::Span;
use std::collections::{hash_map::Entry, HashMap};
use syn::{
	parse::{Parse, ParseStream}, punctuated::Punctuated, spanned::Spanned, token::Comma, Lit, MetaNameValue
};

type R<T> = syn::parse::Result<T>;
type E = syn::parse::Error;

pub struct Params {
	storage: HashMap<String, MetaNameValue>,
}
impl Parse for Params {
	fn parse(input: ParseStream) -> R<Params> {
		type PMNVC = Punctuated<MetaNameValue, Comma>;
		let raw_kv = PMNVC::parse_separated_nonempty(input)?;
		let mut storage = HashMap::new();
		for mnv in raw_kv {
			let key = mnv.ident.to_string();
			match storage.entry(key) {
				Entry::Occupied(_) => return Err(E::new(mnv.ident.span(), "duplicate parameter")),
				Entry::Vacant(vacant) => vacant.insert(mnv),
			};
		}
		Ok(Params { storage })
	}
}
impl Params {
	pub fn get<T: Param>(&mut self, key: &'static str) -> R<T> {
		Param::convert(self.storage.remove(key), key)
	}

	pub fn finish(self) -> R<()> {
		match self.storage.values().next() {
			Some(mnv) => Err(E::new(mnv.ident.span(), "unrecognized parameter")),
			None => Ok(()),
		}
	}
}
pub trait Param: Sized {
	fn convert(mnv: Option<MetaNameValue>, key: &'static str) -> R<Self>;
}
impl Param for Option<String> {
	fn convert(mnv: Option<MetaNameValue>, _key: &'static str) -> R<Self> {
		match mnv {
			Some(mnv) => match mnv.lit {
				Lit::Str(lit) => Ok(Some(lit.value())),
				lit => Err(E::new(lit.span(), "expected a string literal")),
			},
			None => Ok(None),
		}
	}
}
impl Param for String {
	fn convert(mnv: Option<MetaNameValue>, key: &'static str) -> R<Self> {
		match <Option<String> as Param>::convert(mnv, key)? {
			Some(value) => Ok(value),
			None => Err(E::new(Span::call_site(), format!("parameter `{}` is required", key))),
		}
	}
}

#[derive(Debug)]
pub struct Command {
	pub title: String,
	pub key: Option<String>,
}
impl Parse for Command {
	fn parse(input: ParseStream) -> R<Command> {
		let mut params: Params = input.parse()?;
		let r = Command { title: params.get("title")?, key: params.get("key")? };
		params.finish()?;
		Ok(r)
	}
}

#[derive(Debug)]
pub struct Config {
	pub description: String,
}
impl Parse for Config {
	fn parse(input: ParseStream) -> R<Config> {
		let mut params: Params = input.parse()?;
		let r = Config { description: params.get("description")? };
		params.finish()?;
		Ok(r)
	}
}
