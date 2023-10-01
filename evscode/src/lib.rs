//! Evscode is a Rust framework for writing WebAssembly-based Visual Studio Code extensions. More information is
//! included in CONTRIBUTING.md file.

#![feature(const_fn_trait_bound, try_trait_v2,async_closure)]
#![allow(clippy::new_ret_no_self)]
#![deny(missing_docs)]
#![feature(type_alias_impl_trait)]

pub mod config;
pub mod error;
mod glue;
pub mod goodies;
#[doc(hidden)]
pub mod macros;
pub mod marshal;
pub mod meta;
pub mod stdlib;

use crate::glue::PACKAGE;
pub use config::{Config, Configurable};
pub use error::{E, R};
pub use evscode_codegen::{command, config, plugin, *};
use std::{future::Future, pin::Pin};
pub use stdlib::*;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output=T>+'a>>;

/// Spawn an asynchronous operation concurrently to the active one.
pub fn spawn(f: impl Future<Output=R<()>>+'static) {
	wasm_bindgen_futures::spawn_local(async move {
		if let Err(e) = f.await {
			if let Some(on_error) = &PACKAGE.get().unwrap().on_error {
				on_error(e).await;
			}
		}
	});
}
