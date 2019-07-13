//! Associative webview collection that represents a set of computation results.

use crate::{goodies::WebviewHandle, Webview, R};
use std::{
	collections::HashMap, hash::Hash, sync::{Arc, Mutex}
};

/// Trait controlling the webview collection behaviour.
pub trait Computation: Sync {
	/// Key type provided to the computation.
	type K: Eq+Hash+Clone+Send;
	/// Computation results intended to be displayed.
	type V;
	/// Run the computation.
	fn compute(&self, key: &Self::K) -> R<Self::V>;
	/// Create an empty webview for the results of a computation with the given key.
	fn create_empty_webview(&self, key: &Self::K) -> R<Webview>;
	/// Update a webview with given computation results.
	fn update(&self, key: &Self::K, value: &Self::V, webview: &Webview) -> R<()>;
	/// Return a worker function which will handle the messages received from the webview.
	fn manage(&self, key: &Self::K, value: &Self::V, handle: WebviewHandle) -> R<Box<dyn FnOnce()+Send+'static>>;
}

/// State of the webview collection.
pub struct WebviewResultmap<T: Computation> {
	computation: T,
	collection: Mutex<HashMap<T::K, WebviewHandle>>,
}

impl<T: Computation> WebviewResultmap<T> {
	/// Create a new instance of the webview collection.
	pub fn new(computation: T) -> WebviewResultmap<T> {
		WebviewResultmap { computation, collection: Mutex::new(HashMap::new()) }
	}

	/// Run the computation, update the view and return both the webview and the computed values.
	pub fn get_force(&'static self, key: T::K) -> R<(WebviewHandle, T::V)> {
		let (handle, value) = self.raw_get(key, true)?;
		Ok((handle, value.unwrap()))
	}

	/// Run the computation and create the view if it does not already exist.
	/// Return the associated webview.
	pub fn get_lazy(&'static self, key: T::K) -> R<WebviewHandle> {
		Ok(self.raw_get(key, false)?.0)
	}

	/// Select the webview that is currently active.
	pub fn find_active(&self) -> Option<WebviewHandle> {
		let lck = self.collection.lock().unwrap();
		for webview in lck.values() {
			if webview.lock().unwrap().is_active().wait() {
				return Some(webview.clone());
			}
		}
		None
	}

	/// Rerun the computation on all existing webviews and update them.
	pub fn update_all(&'static self) {
		let lck = self.collection.lock().unwrap();
		for k in lck.keys() {
			let k = k.clone();
			crate::runtime::spawn(move || {
				self.get_force(k)?;
				Ok(())
			});
		}
	}

	fn raw_get(&'static self, key: T::K, force: bool) -> R<(WebviewHandle, Option<T::V>)> {
		let mut collection = self.collection.lock().unwrap();
		let (webview, value) = match collection.entry(key.clone()) {
			std::collections::hash_map::Entry::Vacant(e) => {
				let (handle, value) = self.make_new(&key)?;
				e.insert(handle.clone());
				(handle, Some(value))
			},
			std::collections::hash_map::Entry::Occupied(e) => {
				if force {
					let handle = e.get().lock().unwrap();
					let value = self.update_old(&key, &*handle)?;
					(e.get().clone(), Some(value))
				} else {
					(e.get().clone(), None)
				}
			},
		};
		let webview_lock = webview.lock().unwrap();
		drop(collection);
		drop(webview_lock);
		Ok((webview, value))
	}

	fn make_new(&'static self, key: &T::K) -> R<(WebviewHandle, T::V)> {
		let value = self.computation.compute(key)?;
		let webview = self.computation.create_empty_webview(&key)?;
		self.computation.update(key, &value, &webview)?;
		let webview = Arc::new(Mutex::new(webview));
		let worker = self.computation.manage(key, &value, webview.clone())?;
		let key = key.clone();
		let handle = webview.clone();
		crate::runtime::spawn(move || {
			worker();
			let mut collection = self.collection.lock().unwrap();
			match collection.entry(key) {
				std::collections::hash_map::Entry::Occupied(e) => {
					if Arc::ptr_eq(e.get(), &handle) {
						e.remove_entry();
					}
				},
				_ => (),
			}
			Ok(())
		});
		Ok((webview, value))
	}

	fn update_old(&'static self, key: &T::K, webview: &Webview) -> R<T::V> {
		let value = self.computation.compute(key)?;
		self.computation.update(key, &value, webview)?;
		Ok(value)
	}
}
