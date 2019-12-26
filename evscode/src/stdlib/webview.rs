//! Tabs with custom interface built on HTML/CSS/JS.
//!
//! Consider using one of the [predefined webview patterns](../../goodies/index.html) or writing
//! your own general pattern instead of using this directly. See also the [official webview tutorial](https://code.visualstudio.com/api/extension-guides/webview).

use crate::Column;
use futures::{
	channel::{mpsc, oneshot}, Stream
};
use serde::Serialize;
use std::{
	future::Future, ops::Deref, pin::Pin, task::{Context, Poll}
};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

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
	/// However, this flag is required if the webview keep an internal state that can't be
	/// reconstructed from HTML alone. Also, not setting this flags results in a visible delay when
	/// opening the tab again.
	pub fn retain_context_when_hidden(mut self) -> Self {
		self.retain_context_when_hidden = true;
		self
	}

	/// Add a path from which assets created with [`crate::asset`] or `vscode-resource://` scheme
	/// can be used. If this function is not called at all, by default the extension install
	/// directory and the workspace directory are allowed.
	pub fn local_resource_roots(mut self, uris: &'a [&'a str]) -> Self {
		self.local_resource_roots = Some(uris);
		self
	}

	/// Spawn the webview.
	pub fn create(self) -> WebviewMeta {
		let panel = vscode_sys::window::create_webview_panel(
			self.view_type,
			self.title,
			vscode_sys::window::CreateWebviewPanelShowOptions {
				preserve_focus: self.preserve_focus,
				view_column: self.view_column.as_enum_id(),
			},
			vscode_sys::window::CreateWebviewPanelOptions {
				general: vscode_sys::window::WebviewOptions {
					enable_scripts: self.enable_scripts,
					enable_command_uris: self.enable_command_uris,
				},
				panel: vscode_sys::window::WebviewPanelOptions {
					enable_find_widget: self.enable_find_widget,
					retain_context_when_hidden: self.retain_context_when_hidden,
				},
			},
		);
		let webview = Webview { reference: WebviewRef { panel } };
		WebviewMeta { listener: webview.listener(), disposer: webview.disposer(), webview }
	}
}

/// Webview provided by the VS Code API.
///
/// See [module documentation](index.html) for details.
pub struct Webview {
	reference: WebviewRef,
}
impl Webview {
	/// Create a new builder to configure the webview.
	/// View type is a panel type identifier.
	pub fn new<'a>(
		view_type: &'a str,
		title: &'a str,
		view_column: impl Into<Column>,
	) -> Builder<'a>
	{
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
		&self.reference
	}
}

/// A cloneable reference to a webview.
///
/// Remains valid and usable even after the webview is dropped and destroyed, although various
/// methods will naturally return errors.
pub struct WebviewRef {
	pub(crate) panel: vscode_sys::WebviewPanel,
}

impl WebviewRef {
	/// Set the HTML content.
	pub fn set_html(&self, html: &str) {
		self.panel.webview().set_html(html);
	}

	/// Send a message which can be [received by the JS inside the webview](https://code.visualstudio.com/api/extension-guides/webview#passing-messages-from-an-extension-to-a-webview).
	///
	/// The messages are not guaranteed to arrive if the webview is not ["live"](https://code.visualstudio.com/api/references/vscode-api#1637) yet.
	/// To circumvent this horrible behaviour, whenever you call this method on a fresh webview, you
	/// must add script that sends a "I'm ready!" message and wait for it before calling.
	pub async fn post_message(&self, msg: impl Serialize) -> bool {
		self.panel.webview().post_message(JsValue::from_serde(&msg).unwrap()).await
	}

	/// Check if the webview can be seen by the user.
	pub fn is_visible(&self) -> bool {
		self.panel.visible()
	}

	/// Check whether the webview is the currently active webview.
	pub fn is_active(&self) -> bool {
		self.panel.active()
	}

	/// Show the webview in the given view column.
	pub fn reveal(&self, view_column: impl Into<Column>, preserve_focus: bool) {
		self.panel.reveal(view_column.into().as_enum_id(), preserve_focus);
	}

	/// Close the webview.
	pub fn dispose(&self) {
		self.panel.dispose();
	}

	/// Creates the listener stream, only call this once.
	fn listener(&self) -> Listener {
		let (tx, rx) = mpsc::unbounded();
		let capture = Closure::wrap(Box::new(move |event| {
			let _ = tx.unbounded_send(event);
		}) as Box<dyn FnMut(JsValue)>);
		self.panel.webview().on_did_receive_message(&capture);
		Listener { _capture: capture, rx }
	}

	/// Creates the disposer future, only call this once.
	fn disposer(&self) -> Disposer {
		let (tx, rx) = oneshot::channel();
		self.panel.on_did_dispose(Closure::once_into_js(move || {
			let _ = tx.send(());
		}));
		Disposer { rx }
	}
}

impl Clone for WebviewRef {
	fn clone(&self) -> Self {
		WebviewRef { panel: self.panel.clone().unchecked_into::<vscode_sys::WebviewPanel>() }
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
	_capture: Closure<dyn FnMut(JsValue)>,
	rx: mpsc::UnboundedReceiver<JsValue>,
}

impl Stream for Listener {
	type Item = JsValue;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		Pin::new(&mut self.rx).poll_next(cx)
	}
}

/// A future that will yield a value when the webview is destroyed.
pub struct Disposer {
	rx: oneshot::Receiver<()>,
}

impl Future for Disposer {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
		Pin::new(&mut self.rx).poll(cx).map(Result::unwrap)
	}
}
