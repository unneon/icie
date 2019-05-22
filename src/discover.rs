mod comms;
mod manage;
mod render;

#[evscode::command(title = "ICIE Discover", key = "alt+9")]
fn open() -> evscode::R<()> {
	let handle = manage::WEBVIEW.handle()?;
	let lck = handle.lock()?;
	lck.reveal(1);
	Ok(())
}
