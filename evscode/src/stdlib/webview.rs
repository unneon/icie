//! Tabs with custom interface built on HTML/CSS/JS.
//!
//! Consider using one of the [predefined webview patterns](../../goodies/index.html) or writing your own general pattern instead of using this
//! directly. See also the [official webview tutorial](https://code.visualstudio.com/api/extension-guides/webview).

use crate::{
	future::{Pong, PongStream}, internal::executor::{send_object, HANDLE_FACTORY}, Column
};
use futures::Stream;
use json::JsonValue;
use std::{
	future::Future, ops::Deref, pin::Pin, task::{Context, Poll}
};

/// Builder for configurating webviews. See [module documentation](index.html) for details.
#[must_use]
pub struct Builder<'a> {
	view_type: &'a str,
	title: &'a str,
	view_column: Column,
	preserve_focus: bool,
	enable_command_uris: bool,
	enable_scripts: bool,
	local_resource_roots: Option<&'a [&'a str]>,
	enable_find_widget: bool,
	retain_context_when_hidden: bool,
}
impl<'a> Builder<'a> {
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
	pub fn local_resource_roots(mut self, uris: &'a [&'a str]) -> Self {
		self.local_resource_roots = Some(uris);
		self
	}

	/// Spawn the webview.
	pub fn create(self) -> WebviewMeta {
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
		let webview = Webview { hid: WebviewRef { hid } };
		WebviewMeta { listener: webview.listener(), disposer: webview.disposer(), webview }
	}
}

/// Webview provided by the VS Code API.
///
/// See [module documentation](index.html) for details.
pub struct Webview {
	hid: WebviewRef,
}
impl Webview {
	/// Create a new builder to configure the webview.
	/// View type is a panel type identifier.
	pub fn new<'a>(view_type: &'a str, title: &'a str, view_column: impl Into<Column>) -> Builder<'a> {
		Builder {
			view_type,
			title,
			view_column: view_column.into(),
			preserve_focus: false,
			enable_command_uris: false,
			enable_scripts: false,
			local_resource_roots: None,
			enable_find_widget: false,
			retain_context_when_hidden: false,
		}
	}
}

impl Deref for Webview {
	type Target = WebviewRef;

	fn deref(&self) -> &Self::Target {
		&self.hid
	}
}

/// A cloneable reference to a webview.
///
/// Remains valid and usable even after the webview is dropped and destroyed, although various methods will naturally return errors.
#[derive(Clone)]
pub struct WebviewRef {
	pub(crate) hid: u64,
}

impl WebviewRef {
	/// Set the HTML content.
	pub fn set_html(&self, html: &str) {
		send_object(json::object! {
			"tag" => "webview_set_html",
			"hid" => self.hid,
			"html" => html,
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
	pub async fn is_visible(&self) -> bool {
		let hid = self.hid;
		let pong = Pong::new();
		send_object(json::object! {
			"tag" => "webview_is_visible",
			"hid" => hid,
			"aid" => pong.aid(),
		});
		pong.await.as_bool().unwrap()
	}

	/// Check whether the webview is the currently active webview.
	pub async fn is_active(&self) -> bool {
		let hid = self.hid;
		let pong = Pong::new();
		send_object(json::object! {
			"tag" => "webview_is_active",
			"hid" => hid,
			"aid" => pong.aid(),
		});
		pong.await.as_bool().unwrap()
	}

	/// Check whether the webview was closed.
	pub async fn was_disposed(&self) -> bool {
		let hid = self.hid;
		let pong = Pong::new();
		send_object(json::object! {
			"tag" => "webview_was_disposed",
			"hid" => hid,
			"aid" => pong.aid(),
		});
		pong.await.as_bool().unwrap()
	}

	/// Show the webview in the given view column.
	pub fn reveal(&self, view_column: impl Into<Column>, preserve_focus: bool) {
		send_object(json::object! {
			"tag" => "webview_reveal",
			"hid" => self.hid,
			"view_column" => view_column.into(),
			"preserve_focus" => preserve_focus,
		});
	}

	/// Close the webview.
	pub fn dispose(&self) {
		send_object(json::object! {
			"tag" => "webview_dispose",
			"hid" => self.hid,
		});
	}

	/// Creates the listener stream, only call this once.
	fn listener(&self) -> Listener {
		let hid = self.hid;
		let pong = PongStream::new();
		send_object(json::object! {
			"tag" => "webview_register_listener",
			"hid" => hid,
			"aid" => pong.aid(),
		});
		Listener { pong }
	}

	/// Creates the disposer future, only call this once.
	fn disposer(&self) -> Disposer {
		let hid = self.hid;
		let pong = Pong::new();
		send_object(json::object! {
			"tag" => "webview_register_disposer",
			"hid" => hid,
			"aid" => pong.aid(),
		});
		Disposer { pong }
	}
}

/// The products of creating a webview.
///
/// Aside from the actual webview, also contains the event stream and the dispose future.
pub struct WebviewMeta {
	/// The created webview.
	pub webview: Webview,
	/// A stream that will contain JSON messages sent by [JS inside the webview]https://code.visualstudio.com/api/extension-guides/webview#passing-messages-from-a-webview-to-an-extension).
	pub listener: Listener,
	/// A future that will yield a value when the webview is destroyed.
	pub disposer: Disposer,
}

/// A stream that will contain JSON messages sent by [JS inside the webview]https://code.visualstudio.com/api/extension-guides/webview#passing-messages-from-a-webview-to-an-extension).
pub struct Listener {
	pong: PongStream,
}

impl Stream for Listener {
	type Item = JsonValue;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		Pin::new(&mut self.pong).poll_next(cx)
	}
}

/// A future that will yield a value when the webview is destroyed.
pub struct Disposer {
	pong: Pong,
}

impl Future for Disposer {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.pong).poll(cx).map(|_| ())
	}
}
