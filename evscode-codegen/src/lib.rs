#![feature(const_generics, proc_macro_diagnostic)]
#![allow(incomplete_features)]

extern crate proc_macro;

use proc_macro::TokenStream;

mod command;
mod config;
mod configurable;
mod plugin;
mod util;

/// Register the function as a callable command with the given title and [keyboard shortcut](https://code.visualstudio.com/docs/getstarted/keybindings#_accepted-keys).
///
/// The shortcut is optional and can be omitted.
/// The macro also works on functions that return `()` instead of `evscode::R<()>`.
/// Invoking this macro will automatically register the command within the VS Code event system.
/// ```ignore
/// #[evscode::command(title = "Example Evscode Extension - Hello World", key = "ctrl+alt+5")]
/// fn spawn() -> evscode::R<()> {
///     evscode::Message::new("Hello, world!").build().spawn();
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn command(params: TokenStream, item: TokenStream) -> TokenStream {
	command::generate(params, item)
}

/// Create a strongly-typed and automatically updated [config](../evscode/config/index.html) entry.
///
/// Compatible with any type that implements
/// [`evscode::Configurable`](../evscode/config/Configurable.trait). Can be used at any point in
/// global scope, and the entry id will be created based on the module path and variable name.
/// The description will be extracted from the doc comment.
/// ```ignore
/// /// Fooification time limit, expressed in milliseconds
/// #[evscode::config]
/// static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);
/// ```
#[proc_macro_attribute]
pub fn config(_params: TokenStream, item: TokenStream) -> TokenStream {
	config::generate(item)
}

/// Derive Configurable trait for dataless enums, allowing them to be used in configs.
/// ```ignore
/// #[derive(evscode::Configurable)]
/// enum AnimalBackend {
///     #[evscode(name = "Doggo")]
///     Dog,
///     #[evscode(name = "Kitty")]
///     Cat,
/// }
/// ```
#[proc_macro_derive(Configurable, attributes(evscode))]
pub fn derive_configurable(input: TokenStream) -> TokenStream {
	configurable::generate(input)
}

/// Specify all of the plugin metadata.
///
/// See [`evscode::meta::Package`](../evscode/meta/struct.Package.html) for a description and types
/// of all available options. This macro will generate a main function and should only be invoked
/// once, from the main.rs file. ```ignore
/// evscode::plugin! {
///     name: "Example Evscode Extension",
///     publisher: "", // fill in your Marketplace publisher username.
///     description: "An example extension developed using Evscode",
///     keywords: &["test"],
///     categories: &["Other"],
///     license: "", // fill in an SPDX 2.0 identifier of your extension's license
///     repository: "", // fill in an URL of your extension repository.
///     on_activate: None,
///     extra_activations: &[],
///     log_filters: &[],
/// }
/// ```
#[proc_macro]
pub fn plugin(input: TokenStream) -> TokenStream {
	plugin::generate(input)
}
