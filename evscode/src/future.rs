//! Temporary asynchronous programming model which will be later switched to [`std::future`].
//!
//! See description of the [async/await](https://en.wikipedia.org/wiki/Async/await) pattern.

use crate::{
	error::Cancellable, internal::executor::{ASYNC_ID_FACTORY, ASYNC_OPS}, R
};
use std::{
	any::Any, fmt::Debug, mem, sync::mpsc::{channel, Receiver, Sender}
};

/// Value representing a stream of asynchronous computations.
pub struct Future<T> {
	tx: Sender<Packet>,
	rx: Receiver<Packet>,
	parsers: Vec<(u64, Parser<T>)>,
}
impl<T: 'static> Future<T> {
	/// Synchronously wait for the computation to yield a value.
	pub fn wait(&self) -> T {
		let packet = self.rx.recv().expect("evscode::Future::wait RecvError");
		let parser = &self
			.parsers
			.iter()
			.find(|(aid, _)| *aid == packet.aid)
			.expect("evscode::Future::wait received message with unknown aid")
			.1;
		let obj = parser(packet);
		obj
	}

	/// Apply a function to the value after is will be computed.
	pub fn map<U: 'static>(mut self, f: impl 'static+Clone+Fn(T) -> U+Send) -> Future<U> {
		let (dummy_tx, dummy_rx) = channel();
		Future {
			tx: mem::replace(&mut self.tx, dummy_tx),
			rx: mem::replace(&mut self.rx, dummy_rx),
			parsers: mem::replace(&mut self.parsers, Vec::new())
				.into_iter()
				.map(|(aid, parser)| {
					let f2 = f.clone();
					let new_parser = move |raw: Packet| f2(parser(raw));
					let boxed = Box::new(new_parser);
					let nice_box: Parser<U> = boxed;
					(aid, nice_box)
				})
				.collect(),
		}
	}

	/// Return a future yielding all values from both arguments, whichever is faster
	pub fn join(mut self, other: LazyFuture<T>) -> Future<T> {
		let LazyFuture { spawner, parser } = other;
		let aid = ASYNC_ID_FACTORY.generate();
		ASYNC_OPS.lock().expect("evscode::Future::join ASYNC_OPS PoisonError").insert(aid, self.tx.clone());
		spawner(aid, &self.tx);
		self.parsers.push((aid, parser));
		self
	}

	/// Return a future yielding values from this future, unless the other future yields any value.
	/// In that case the future will yield a Result-like value representing a cancelled operation, that can be forwarder using the ? operator.
	pub fn cancel_on(self, other: LazyFuture<()>) -> Future<Cancellable<T>> {
		self.map(|x| Cancellable(Some(x))).join(other.map(|()| Cancellable(None)))
	}

	/// Create a future that will yield no values.
	pub fn new_empty() -> Future<T> {
		let (tx, rx) = channel();
		Future { tx, rx, parsers: Vec::new() }
	}
}
impl<T: 'static> Iterator for Future<Cancellable<T>> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.wait().0
	}
}
impl<T> Drop for Future<T> {
	fn drop(&mut self) {
		let mut lck = ASYNC_OPS.lock().expect("evscode::Future::drop ASYNC_OPS PoisonError");
		for (aid, _) in &self.parsers {
			lck.remove(aid);
		}
	}
}

/// Value representing a stream of asynchronous computations that have not been started yet.
///
/// Lazy futures must be used by one of the methods that returns a [`Future`] or T.
/// Otherwise, the computation will not be started at all.
/// This type exists to make the implementation of waiting for multiple futures simpler.
#[must_use]
pub struct LazyFuture<T> {
	spawner: Box<dyn FnOnce(u64, &Sender<Packet>)>,
	parser: Parser<T>,
}
impl<T: Send+'static> LazyFuture<T> {
	pub(crate) fn new_vscode(spawner: impl FnOnce(u64)+'static, parser: impl Fn(json::JsonValue) -> T+Send+'static) -> LazyFuture<T> {
		LazyFuture {
			spawner: Box::new(move |aid, _| spawner(aid)),
			parser: Box::new(move |raw| parser(raw.downcast::<json::JsonValue>().unwrap())),
		}
	}

	/// Spawn a worker thread that will receive a [`Carrier`] instance.
	/// The values sent using the [`Carrier::send`] will be returned from the future.
	/// If the worker returns an error, it will also be returned from the future.
	/// The worker is not actually spawned until the lazy future is changed to a normal [`Future`].
	pub fn new_worker(f: impl FnOnce(Carrier<T>) -> R<()>+Send+'static) -> LazyFuture<R<T>> {
		LazyFuture {
			spawner: Box::new(move |aid, tx| {
				let tx = tx.clone();
				let tx2 = tx.clone();
				std::thread::spawn(move || {
					let carrier = Carrier {
						aid,
						tx,
						_phantom: std::marker::PhantomData,
					};
					match f(carrier) {
						Ok(()) => (),
						Err(e) => {
							let e: R<T> = Err(e);
							if tx2.send(Packet::new(aid, e)).is_err() {
								log::warn!("dropped error in worker thread");
							}
						},
					}
				});
			}),
			parser: Box::new(move |raw: Packet| raw.downcast::<R<T>>().unwrap()),
		}
	}

	/// Spawn the asynchronous computation.
	pub fn spawn(self) -> Future<T> {
		let LazyFuture { spawner, parser } = self;
		let aid = ASYNC_ID_FACTORY.generate();
		let (tx, rx) = channel();
		ASYNC_OPS.lock().expect("evscode::LazyFuture::spawn ASYNC_OPS PoisonError").insert(aid, tx.clone());
		spawner(aid, &tx);
		Future {
			tx,
			rx,
			parsers: vec![(aid, parser)],
		}
	}

	/// Synchronously wait for the computation to yield a value.
	pub fn wait(self) -> T {
		self.spawn().wait()
	}

	/// Apply a function to the value after is will be computed.
	pub fn map<U>(self, f: impl 'static+Fn(T) -> U+Send) -> LazyFuture<U> {
		let parser = self.parser;
		LazyFuture {
			spawner: self.spawner,
			parser: Box::new(move |raw| f(parser(raw))),
		}
	}

	/// Return a future yielding all values from both arguments, whichever is faster
	pub fn join(self, other: LazyFuture<T>) -> Future<T> {
		self.spawn().join(other)
	}
}

/// This struct will be passed to the worker given in [`LazyFuture::new_worker`].
///
/// The values sent through this carrier will be yielded from the returned future.
pub struct Carrier<T: Send+'static> {
	aid: u64,
	tx: Sender<Packet>,
	_phantom: std::marker::PhantomData<T>,
}
impl<T: Any+Debug+Send+'static> Carrier<T> {
	/// Send the value back to the future returned from [`LazyFuture::new_worker`].
	pub fn send(&self, x: T) -> bool {
		let r: R<T> = Ok(x);
		self.tx.send(Packet::new(self.aid, r)).is_ok()
	}
}

#[doc(hidden)]
pub struct Packet {
	aid: u64,
	value: Box<dyn Any+Send>,
}
impl Packet {
	#[doc(hidden)]
	pub fn new<T: Any+Send>(aid: u64, x: T) -> Packet {
		Packet { aid, value: Box::new(x) }
	}

	#[doc(hidden)]
	pub fn downcast<T: 'static>(self) -> Option<T> {
		self.value.downcast::<T>().ok().map(|value| *value)
	}
}

#[doc(hidden)]
pub type Parser<T> = Box<dyn Fn(Packet) -> T+Send>;
