//! Extension metadata types.

use crate::{config::ErasedConfig, BoxFuture, E, R};
use std::fmt::{self, Write};

/// Returns a vector with metadata on all configuration entries in the plugin.
pub fn config_entries() -> &'static [ConfigEntry] {
	crate::glue::CONFIG_ENTRIES.get().unwrap()
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Identifier {
	pub module_path: &'static str,
	pub local_name: &'static str,
}

impl fmt::Display for Identifier {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		for part in self.module_path.split("::") {
			crate::marshal::camel_case(part, f)?;
			f.write_char('.')?;
		}
		crate::marshal::camel_case(self.local_name, f)
	}
}

impl Identifier {
	#[doc(hidden)]
	pub fn to_telemetry_fmt(&self) -> String {
		format!("config_delta_{}", self).replace(".", "").to_lowercase()
	}

	pub(crate) fn extension_id(&self) -> String {
		let full_path = self.to_string();
		let i = full_path.find('.').unwrap();
		full_path[..i].to_owned()
	}

	pub(crate) fn inner_path(&self) -> String {
		let full_path = self.to_string();
		let i = full_path.find('.').unwrap();
		full_path[i + 1..].to_owned()
	}
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Command {
	pub id: Identifier,
	pub title: &'static str,
	pub key: Option<&'static str>,
	pub trigger: fn() -> BoxFuture<'static, R<()>>,
}

/// Metadata of a configuration entry.
#[derive(Clone)]
pub struct ConfigEntry {
	#[doc(hidden)]
	pub id: Identifier,
	/// An nicely formatted identifier for use with telemetry or other purposes.
	/// Will look like "config_delta_pdfautopen".
	pub telemetry_id: String,
	/// Uses Markdown.
	#[doc(hidden)]
	pub description: &'static str,
	#[doc(hidden)]
	pub reference: &'static dyn ErasedConfig,
	#[doc(hidden)]
	pub schema: fn() -> serde_json::Value,
}

impl ConfigEntry {
	/// Returns a 0 or 1 depending on whether user has changed the value of this entry.
	pub fn telemetry_config_delta(&self) -> f64 {
		if self.reference.is_default() { 0.0 } else { 1.0 }
	}
}

/// [Activation event](https://code.visualstudio.com/api/references/activation-events) checked by VS Code even when the extension is not active.
///
/// Set the [`Package::extra_activations`] field in
/// [`evscode::plugin!`](../../evscode_codegen/macro.plugin.html) call to register the check.
pub enum Activation<S: AsRef<str>> {
	#[doc(hidden)]
	OnCommand { command: Identifier },
	/// Fires when a folder is opened and it contains at least one file that matched the given
	/// selector. See [official documentation](https://code.visualstudio.com/api/references/activation-events#workspaceContains).
	WorkspaceContains {
		/// Glob file pattern, like `**/.editorconfig`.
		selector: S,
	},
}
#[doc(hidden)]
impl Activation<&'static str> {
	pub fn own(&self) -> Activation<String> {
		match self {
			Activation::OnCommand { command } => Activation::OnCommand { command: *command },
			Activation::WorkspaceContains { selector } => {
				Activation::WorkspaceContains { selector: (*selector).to_owned() }
			},
		}
	}
}
#[doc(hidden)]
impl Activation<String> {
	pub fn package_json_format(&self) -> String {
		match self {
			Activation::OnCommand { command } => format!("onCommand:{}", command),
			Activation::WorkspaceContains { selector } => format!("workspaceContains:{}", selector),
		}
	}
}

/// How should the banner text be displayed.
pub enum GalleryTheme {
	/// Light background, dark text.
	Light,
	/// Dark background, light text.
	Dark,
}

/// Metadata about banner in VS Marketplace.
pub struct Gallery {
	/// A HTML RGB color of the banner.
	pub color: &'static str,
	/// Options of rendering text on top of the banner.
	pub theme: GalleryTheme,
}

/// A future that executes a non-Send operation only once, but is Send.
pub type LazyOnceFuture = dyn (FnOnce() -> BoxFuture<'static, R<()>>)+Send+Sync;

/// A lazy future that executes a non-Send operation, but is Send.
pub type ErrorCatcher = dyn (Fn(E) -> BoxFuture<'static, ()>)+Send+Sync;

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
	/// Categories that describe your extension, out of the following list: `Programming Languages,
	/// Snippets, Linters, Themes, Debuggers, Formatters, Keymaps, SCM Providers, Other, Extension
	/// Packs, Language Packs`
	pub categories: &'static [&'static str],
	/// [SPDX 2.0](https://spdx.org/licenses/) identifier of your extension's license.
	pub license: &'static str,
	/// URL of your extension repository.
	pub repository: &'static str,
	/// Metadata about the display in VS Marketplace.
	pub gallery: Gallery,
	/// Function intended to run when the extension is activated.
	/// Prefer to use [lazy_static](https://docs.rs/lazy_static) for initializing global state.
	pub on_activate: Option<Box<LazyOnceFuture>>,
	/// Function intended to run when the extension is deactivated.
	pub on_deactivate: Option<Box<LazyOnceFuture>>,
	/// Function intended to run when an error is returned from an action handler.
	pub on_error: Option<Box<ErrorCatcher>>,
	/// Additional [`Activation`] events that will activate your extension.
	/// Evscode will automatically add events related to the commands in your extension.
	pub extra_activations: &'static [Activation<&'static str>],
	/// Minimal required vscode version.
	/// TODO: Check what happens when the requirement is not fulfilled.
	pub vscode_version: &'static str,
	/// Additional dependencies to append to package.json.
	/// They can be later imported using wasm-bindgen.
	pub node_dependencies: &'static [(&'static str, &'static str)],
	/// Telemetry instrumentation key, set up in [Azure Apllication Insights](https://github.com/microsoft/vscode-extension-telemetry).
	pub telemetry_key: &'static str,
}
