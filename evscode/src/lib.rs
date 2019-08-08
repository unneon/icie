//! <!-- UPDATE THIS DOCUMENTATION IN BOTH README.md and src/lib.rs WHEN UPDATING -->
//!
//! # Evscode
//!
//! Evscode is a Rust framework for writing Visual Studio Code extensions. Extensions are based on native code, not wasm, despite it being
//! discouraged. This approach is rather hacky, requires nightly and works only on Linux, but is designed mainly to be pleasant to use. This means
//! following a batteries-included mindset, so Evscode contains a custom build system, handles application event loops, configuration and offers some
//! helpers for common webview usage patterns.
//!
//! ## Developing extensions
//!
//! Create a new Rust executable crate, add Evscode to dependencies, create `README.md`, `CHANGELOG.md` and enter the following in `main.rs`:
//! ```ignore
//! #![feature(specialization)]
//!
//! #[evscode::command(title = "Example Evscode Extension - Hello World", key = "ctrl+alt+5")]
//! fn spawn() -> evscode::R<()> {
//! 	evscode::Message::new("Hello, world!").build().spawn();
//! 	Ok(())
//! }
//!
//! evscode::plugin! {
//! 	name: "Example Evscode Extension",
//! 	publisher: "", // fill in your Marketplace publisher username.
//! 	description: "An example extension developed using Evscode",
//! 	keywords: &["test"],
//! 	categories: &["Other"],
//! 	license: "", // fill in an SPDX 2.0 identifier of your extension's license
//! 	repository: "", // fill in an URL of your extension repository.
//! 	on_activate: None,
//! 	extra_activations: &[],
//! 	log_filters: &[],
//! }
//! ```
//! Run the extension with `cargo run` and see that it displays the message after pressing <kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>5</kbd>.
//!
//! ## Build system
//!
//! The built extensions will work on Linux, and compilation also requires Linux. First, make sure npm and rsync are installed. Then, run `cargo run` to launch a debug session. To package an extension, run `cargo run --release -- --package`(requires that [vsce](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#installation) is installed). To publish an extension, [log in to vsce](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#publishing-extensions) and run `cargo run --release -- --publish`.

#![feature(associated_type_defaults, const_fn, try_trait)]
#![allow(clippy::new_ret_no_self)]
#![deny(missing_docs)]

pub mod config;
pub mod error;
pub mod future;
pub mod goodies;
#[doc(hidden)]
pub mod internal;
pub mod marshal;
pub mod meta;
pub mod runtime;
pub mod stdlib;

pub use config::{Config, Configurable};
pub use error::{E, R};
pub use evscode_codegen::{command, config, plugin, *};
pub use future::{Future, LazyFuture};
pub use goodies::*;
pub use json;
pub use stdlib::*;
