//! Tabs with custom interface built on HTML/CSS/JS.
//!
//! Consider using one of the [predefined webview patterns](../../goodies/index.html) or writing your own general pattern instead of using this
//! directly. See also the [official webview tutorial](https://code.visualstudio.com/api/extension-guides/webview).

use crate::{
	internal::executor::{send_object, HANDLE_FACTORY}, Column, LazyFuture
};
use json::JsonValue;
use std::sync::atomic::{AtomicBool, Ordering};

/// Builder for configurating webviews. See [module documentation](index.html) for details.
#[must_use]
pub struct Builder {
	view_type: String,
	title: String,
	view_column: Column,
	preserve_focus: bool,
	enable_command_uris: bool,
	enable_scripts: bool,
	local_resource_roots: Option<Vec<String>>,
	enable_find_widget: bool,
	retain_context_when_hidden: bool,
}
impl Builder {
	/// Do not focus the newly created webview.
	pub fn preserve_focus(mut self) -> Self {
		self.preserve_focus = true;
		self
	}

	/// Allow HTML/JS to launch VS Code commands using special URIs.
	pub fn enable_command_uris(mut self) -> Self {
		self.enable_command_uris = true;
		self
	}

	/// Allow JavaScript to run.
	pub fn enable_scripts(mut self) -> Self {
		self.enable_scripts = true;
		self
	}

	/// Enabled the find window available under the Ctrl+F shortcut.
	pub fn enable_find_widget(mut self) -> Self {
		self.enable_find_widget = true;
		self
	}

	/// Do not destroy webview state when the tab stops to be visible.
	/// According to VS Code API, this results in increased memory usage.
	/// However, this flag is required if the webview keep an internal state that can't be reconstructed from HTML alone.
	/// Also, not setting this flags results in a visible delay when opening the tab again.
	pub fn retain_context_when_hidden(mut self) -> Self {
		self.retain_context_when_hidden = true;
		self
	}

	/// Add a path from which assets created with [`crate::asset`] or `vscode-resource://` scheme can be used.
	/// If this function is not called at all, by default the extension install directory and the workspace directory are allowed.
	pub fn local_resource_root(mut self, uri: impl AsRef<str>) -> Self {
		match self.local_resource_roots.as_mut() {
			Some(lrr) => lrr.push(uri.as_ref().to_owned()),
			None => self.local_resource_roots = Some(vec![uri.as_ref().to_owned()]),
		}
		self
	}

	/// Spawn the webview.
	pub fn create(self) -> Webview {
		let hid = HANDLE_FACTORY.generate();
		send_object(json::object! {
			"tag" => "webview_create",
			"view_type" => self.view_type,
			"title" => self.title,
			"view_column" => self.view_column,
			"preserve_focus" => self.preserve_focus,
			"enable_command_uris" => self.enable_command_uris,
			"enable_scripts" => self.enable_scripts,
			"local_resource_roots" => self.local_resource_roots,
			"enable_find_widget" => self.enable_find_widget,
			"retain_context_when_hidden" => self.retain_context_when_hidden,
			"hid" => hid,
		});
		Webview { hid, listener_spawned: AtomicBool::new(false), disposer_spawned: AtomicBool::new(false) }
	}
}

/// Webview provided by the VS Code API.
///
/// See [module documentation](index.html) for details.
pub struct Webview {
	hid: u64,
	listener_spawned: AtomicBool,
	disposer_spawned: AtomicBool,
}
impl Webview {
	/// Create a new builder to configure the webview.
	/// View type is a panel type identifier.
	pub fn new(view_type: impl AsRef<str>, title: impl AsRef<str>, view_column: impl Into<Column>) -> Builder {
		Builder {
			view_type: view_type.as_ref().to_owned(),
			title: title.as_ref().to_owned(),
			view_column: view_column.into(),
			preserve_focus: false,
			enable_command_uris: false,
			enable_scripts: false,
			local_resource_roots: None,
			enable_find_widget: false,
			retain_context_when_hidden: false,
		}
	}

	/// Set the HTML content.
	pub fn set_html(&self, html: impl AsRef<str>) {
		send_object(json::object! {
			"tag" => "webview_set_html",
			"hid" => self.hid,
			"html" => html.as_ref(),
		});
	}

	/// Send a message which can be [received by the JS inside the webview](https://code.visualstudio.com/api/extension-guides/webview#passing-messages-from-an-extension-to-a-webview).
	///
	/// The messages are not guaranteed to arrive if the webview is not ["live"](https://code.visualstudio.com/api/references/vscode-api#1637) yet.
	/// To circumvent this horrible behaviour, whenever you call this method on a fresh webview, you must add script that sends a "I'm ready!" message and wait for it before calling.
	pub fn post_message(&self, msg: impl Into<JsonValue>) {
		send_object(json::object! {
			"tag" => "webview_post_message",
			"hid" => self.hid,
			"message" => msg,
		});
	}

	/// Check if the webview can be seen by the user.
	pub fn is_visible(&self) -> LazyFuture<bool> {
		let hid = self.hid;
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "webview_is_visible",
					"hid" => hid,
					"aid" => aid,
				})
			},
			|raw| raw.as_bool().unwrap(),
		)
	}

	/// Check whether the webview is the currently active webview.
	pub fn is_active(&self) -> LazyFuture<bool> {
		let hid = self.hid;
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "webview_is_active",
					"hid" => hid,
					"aid" => aid,
				})
			},
			|raw| raw.as_bool().unwrap(),
		)
	}

	/// Check whether the webview was closed.
	pub fn was_disposed(&self) -> LazyFuture<bool> {
		let hid = self.hid;
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "webview_was_disposed",
					"hid" => hid,
					"aid" => aid,
				})
			},
			|raw| raw.as_bool().unwrap(),
		)
	}

	/// Show the webview in the given view column.
	pub fn reveal(&self, view_column: impl Into<Column>) {
		send_object(json::object! {
			"tag" => "webview_reveal",
			"hid" => self.hid,
			"view_column" => view_column.into(),
		});
	}

	/// Close the webview.
	pub fn dispose(&self) {
		send_object(json::object! {
			"tag" => "webview_dispose",
			"hid" => self.hid,
		});
	}

	/// Returns a lazy future that will yield message [sent by JS inside the webview](https://code.visualstudio.com/api/extension-guides/webview#passing-messages-from-a-webview-to-an-extension).
	/// This function can only be called once.
	pub fn listener(&self) -> LazyFuture<JsonValue> {
		assert!(!self.listener_spawned.fetch_or(true, Ordering::SeqCst));
		let hid = self.hid;
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "webview_register_listener",
					"hid" => hid,
					"aid" => aid,
				})
			},
			|raw| raw.clone(),
		)
	}

	/// Returns a lazy future that will yield `()` when the webview is closed.
	/// This function can only be called once.
	pub fn disposer(&self) -> LazyFuture<()> {
		assert!(!self.disposer_spawned.fetch_or(true, Ordering::SeqCst));
		let hid = self.hid;
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "webview_register_disposer",
					"hid" => hid,
					"aid" => aid,
				})
			},
			|_| (),
		)
	}
}
