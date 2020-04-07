pub mod manage;
mod render;

use crate::telemetry::TELEMETRY;
use evscode::R;

#[evscode::command(title = "ICIE Stress", key = "alt+9")]
async fn open() -> R<()> {
	TELEMETRY.stress_start.spark();
	let webview = manage::WEBVIEW.get_lazy(()).await?;
	webview.reveal(1, false);
	Ok(())
}
