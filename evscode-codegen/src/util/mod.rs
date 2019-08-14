use proc_macro::Diagnostic;
use quote::quote;

pub mod invoke_list;
pub mod params;

pub struct ProcError(());

impl ProcError {
	pub fn new(e: Diagnostic) -> ProcError {
		e.emit();
		ProcError(())
	}
}

pub fn option_literal(x: Option<impl quote::ToTokens>) -> proc_macro2::TokenStream {
	match x {
		Some(x) => quote! { Some(#x) },
		None => quote! { None },
	}
}
