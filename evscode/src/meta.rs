//! Extension metadata types.

use crate::{config::ErasedConfig, R};
use json::JsonValue;

#[doc(hidden)]
#[derive(Debug)]
pub struct Command {
	pub inner_id: &'static str,
	pub title: &'static str,
	pub key: Option<&'static str>,
	pub trigger: fn() -> R<()>,
}

#[doc(hidden)]
pub struct ConfigEntry {
	pub id: &'static str,
	pub markdown_description: &'static str,
	pub reference: &'static dyn ErasedConfig,
	pub schema: fn() -> JsonValue,
}

/// [Activation event](https://code.visualstudio.com/api/references/activation-events) checked by VS Code even when the extension is not active.
///
/// Set the [`Package::extra_activations`] field in [`evscode::plugin!`](../../evscode_codegen/macro.plugin.html) call to register the check.
pub enum Activation<S: AsRef<str>> {
	#[doc(hidden)]
	OnCommand { command: S },
	/// Fires when a folder is opened and it contains at least one file that matched the given selector.
	/// See [official documentation](https://code.visualstudio.com/api/references/activation-events#workspaceContains).
	WorkspaceContains {
		/// Glob file pattern, like `**/.editorconfig`.
		selector: S,
	},
}
#[doc(hidden)]
impl Activation<&'static str> {
	pub fn own(&self) -> Activation<String> {
		match self {
			Activation::OnCommand { command } => Activation::OnCommand { command: command.to_string() },
			Activation::WorkspaceContains { selector } => Activation::WorkspaceContains { selector: selector.to_string() },
		}
	}
}
#[doc(hidden)]
impl Activation<String> {
	pub fn package_json_format(&self) -> JsonValue {
		json::from(match self {
			Activation::OnCommand { command } => format!("onCommand:{}", command),
			Activation::WorkspaceContains { selector } => format!("workspaceContains:{}", selector),
		})
	}
}

/// Extension metadata.
///
/// See [official documentation](https://code.visualstudio.com/api/references/extension-manifest) for detailed information.
pub struct Package {
	#[doc(hidden)]
	pub identifier: &'static str,
	#[doc(hidden)]
	pub version: &'static str,
	#[doc(hidden)]
	pub commands: Vec<Command>,
	#[doc(hidden)]
	pub configuration: Vec<ConfigEntry>,
	/// Display name seen by end users.
	pub name: &'static str,
	/// Your Marketplace [publisher](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#publishers-and-personal-access-tokens) username.
	pub publisher: &'static str,
	/// Short description of your extension.
	pub description: &'static str,
	/// Up to 5 keywords to make it easier to find the extension.
	pub keywords: &'static [&'static str],
	/// Categories that describe your extension, out of the following list: `Programming Languages, Snippets, Linters, Themes, Debuggers, Formatters,
	/// Keymaps, SCM Providers, Other, Extension Packs, Language Packs`
	pub categories: &'static [&'static str],
	/// [SPDX 2.0](https://spdx.org/licenses/) identifier of your extension's license.
	pub license: &'static str,
	/// URL of your extension repository.
	pub repository: &'static str,
	/// Function intended to run when the extension is activated.
	/// Prefer to use [lazy_static](https://docs.rs/lazy_static) for initializing global state.
	pub on_activate: Option<fn() -> R<()>>,
	/// Additional [`Activation`] events that will activate your extension.
	/// Evscode will automatically add events related to the commands in your extension.
	pub extra_activations: &'static [Activation<&'static str>],
	/// List of filters that specify what can be logged.
	/// An entry like `("html5ever", LevelFilter::Error)` means that only errors of level Error or higher will be visible in developer tools.
	pub log_filters: &'static [(&'static str, log::LevelFilter)],
}
