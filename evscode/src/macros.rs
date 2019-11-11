pub use crate::{
	config::Configurable, glue::{activate, deactivate, generate_package_json}
};
pub use lazy_static::lazy_static;
pub use vscode_sys::ExtensionContext;

pub trait InvocChain<T> {
	type Payload;
	fn payload() -> Self::Payload;
	fn is_last() -> bool;
	type Next: InvocChain<(), Payload=Self::Payload>;
}

pub fn collect_payloads<C: InvocChain<()>>() -> Vec<C::Payload> {
	let mut buf = Vec::new();
	ic_recurse::<C>(&mut buf, true);
	buf
}

fn ic_recurse<C: InvocChain<()>>(buf: &mut Vec<C::Payload>, is_guard: bool) {
	if !is_guard {
		buf.push(C::payload());
	}
	if !C::is_last() {
		ic_recurse::<C::Next>(buf, false);
	}
}
