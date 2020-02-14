//! Workspace-local and global state
//!
//! Create a global const state object and query it using [`State::get`] and [`State::set`] methods.
//! ```
//! # use evscode::{Scope, State};
//! const DEBUGGER_LAUNCH_COUNT: State<i64> = State::new("debugger-launch-count", Scope::Global);
//! ```

use crate::{marshal::Marshal, E, R};
use std::marker::PhantomData;
use wasm_bindgen::{JsCast, JsValue};

/// Scope of the stored values.
pub enum Scope {
	/// State with workspace scope will be different for every VS Code workspace.
	Workspace,
	/// State with global scope will be the same for every VS Code workspace.
	Global,
}

/// Object used to store storage entry metadata information.
pub struct State<T: Marshal> {
	key: &'static str,
	scope: Scope,
	_phantom: PhantomData<T>,
}

impl<T: Marshal+Send+serde::Serialize> State<T> {
	/// Create a new storage entry with a given identifier and scope.
	/// Multiple storage objects with the same key will refer to the same entry.
	pub const fn new(key: &'static str, scope: Scope) -> State<T> {
		State { key, scope, _phantom: PhantomData }
	}

	/// Query the stored value, if it was ever saved.
	pub fn get(&self) -> R<Option<T>> {
		let value = self.memento().get(self.key);
		if !value.is_undefined() {
			match T::from_js(value) {
				Ok(obj) => Ok(Some(obj)),
				Err(e) => Err(E::error(e).context("internal extension type error")),
			}
		} else {
			Ok(None)
		}
	}

	/// Set the storage entry to a given value.
	pub async fn set(&self, value: &T) {
		self.memento().update(self.key, &JsValue::from_serde(value).unwrap()).await;
	}

	fn memento(&self) -> vscode_sys::Memento {
		let getter = match self.scope {
			Scope::Global => vscode_sys::ExtensionContext::global_state,
			Scope::Workspace => vscode_sys::ExtensionContext::workspace_state,
		};
		crate::glue::EXTENSION_CONTEXT.with(|ext_ctx| {
			getter(ext_ctx.get().unwrap().unchecked_ref::<vscode_sys::ExtensionContext>())
		})
	}
}
