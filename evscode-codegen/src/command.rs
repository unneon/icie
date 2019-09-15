use crate::util::{
	invoke_list::InvocationList, option_literal, params::{self, ParamMap}
};
use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{
	parse::{Parse, ParseStream}, parse_macro_input, ItemFn, LitStr
};

pub static COMMAND_INVOKELIST: InvocationList = InvocationList::new("Command");

pub fn generate(params: TokenStream, item: TokenStream) -> TokenStream {
	let params: Params = parse_macro_input!(params);
	let item: ItemFn = parse_macro_input!(item);
	let local_name = &item.sig.ident;
	let title = LitStr::new(&params.title, Span::call_site().into());
	let key = option_literal(params.key.map(|key| LitStr::new(&key, Span::call_site().into())));
	let raw_name = &item.sig.ident;
	let machinery = COMMAND_INVOKELIST.invoke(quote! {
		evscode::meta::Command {
			id: evscode::meta::Identifier {
				module_path: module_path!(),
				local_name: stringify!(#local_name),
			},
			title: #title,
			key: #key,
			trigger: || Box::pin(#raw_name()),
		}
	});
	TokenStream::from(quote! {
		#item
		#machinery
	})
}

#[derive(Debug)]
pub struct Params {
	pub title: String,
	pub key: Option<String>,
}
impl Parse for Params {
	fn parse(input: ParseStream) -> params::R<Params> {
		let mut params: ParamMap = input.parse()?;
		let r = Params { title: params.get("title")?, key: params.get("key")? };
		params.finish()?;
		Ok(r)
	}
}
