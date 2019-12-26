use crate::util::ProcError;
use proc_macro::{Diagnostic, Level, Span, TokenStream};
use quote::quote;
use syn::{
	parse_macro_input, spanned::Spanned, Attribute, Ident, ItemEnum, Lit, LitStr, Meta, MetaNameValue, NestedMeta
};

pub fn generate(input: TokenStream) -> TokenStream {
	let item: ItemEnum = parse_macro_input!(input);
	transform(&item).unwrap_or_else(|_| dummy(&item))
}

fn transform(item: &ItemEnum) -> Result<TokenStream, ProcError> {
	let enum_name = &item.ident;
	let variants = extract_enum_variants(&item)?;
	let variant_idents = variants.iter().map(|variant| &variant.ident).collect::<Vec<_>>();
	let variant_names = variants.iter().map(|variant| &variant.name).collect::<Vec<_>>();
	Ok(TokenStream::from(quote! {
		impl evscode::marshal::Marshal for #enum_name {
			fn to_js(&self) -> wasm_bindgen::JsValue {
				wasm_bindgen::JsValue::from_str(match self {
					#(#enum_name::#variant_idents => #variant_names,)*
				})
			}

			fn from_js(obj: wasm_bindgen::JsValue) -> Result<Self, String> {
				match obj.as_string().unwrap().as_str() {
					#(#variant_names => Ok(#enum_name::#variant_idents),)*
					got => Err(format!("expected one of [{}], found `{:?}`", stringify!(#(#variant_names),*), got)),
				}
			}
		}
		impl evscode::Configurable for #enum_name {
			fn to_json(&self) -> serde_json::Value {
				serde_json::Value::from(match self {
					#(#enum_name::#variant_idents => #variant_names,)*
				})
			}

			fn schema(default: Option<&Self>) -> serde_json::Value {
				let mut obj = serde_json::json!({
					"type": "string",
					"enum": vec! [
						#(#variant_names,)*
					],
				});
				if let Some(default) = default {
					obj["default"] = default.to_json();
				}
				obj
			}
		}
	}))
}

fn dummy(item: &ItemEnum) -> TokenStream {
	let ident = &item.ident;
	TokenStream::from(quote! {
		impl evscode::marshal::Marshal for #ident {
			fn to_json(&self) -> evscode::json::JsonValue {
				unreachable!()
			}
			fn from_json(_: evscode::json::JsonValue) -> Result<Self, String> {
				unreachable!()
			}
		}
		impl evscode::Configurable for #ident {
			fn schema(_: Option<&Self>) -> evscode::json::JsonValue {
				unreachable!()
			}
		}
	})
}

#[derive(Debug)]
struct EnumVariant<'a> {
	ident: &'a Ident,
	name: LitStr,
}
fn extract_enum_variants(item: &ItemEnum) -> Result<Vec<EnumVariant>, ProcError> {
	item.variants
		.iter()
		.map(|variant| {
			let ident = &variant.ident;
			let attr = find_attribute("evscode", &variant.attrs, ident.span().unwrap())?;
			let name = parse_attribute(attr)?;
			Ok(EnumVariant { ident, name })
		})
		.collect::<Vec<_>>()
		.into_iter()
		.collect()
}

fn find_attribute<'a>(
	ident: &'static str,
	attrs: &'a [Attribute],
	span: Span,
) -> Result<&'a Attribute, ProcError>
{
	attrs.iter().find(|attr| attr.path.is_ident(ident)).ok_or_else(|| {
		ProcError::new(Diagnostic::spanned(
			span,
			Level::Error,
			format!("requires `{}` attribute", ident),
		))
	})
}

fn parse_attribute(attr: &Attribute) -> Result<LitStr, ProcError> {
	let name =
		parse_meta_name_value_list::<1>(attr).and_then(|[meta_name_value]| match &meta_name_value
			.lit
		{
			Lit::Str(name) if meta_name_value.path.is_ident("name") => Some(name.clone()),
			_ => None,
		});
	name.ok_or_else(|| {
		ProcError::new(Diagnostic::spanned(
			attr.span().unwrap(),
			Level::Error,
			"expected `(name = \"...\")` inside the attribute",
		))
	})
}

fn parse_meta_name_value_list<const N: usize>(attr: &Attribute) -> Option<[MetaNameValue; N]>
where [MetaNameValue; N]: array_init::IsArray<Item=MetaNameValue> {
	let meta = attr.parse_meta().ok()?;
	let meta_list = match meta {
		Meta::List(meta_list) => meta_list,
		_ => return None,
	};
	if meta_list.nested.len() != N {
		return None;
	}
	array_init::from_iter::<[MetaNameValue; N], _>(
		meta_list
			.nested
			.into_iter()
			.map(|nested_meta| match nested_meta {
				NestedMeta::Meta(Meta::NameValue(meta_name_value)) => Some(meta_name_value),
				_ => None,
			})
			.collect::<Option<Vec<_>>>()?,
	)
}
