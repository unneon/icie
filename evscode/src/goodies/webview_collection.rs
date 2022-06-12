//! Associative webview collection that represents a set of computation results.

use crate::{
	webview::{Disposer, Listener, WebviewMeta, WebviewRef}, Webview, R
};
use async_trait::async_trait;
use futures::{future::join_all, lock::Mutex};
use std::{collections::HashMap, future::Future, hash::Hash, ops::Deref};

/// Trait controlling the webview collection behaviour.
#[async_trait]
pub trait Behaviour: Send+Sync {
	/// Key type provided to the computation.
	type K: Eq+Hash+Clone+Send+Sync;
	/// Computation results intended to be displayed.
	type V: Send+Sync;
	/// Create an empty webview for the results of a computation with the given key.
	fn create_empty(&self, key: Self::K) -> R<WebviewMeta>;
	/// Run the computation.
	async fn compute(&self, key: Self::K) -> R<Self::V>;
	/// Update a webview with given computation results.
	async fn update(&self, key: Self::K, value: &Self::V, webview: WebviewRef) -> R<()>;
	/// Return a worker function which will handle the messages received from the webview.
	async fn manage(&self, key: Self::K, webview: WebviewRef, listener: Listener, disposer: Disposer) -> R<()>;
}

/// State of the webview collection.
pub struct Collection<T: Behaviour> {
	computation: Mutex<T>,
	collection: Mutex<HashMap<T::K, Webview>>,
}



impl<T: Behaviour> Collection<T> {
	/// Create a new instance of the webview collection.
	pub fn new(computation: T) -> Collection<T> {
		Collection { computation:Mutex::new(computation), collection: Mutex::new(HashMap::new()) }
	}

	/// Run the computation, update the view and return both the webview and the computed values.
	pub async fn get_force(&'static self, key: T::K) -> R<(WebviewRef, T::V)> {
		let (handle, value) = self.raw_get(key, true).await?;
		Ok((handle, value.unwrap()))
	}

	/// Run the computation and create the view if it does not already exist.
	/// Return the associated webview.
	pub async fn get_lazy(&'static self, key: T::K) -> R<WebviewRef> {
		Ok(self.raw_get(key, false).await?.0)
	}

	/// Select the webview that is currently active.
	pub async fn find_active(&self) -> Option<WebviewRef> {
		let lck = self.collection.lock().await;
		for webview in lck.values() {
			if webview.is_active() {
				return Some(webview.deref().clone());
			}
		}
		None
	}

	/// Selects the active webview, or a specified one if no webview is active.
	pub async fn active_or_lazy(&'static self, key: T::K) -> R<WebviewRef> {
		match self.find_active().await {
			Some(active) => Ok(active),
			None => self.get_lazy(key).await,
		}
	}

	/// Rerun the computation on all existing webviews and update them.
	pub async fn update_all(&'static self) -> R<()> {
		let lck = self.collection.lock().await;
		let to_update = join_all(lck.keys().map(|k| self.get_force(k.clone())));
		drop(lck);
		for r in to_update.await {
			r?;
		}
		Ok(())
	}

	async fn raw_get(&'static self, key: T::K, force: bool) -> R<(WebviewRef, Option<T::V>)> {
		let mut collection = self.collection.lock().await;
		Ok(match collection.entry(key.clone()) {
			std::collections::hash_map::Entry::Vacant(e) => {
				let (webview, value) = self.make_new(&key).await?;
				let handle = webview.deref().clone();
				e.insert(webview);
				(handle, Some(value))
			},
			std::collections::hash_map::Entry::Occupied(e) => {
				if force {
					let webview = e.get();
					let value = self.update_old(&key, webview).await?;
					(webview.deref().clone(), Some(value))
				} else {
					(e.get().deref().clone(), None)
				}
			},
		})
	}

	async fn make_new(&'static self, key: &T::K) -> R<(Webview, T::V)> {
		let comp=self.computation.lock().await;
		let value = comp.compute(key.clone()).await?;
		let WebviewMeta { webview, listener, disposer } = comp.create_empty(key.clone())?;
		comp.update(key.clone(), &value, webview.deref().clone()).await?;
		let worker = comp.manage(key.clone(), webview.deref().clone(), listener, disposer);
		let key = key.clone();
		let handle = webview.clone();
		crate::spawn(async move {
			let resultmap: &'static Collection<T> = self;
			let delayed_error = worker.await;
			let mut collection = resultmap.collection.lock().await;
			if let std::collections::hash_map::Entry::Occupied(e) = collection.entry(key) {
				// TODO: Is this the right webview?
				e.remove_entry();
			}
			handle.dispose();
			delayed_error
		});
		Ok((webview, value))
	}

	fn update_old<'a>(&'static self, key: &'a T::K, webview: &'a Webview) -> impl Future<Output=R<T::V>>+'a {
		async move {
			let mut comp=self.computation.lock().await;
			let value = comp.compute(key.clone()).await?;
			comp.update(key.clone(), &value, webview.deref().clone()).await?;
			Ok(value)
		}
	}
}
