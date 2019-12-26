use crate::util::{invoke_list::InvocationList, ProcError};
use proc_macro::{Diagnostic, Level, TokenStream};
use quote::quote;
use std::iter;
use syn::{
	parse2, parse_macro_input, spanned::Spanned, GenericArgument, ItemStatic, LitStr, PathArguments, Type
};

pub static CONFIG_INVOKELIST: InvocationList = InvocationList::new("Config");

pub fn generate(item: TokenStream) -> TokenStream {
	let item: ItemStatic = parse_macro_input!(item);
	transform(&item).unwrap_or_else(|_| dummy(&item))
}

fn transform(item: &ItemStatic) -> Result<TokenStream, ProcError> {
	let ty = extract_inner_type(item)?;
	let default = &item.expr;
	let local_name = &item.ident;
	let description = extract_description(item).unwrap_or_default();
	let vis = &item.vis;
	let machinery = CONFIG_INVOKELIST.invoke(quote! {
		evscode::meta::ConfigEntry {
			id: evscode::meta::Identifier {
				module_path: module_path!(),
				local_name: stringify!(#local_name),
			},
			telemetry_id: evscode::meta::Identifier {
				module_path: module_path!(),
				local_name: stringify!(#local_name)
			}.to_telemetry_fmt(),
			schema: || <#ty as evscode::Configurable>::schema(
				Some(&<#ty as From<_>>::from(#default)),
			),
			description: #description,
			reference: std::ops::Deref::deref(&#local_name),
		}
	});
	Ok(TokenStream::from(quote! {
		evscode::macros::lazy_static! {
			#vis static ref #local_name: evscode::Config<#ty> = evscode::Config::placeholder(
				<#ty as From<_>>::from(#default),
				evscode::meta::Identifier {
					module_path: module_path!(),
					local_name: stringify!(#local_name),
				},
			);
		}
		#machinery
	}))
}

fn extract_inner_type(item: &ItemStatic) -> Result<&Type, ProcError> {
	match &*item.ty {
		Type::Path(path) => {
			path.path.segments.iter().nth(1).and_then(|segment| match &segment.arguments {
				PathArguments::AngleBracketed(args) => {
					args.args.iter().next().and_then(|arg| match arg {
						GenericArgument::Type(ty) => Some(ty),
						_ => None,
					})
				},
				_ => None,
			})
		},
		_ => None,
	}
	.ok_or_else(|| {
		ProcError::new(Diagnostic::spanned(
			item.ty.span().unwrap(),
			Level::Error,
			"expected type `evscode::Config<...>`",
		))
	})
}

fn extract_description(item: &ItemStatic) -> Result<String, ProcError> {
	let value = item
		.attrs
		.iter()
		.filter(|attr| attr.path.is_ident("doc"))
		.map(|attr| {
			let lit = attr.tokens.clone().into_iter().nth(1).unwrap();
			let lit: LitStr = parse2(iter::once(lit).collect()).unwrap();
			lit.value()
		})
		.fold(None, |acc, line| {
			Some(match acc {
				Some(acc) => format!("{}\n{}", acc, line.trim()),
				None => line,
			})
		})
		.ok_or_else(|| {
			ProcError::new(Diagnostic::spanned(
				item.span().unwrap(),
				Level::Error,
				"configuration entries must have an attached doc comment",
			))
		})?;
	Ok(value)
}

fn dummy(_item: &ItemStatic) -> TokenStream {
	TokenStream::new()
}
