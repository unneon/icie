use crate::util::{fs, path::Path};
use evscode::{E, R};
use futures::channel::oneshot;
use std::{
	future::Future, pin::Pin, time::{Duration, SystemTime}
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

pub async fn read_dir(path: &Path) -> R<Vec<Path>> {
	let (tx, rx) = make_callback2();
	node_sys::fs::readdir(
		path.as_str(),
		node_sys::fs::ReaddirOptions { encoding: Some("utf8"), with_file_types: None },
		tx,
	);
	Ok(rx
		.await?
		.dyn_into::<js_sys::Array>()
		.unwrap()
		.values()
		.into_iter()
		.map(|file| path.join(file.unwrap().as_string().unwrap()))
		.collect())
}
#[derive(Debug, Deserialize)]
pub struct Dirent {
	pub name: String,
	pub ftype: u8,
}
pub async fn is_directory(path: Path) -> R<bool> {
	let (tx, rx) = make_callback2();
	node_sys::fs::stat(path.as_str(), node_sys::fs::StatOptions { bigint: false }, tx);
	let stat = rx.await?;
	let nlink = js_sys::Reflect::get(&stat, &JsValue::from_str("nlink"))
		.map_err(|_| E::error("javascript file stats object has no nlink "))?
		.as_f64()
		.unwrap();
	//let modified = SystemTime::UNIX_EPOCH + Duration::from_millis(mtime_ms as u64);
	if nlink>1.0{
		Ok(true)
	}
	else{
		Ok(false)
	}
}
pub async fn find_a_dir(path: &Path) -> R<Path> {
	let lists=read_dir(path).await?;
	for dir in lists.into_iter() {
        if is_directory(dir.clone()).await?==true && exists(&dir.clone().join(".icie")).await?{
			return Ok(dir);
		}
    }
	return Err(E::cancel());
}

pub async fn read_to_string(path: &Path) -> R<String> {
	let (tx, rx) = make_callback2();
	node_sys::fs::read_file(path.as_str(), node_sys::fs::ReadFileOptions { encoding: Some("utf-8"), flag: "r" }, tx);
	Ok(rx.await?.as_string().unwrap())
}

pub async fn write(path: &Path, content: impl AsRef<[u8]>) -> R<()> {
	let (tx, rx) = make_callback1();
	let js_buffer = node_sys::buffer::Buffer::from(js_sys::Uint8Array::from(content.as_ref()));
	node_sys::fs::write_file(
		path.as_str(),
		js_buffer,
		node_sys::fs::WriteFileOptions { encoding: None, mode: None, flag: None },
		tx,
	);
	rx.await?;
	Ok(())
}

pub async fn remove_file(path: &Path) -> R<()> {
	let (tx, rx) = make_callback1();
	node_sys::fs::unlink(path.as_str(), tx);
	rx.await?;
	Ok(())
}

pub fn remove_file_sync(path: &Path) -> R<()> {
	node_sys::fs::unlink_sync(path.as_str());
	Ok(())
}

pub async fn create_dir(path: &Path) -> R<()> {
	let (tx, rx) = make_callback1();
	node_sys::fs::mkdir(path.as_str(), node_sys::fs::MkdirOptions { mode: None }, tx);
	rx.await?;
	Ok(())
}

pub async fn create_dir_all(path: &Path) -> R<()> {
	// This routine must be implemented manually because {recursive:true} is only supported on Node
	// 12. TODO: Does not check if path actually is a directory.
	if !fs::exists(path).await? {
		fs::create_dir_all_boxed(&path.parent()).await?;
		fs::create_dir(path).await?;
	}
	Ok(())
}

fn create_dir_all_boxed<'a>(path: &'a Path) -> Pin<Box<dyn Future<Output=R<()>>+'a>> {
	Box::pin(create_dir_all(path))
}

pub async fn exists(path: &Path) -> R<bool> {
	let (tx, rx) = make_callback1();
	node_sys::fs::access(path.as_str(), tx);
	Ok(rx.await.is_ok())
}

pub struct Metadata {
	pub modified: SystemTime,
}

pub async fn metadata(path: &Path) -> R<Metadata> {
	let (tx, rx) = make_callback2();
	node_sys::fs::stat(path.as_str(), node_sys::fs::StatOptions { bigint: false }, tx);
	let stat = rx.await?;
	let mtime_ms = js_sys::Reflect::get(&stat, &JsValue::from_str("mtimeMs"))
		.map_err(|_| E::error("javascript file stats object has no modification time"))?
		.as_f64()
		.unwrap();
	let modified = SystemTime::UNIX_EPOCH + Duration::from_millis(mtime_ms as u64);
	Ok(Metadata { modified })
}

fn make_callback1() -> (JsValue, impl Future<Output=Result<(), js_sys::Error>>) {
	let (tx, rx) = oneshot::channel();
	let closure = Closure::once_into_js(move |err: JsValue| {
		let _ = tx.send(match err.dyn_into::<js_sys::Error>() {
			Ok(err) => Err(err),
			Err(_) => Ok(()),
		});
	});
	let completion = async move { rx.await.unwrap() };
	(closure, completion)
}

pub fn make_callback2() -> (JsValue, impl Future<Output=Result<JsValue, js_sys::Error>>) {
	let (tx, rx) = oneshot::channel();
	let closure = Closure::once_into_js(move |err: JsValue, value: JsValue| {
		let _ = tx.send(match err.dyn_into::<js_sys::Error>() {
			Ok(err) => Err(err),
			Err(_) => Ok(value),
		});
	});
	let completion = async move { rx.await.unwrap() };
	(closure, completion)
}
