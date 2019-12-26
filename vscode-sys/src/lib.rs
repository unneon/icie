macro_rules! wasm_abi_serde {
	($t:ty) => {
		impl wasm_bindgen::convert::IntoWasmAbi for $t {
			type Abi = <wasm_bindgen::JsValue as wasm_bindgen::convert::IntoWasmAbi>::Abi;

			fn into_abi(self) -> Self::Abi {
				wasm_bindgen::JsValue::from_serde(&self).unwrap().into_abi()
			}
		}

		impl wasm_bindgen::describe::WasmDescribe for $t {
			fn describe() {
				<wasm_bindgen::JsValue as wasm_bindgen::describe::WasmDescribe>::describe()
			}
		}
	};
}
macro_rules! wasm_abi_enumi32 {
	($t:ty) => {
		impl wasm_bindgen::convert::IntoWasmAbi for $t {
			type Abi = <i32 as wasm_bindgen::convert::IntoWasmAbi>::Abi;

			fn into_abi(self) -> Self::Abi {
				self as i32
			}
		}

		impl wasm_bindgen::describe::WasmDescribe for $t {
			fn describe() {
				<i32 as wasm_bindgen::describe::WasmDescribe>::describe()
			}
		}
	};
}

use serde::{Deserialize, Serialize};
use std::{
	future::Future, marker::PhantomData, pin::Pin, task::{Context, Poll}
};
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};

pub struct Thenable<T: Thenability> {
	inner: wasm_bindgen_futures::JsFuture,
	phantom: PhantomData<T>,
}

impl<T: Thenability> Future for Thenable<T> {
	type Output = T;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
		Pin::new(&mut self.inner).poll(cx).map(|x| {
			Thenability::convert(match x {
				Ok(x) => x,
				Err(x) => x,
			})
		})
	}
}
impl<T: Thenability> wasm_bindgen::describe::WasmDescribe for Thenable<T> {
	fn describe() {
		<js_sys::Promise as wasm_bindgen::describe::WasmDescribe>::describe()
	}
}
impl<T: Thenability> wasm_bindgen::convert::FromWasmAbi for Thenable<T> {
	type Abi = <js_sys::Promise as wasm_bindgen::convert::FromWasmAbi>::Abi;

	unsafe fn from_abi(js: Self::Abi) -> Self {
		let promise = <js_sys::Promise as wasm_bindgen::convert::FromWasmAbi>::from_abi(js);
		Thenable { inner: wasm_bindgen_futures::JsFuture::from(promise), phantom: PhantomData }
	}
}

pub trait Thenability: Unpin {
	fn convert(x: JsValue) -> Self;
}
macro_rules! thenable_impl_raw {
	($t:ty, $method:ident) => {
		impl Thenability for $t {
			fn convert(x: JsValue) -> Self {
				x.$method().unwrap()
			}
		}
	};
}
macro_rules! thenable_impl_jscast {
	($t:ty) => {
		impl Thenability for $t {
			fn convert(x: JsValue) -> Self {
				x.unchecked_into()
			}
		}
	};
}
impl Thenability for () {
	fn convert(_: JsValue) -> Self {
	}
}
impl<T: Thenability> Thenability for Option<T> {
	fn convert(x: JsValue) -> Self {
		if x.is_null() || x.is_undefined() { None } else { Some(T::convert(x)) }
	}
}
impl<T: Thenability> Thenability for Vec<T> {
	fn convert(x: JsValue) -> Self {
		x.dyn_into::<js_sys::Array>()
			.unwrap()
			.values()
			.into_iter()
			.map(|val| T::convert(val.unwrap()))
			.collect()
	}
}
impl<T: Thenability> Thenability for Result<T, js_sys::Error> {
	fn convert(x: JsValue) -> Self {
		match x.dyn_into::<js_sys::Error>() {
			Ok(x) => Err(x),
			Err(x) => Ok(T::convert(x)),
		}
	}
}
thenable_impl_raw!(bool, as_bool);
thenable_impl_raw!(String, as_string);
thenable_impl_jscast!(JsValue);
thenable_impl_jscast!(TextEditor);
thenable_impl_jscast!(TextDocument);
thenable_impl_jscast!(Uri);

