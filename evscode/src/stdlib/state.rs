//! Workspace-local and global state
//!
//! Create a global const state object and query it using [`State::get`] and [`State::set`] methods.
//! ```
//! # use evscode::{Scope, State};
//! const DEBUGGER_LAUNCH_COUNT: State<i64> = State::new("debugger-launch-count", Scope::Global);
//! ```

use crate::{internal::executor::send_object, marshal::Marshal, LazyFuture, E, R};
use std::marker::PhantomData;

/// Scope of the stored values.
pub enum Scope {
	/// State with workspace scope will be different for every VS Code workspace.
	Workspace,
	/// State with global scope will be the same for every VS Code workspace.
	Global,
}

/// Object used to store storage entry metadata information.
pub struct State<T: Marshal+Send> {
	key: &'static str,
	scope: Scope,
	_phantom: PhantomData<T>,
}

impl<T: Marshal+Send> State<T> {
	/// Create a new storage entry with a given identifier and scope.
	/// Multiple storage objects with the same key will refer to the same entry.
	pub const fn new(key: &'static str, scope: Scope) -> State<T> {
		State {
			key,
			scope,
			_phantom: PhantomData,
		}
	}

	/// Query the stored value, if it was ever saved.
	pub fn get(&'static self) -> LazyFuture<R<Option<T>>> {
		LazyFuture::new_vscode(
			move |aid| {
				send_object(json::object! {
					"tag" => "reaction_memento_get",
					"aid" => aid,
					"key" => self.key,
					"dst" => match self.scope {
						Scope::Workspace => "workspace",
						Scope::Global => "global",
					},
				})
			},
			|mut raw| {
				let found = raw["found"].as_bool().unwrap();
				let value = raw["value"].take();
				if found {
					match T::from_json(value) {
						Ok(obj) => Ok(Some(obj)),
						Err(e) => Err(E::error(e).context("internal extension type error")),
					}
				} else {
					Ok(None)
				}
			},
		)
	}

	/// Set the storage entry to a given value.
	pub fn set(&'static self, value: &T) {
		send_object(json::object! {
			"tag" => "reaction_memento_set",
			"key" => self.key,
			"val" => value.to_json(),
			"dst" => match self.scope {
				Scope::Workspace => "workspace",
				Scope::Global => "global",
			},
		});
	}
}
