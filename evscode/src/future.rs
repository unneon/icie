use crate::internal::executor::{ASYNC_ID_FACTORY, VSCODE_FUTURES, VSCODE_STREAMS};
use futures::{
	channel::{mpsc, oneshot}, Stream
};
use json::JsonValue;
use std::{
	future::Future, pin::Pin, task::{Context, Poll}
};

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output=T>+Send+'a>>;

pub struct Pong {
	id: u64,
	rx: oneshot::Receiver<JsonValue>,
}

impl Pong {
	pub fn new() -> Pong {
		let id = ASYNC_ID_FACTORY.generate();
		let (tx, rx) = oneshot::channel();
		VSCODE_FUTURES.lock().unwrap().insert(id, tx);
		Pong { id, rx }
	}

	pub fn aid(&self) -> u64 {
		self.id
	}
}

pub struct PongStream {
	id: u64,
	rx: mpsc::UnboundedReceiver<JsonValue>,
}

impl PongStream {
	pub fn new() -> PongStream {
		let id = ASYNC_ID_FACTORY.generate();
		let (tx, rx) = mpsc::unbounded();
		VSCODE_STREAMS.lock().unwrap().insert(id, tx);
		PongStream { id, rx }
	}

	pub fn aid(&self) -> u64 {
		self.id
	}
}

impl Future for Pong {
	type Output = JsonValue;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
		Pin::new(&mut self.rx).poll(cx).map(Result::unwrap)
	}
}

impl Stream for PongStream {
	type Item = JsonValue;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		Pin::new(&mut self.rx).poll_next(cx)
	}
}

impl Drop for Pong {
	fn drop(&mut self) {
		VSCODE_FUTURES.lock().unwrap().remove(&self.id);
	}
}

impl Drop for PongStream {
	fn drop(&mut self) {
		VSCODE_STREAMS.lock().unwrap().remove(&self.id);
	}
}