#[wasm_bindgen(module = vscode)]
extern "C" {

	pub type Uri;

	#[wasm_bindgen(static_method_of = Uri)]
	pub fn file(path: &str) -> Uri;

	#[wasm_bindgen(method, getter, js_name = fsPath)]
	pub fn fs_path(this: &Uri) -> String;

	#[wasm_bindgen(static_method_of = Uri)]
	pub fn parse(path: &str, strict: bool) -> Uri;

	pub type ExtensionContext;

	#[wasm_bindgen(method, getter, js_name = extensionPath)]
	pub fn get_extension_path(this: &ExtensionContext) -> String;

	#[wasm_bindgen(method, getter, js_name = globalState)]
	pub fn global_state(this: &ExtensionContext) -> Memento;

	#[wasm_bindgen(method, getter, js_name = workspaceState)]
	pub fn workspace_state(this: &ExtensionContext) -> Memento;

	pub type Webview;
	pub type WebviewPanel;

	#[wasm_bindgen(method, getter)]
	pub fn active(this: &WebviewPanel) -> bool;

	#[wasm_bindgen(method)]
	pub fn dispose(this: &WebviewPanel);

	#[wasm_bindgen(method, js_name = onDidDispose)]
	pub fn on_did_dispose(this: &WebviewPanel, callback: JsValue);

	#[wasm_bindgen(method)]
	pub fn reveal(this: &WebviewPanel, view_column: i32, preserve_focus: bool);

	#[wasm_bindgen(method, getter)]
	pub fn visible(this: &WebviewPanel) -> bool;

	#[wasm_bindgen(method, getter)]
	pub fn webview(this: &WebviewPanel) -> Webview;

	#[wasm_bindgen(method, js_name = onDidReceiveMessage)]
	pub fn on_did_receive_message(this: &Webview, callback: &Closure<dyn FnMut(JsValue)>);

	#[wasm_bindgen(method, js_name = postMessage)]
	pub fn post_message(this: &Webview, message: JsValue) -> Thenable<bool>;

	#[wasm_bindgen(method, setter)]
	pub fn set_html(this: &Webview, html: &str);

	pub type TextDocument;

	#[wasm_bindgen(method, getter, js_name = fileName)]
	pub fn file_name(this: &TextDocument) -> String;

	#[wasm_bindgen(method, js_name = getText)]
	pub fn text(this: &TextDocument) -> String;

	pub type TextEditor;

	#[wasm_bindgen(method)]
	pub fn edit(
		this: &TextEditor,
		callback: &Closure<dyn FnMut(&TextEditorEdit)>,
	) -> Thenable<bool>;

	#[wasm_bindgen(method, getter)]
	pub fn document(this: &TextEditor) -> TextDocument;

	#[wasm_bindgen(method, js_name = revealRange)]
	pub fn reveal_range(this: &TextEditor, range: &Range, reveal_type: TextEditorRevealType);

	#[wasm_bindgen(method, setter)]
	pub fn set_selection(this: &TextEditor, selection: Selection);

	pub type TextEditorEdit;

	#[wasm_bindgen(method)]
	pub fn insert(this: &TextEditorEdit, location: &Position, value: &str);

	pub type Position;

	#[wasm_bindgen(constructor)]
	pub fn new(line: usize, character: usize) -> Position;

	pub type Clipboard;

	#[wasm_bindgen(method, js_name = writeText)]
	pub fn write_text(this: &Clipboard, value: &str) -> Thenable<()>;

	pub type Selection;

	#[wasm_bindgen(constructor)]
	pub fn new(anchor: &Position, active: &Position) -> Selection;

	pub type Range;

	#[wasm_bindgen(constructor)]
	pub fn new(start: &Position, end: &Position) -> Range;

	pub type Terminal;

	#[wasm_bindgen(method, js_name = sendText)]
	pub fn send_text(this: &Terminal, text: &str, add_new_line: Option<bool>);

	#[wasm_bindgen(method)]
	pub fn show(this: &Terminal, preserve_focus: Option<bool>);

	pub type Memento;

	#[wasm_bindgen(method)]
	pub fn get(this: &Memento, key: &str) -> JsValue;

	#[wasm_bindgen(method)]
	pub fn update(this: &Memento, key: &str, value: &JsValue) -> Thenable<()>;

	pub type StatusBarItem;

	#[wasm_bindgen(method)]
	pub fn hide(this: &StatusBarItem);

	#[wasm_bindgen(method, setter)]
	pub fn set_text(this: &StatusBarItem, text: &str);

	#[wasm_bindgen(method)]
	pub fn show(this: &StatusBarItem);

	// Not an actual type, but Progress<{increment: number, message: string}>.
	pub type ProgressProgress;

	#[wasm_bindgen(method)]
	pub fn report(this: &ProgressProgress, value: ProgressProgressValue);

	pub type CancellationToken;

	#[wasm_bindgen(method, getter, js_name = onCancellationRequested)]
	pub fn on_cancellation_requested(this: &CancellationToken) -> Event;

	pub type Event;

}

#[derive(Deserialize)]
pub struct ItemRet<T> {
	pub id: T,
}

#[repr(i32)]
pub enum TextEditorRevealType {
	AtTop = 3,
	Default = 0,
	InCenter = 1,
	InCenterIfOutsideViewport = 2,
}
wasm_abi_enumi32!(TextEditorRevealType);

