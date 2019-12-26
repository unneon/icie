use proc_macro2::Span;
use std::collections::HashMap;
use syn::{
	parse::{Parse, ParseStream}, punctuated::Punctuated, spanned::Spanned, token::Comma, Lit, MetaNameValue
};

pub type R<T> = syn::parse::Result<T>;
type E = syn::parse::Error;

pub struct ParamMap {
	storage: HashMap<String, MetaNameValue>,
}
impl Parse for ParamMap {
	fn parse(input: ParseStream) -> R<ParamMap> {
		type PMNVC = Punctuated<MetaNameValue, Comma>;
		let raw_kv = PMNVC::parse_separated_nonempty(input)?;
		let mut storage = HashMap::new();
		for mnv in raw_kv {
			let key = mnv.path.segments.first().unwrap().ident.to_string();
			if let Some(dupl) = storage.insert(key, mnv) {
				return Err(E::new(dupl.path.span(), "duplicate parameter"));
			}
		}
		Ok(ParamMap { storage })
	}
}
impl ParamMap {
	pub fn get<T: Param>(&mut self, key: &'static str) -> R<T> {
		Param::convert(self.storage.remove(key), key)
	}

	pub fn finish(self) -> R<()> {
		match self.storage.values().next() {
			Some(mnv) => Err(E::new(mnv.path.span(), "unrecognized parameter")),
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
		<Option<String> as Param>::convert(mnv, key)?
			.ok_or_else(|| E::new(Span::call_site(), format!("parameter `{}` is required", key)))
	}
}
