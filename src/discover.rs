mod comms;
pub mod manage;
mod render;

use evscode::R;

#[evscode::command(title = "ICIE Discover", key = "alt+9")]
fn open() -> R<()> {
	let handle = manage::WEBVIEW.handle()?;
	let lck = handle.lock().unwrap();
	lck.reveal(1);
	Ok(())
}