#[derive(Serialize)]
pub struct ProgressProgressValue<'a> {
	pub increment: Option<f64>,
	pub message: Option<&'a str>,
}
wasm_abi_serde!(ProgressProgressValue<'_>);

pub mod commands {
	use crate::Thenable;
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = vscode)]
	extern "C" {

		#[wasm_bindgen(js_namespace = commands, js_name = executeCommand, variadic)]
		pub fn execute_command(command: &str, rest: js_sys::Array) -> Thenable<JsValue>;

		#[wasm_bindgen(js_namespace = commands, js_name = registerCommand)]
		pub fn register_command(command: &str, callback: &Closure<dyn FnMut()>);

	}
}

pub mod env {

	use crate::{Clipboard, Thenable, Uri};
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = vscode)]
	extern "C" {

		#[wasm_bindgen(js_namespace = env, js_name = clipboard)]
		pub static CLIPBOARD: Clipboard;

		#[wasm_bindgen(js_namespace = env, js_name = openExternal)]
		pub fn open_external(uri: &Uri) -> Thenable<bool>;

	}
}

pub mod window {

	use crate::{StatusBarItem, Terminal, TextDocument, TextEditor, Thenable, WebviewPanel};
	use serde::{Serialize, Serializer};
	use std::collections::HashMap;
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = vscode)]
	extern "C" {

		#[wasm_bindgen(js_namespace = window, js_name = activeTextEditor)]
		pub static ACTIVE_TEXT_EDITOR: Option<TextEditor>;

		#[wasm_bindgen(js_namespace = window, js_name = createStatusBarItem)]
		pub fn create_status_bar_item() -> StatusBarItem;

		#[wasm_bindgen(js_namespace = window, js_name = createTerminal)]
		pub fn create_terminal(options: TerminalOptions) -> Terminal;

		#[wasm_bindgen(js_namespace = window, js_name = createWebviewPanel)]
		pub fn create_webview_panel(
			view_type: &str,
			title: &str,
			show_options: CreateWebviewPanelShowOptions,
			options: CreateWebviewPanelOptions,
		) -> WebviewPanel;

		#[wasm_bindgen(js_namespace = window, js_name = showErrorMessage, variadic)]
		pub fn show_error_message(
			message: &str,
			options: &JsValue,
			items: Vec<JsValue>,
		) -> Thenable<JsValue>;

		#[wasm_bindgen(js_namespace = window, js_name = showInformationMessage, variadic)]
		pub fn show_information_message(
			message: &str,
			options: &JsValue,
			items: Vec<JsValue>,
		) -> Thenable<JsValue>;

		#[wasm_bindgen(js_namespace = window, js_name = showInputBox)]
		pub fn show_input_box(options: InputBoxOptions) -> Thenable<Option<String>>;

		#[wasm_bindgen(js_namespace = window, js_name = showOpenDialog)]
		pub fn show_open_dialog(options: OpenDialogOptions) -> Thenable<Option<Vec<String>>>;

		#[wasm_bindgen(js_namespace = window, js_name = showQuickPick)]
		pub fn show_quick_pick(
			items: &js_sys::Array,
			options: ShowQuickPickOptions,
		) -> Thenable<JsValue>;

		#[wasm_bindgen(js_namespace = window, js_name = showTextDocument)]
		pub fn show_text_document(document: &TextDocument) -> Thenable<TextEditor>;

		#[wasm_bindgen(js_namespace = window, js_name = showWarningMessage, variadic)]
		pub fn show_warning_message(
			message: &str,
			options: &JsValue,
			items: Vec<JsValue>,
		) -> Thenable<JsValue>;

		#[wasm_bindgen(js_namespace = window, js_name = visibleTextEditors)]
		pub static VISIBLE_TEXT_EDITORS: js_sys::Array;

		#[wasm_bindgen(js_namespace = window, js_name = withProgress)]
		pub fn with_progress(options: ProgressOptions, task: JsValue);

	}

	#[derive(Serialize)]
	pub struct CreateWebviewPanelOptions {
		#[serde(flatten)]
		pub general: WebviewOptions,
		#[serde(flatten)]
		pub panel: WebviewPanelOptions,
	}
	wasm_abi_serde!(CreateWebviewPanelOptions);

	#[derive(Serialize)]
	pub struct InputBoxOptions<'a> {
		#[serde(rename = "ignoreFocusOut")]
		pub ignore_focus_out: bool,
		pub password: bool,
		#[serde(rename = "placeHolder")]
		pub place_holder: Option<&'a str>,
		pub prompt: Option<&'a str>,
		pub value: Option<&'a str>,
		#[serde(rename = "valueSelection")]
		pub value_selection: Option<[usize; 2]>,
	}
	wasm_abi_serde!(InputBoxOptions<'_>);

	#[derive(Serialize)]
	pub struct OpenDialogOptions {
		#[serde(rename = "canSelectFiles")]
		pub can_select_files: bool,
		#[serde(rename = "canSelectFolders")]
		pub can_select_folders: bool,
		#[serde(rename = "canSelectMany")]
		pub can_select_many: bool,
		pub filters: Option<HashMap<String, Vec<String>>>,
		#[serde(rename = "openLabel")]
		pub open_label: Option<String>,
	}
	wasm_abi_serde!(OpenDialogOptions);

	#[derive(Serialize)]
	pub struct ProgressOptions<'a> {
		pub cancellable: bool,
		pub location: ProgressLocation,
		pub title: Option<&'a str>,
	}
	wasm_abi_serde!(ProgressOptions<'_>);

	#[repr(i32)]
	#[derive(Copy, Clone)]
	pub enum ProgressLocation {
		Notification = 15,
		SourceControl = 1,
		Window = 10,
	}
	wasm_abi_serde!(ProgressLocation);
	impl Serialize for ProgressLocation {
		fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
			serializer.serialize_i32(*self as i32)
		}
	}

	#[derive(Serialize)]
	pub struct TerminalOptions<'a> {
		pub cwd: Option<&'a str>,
		pub env: Option<HashMap<String, String>>,
		#[serde(rename = "hideFromUser")]
		pub hide_from_user: Option<bool>,
		pub name: Option<&'a str>,
		#[serde(rename = "shellArgs")]
		pub shell_args: Option<Vec<String>>,
		#[serde(rename = "shellPath")]
		pub shell_path: Option<&'a str>,
		#[serde(rename = "strictEnv")]
		pub strict_env: Option<bool>,
	}
	wasm_abi_serde!(TerminalOptions<'_>);

	#[derive(Serialize)]
	pub struct WebviewPanelOptions {
		#[serde(rename = "enableFindWidget")]
		pub enable_find_widget: bool,
		#[serde(rename = "retainContextWhenHidden")]
		pub retain_context_when_hidden: bool,
	}

	#[derive(Serialize)]
	pub struct WebviewOptions {
		#[serde(rename = "enableCommandUris")]
		pub enable_command_uris: bool,
		#[serde(rename = "enableScripts")]
		pub enable_scripts: bool,
	}

	#[derive(Serialize)]
	pub struct CreateWebviewPanelShowOptions {
		#[serde(rename = "preserveFocus")]
		pub preserve_focus: bool,
		#[serde(rename = "viewColumn")]
		pub view_column: i32,
	}
	wasm_abi_serde!(CreateWebviewPanelShowOptions);

	#[derive(Serialize)]
	pub struct ShowMessageOptions {
		pub modal: bool,
	}
	wasm_abi_serde!(ShowMessageOptions);

	#[derive(Serialize)]
	pub struct ShowQuickPickOptions<'a> {
		#[serde(rename = "canPickMany")]
		pub can_pick_many: bool,
		#[serde(rename = "ignoreFocusOut")]
		pub ignore_focus_out: bool,
		#[serde(rename = "matchOnDescription")]
		pub match_on_description: bool,
		#[serde(rename = "matchOnDetail")]
		pub match_on_detail: bool,
		#[serde(rename = "placeHolder")]
		pub place_holder: Option<&'a str>,
	}
	wasm_abi_serde!(ShowQuickPickOptions<'_>);

	#[derive(Serialize)]
	pub struct ShowMessageItem<'a, T> {
		#[serde(rename = "isCloseAffordance")]
		pub is_close_affordance: bool,
		pub title: &'a str,
		pub id: T,
	}

	#[derive(Serialize)]
	pub struct ShowQuickPickItem<'a, T> {
		#[serde(rename = "alwaysShow")]
		pub always_show: bool,
		pub description: Option<&'a str>,
		pub detail: Option<&'a str>,
		pub label: &'a str,
		pub picked: bool,
		pub id: T,
	}
}

pub mod workspace {

	use crate::{TextDocument, Thenable, Uri};
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = vscode)]
	extern "C" {

		#[wasm_bindgen(js_namespace = workspace, js_name = findFiles)]
		pub fn find_files(include: &str) -> Thenable<Vec<Uri>>;

		#[wasm_bindgen(js_namespace = workspace, js_name = getConfiguration)]
		pub fn get_configuration(id: &str) -> JsValue;

		#[wasm_bindgen(js_namespace = workspace, js_name = openTextDocument)]
		pub fn open_text_document(file_name: &str)
		-> Thenable<Result<TextDocument, js_sys::Error>>;

		#[wasm_bindgen(js_namespace = workspace, js_name = rootPath)]
		pub static ROOT_PATH: JsValue;

		#[wasm_bindgen(js_namespace = workspace, js_name = saveAll)]
		pub fn save_all(include_untitled: bool) -> Thenable<bool>;

	}
}
