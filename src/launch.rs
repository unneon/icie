use crate::{
	dir, init, manifest::Manifest, net::{interpret_url, require_task}, telemetry::{self, TELEMETRY}, util
};
use evscode::{error::ResultExt, quick_pick, stdlib::webview::WebviewMeta, QuickPick, E, R};
use futures::stream::StreamExt;
use std::time::Instant;
use unijudge::{Backend, Resource, Statement};

pub async fn activate() -> R<()> {
	let _status = crate::STATUS.push("Launching");
	*telemetry::START_TIME.lock().unwrap() = Some(Instant::now());
	evscode::spawn(crate::newsletter::check());
	layout_setup().await?;
	init::contest::check_for_manifest().await?;
	Ok(())
}

pub async fn deactivate() -> R<()> {
	telemetry::send_usage();
	Ok(())
}

pub async fn layout_setup() -> R<()> {
	let _status = crate::STATUS.push("Opening");
	if let (Ok(_), Ok(manifest), Ok(solution)) = (evscode::workspace_root(), Manifest::load().await, dir::solution()) {
		evscode::open_editor(&solution).cursor(util::find_cursor_place(&solution).await).view_column(1).open().await;
		if manifest.statement.is_some() {
			statement().await?;
		}
		// refocus the cursor, because apparently preserve_focus is useless
		evscode::open_editor(&solution).cursor(util::find_cursor_place(&solution).await).view_column(1).open().await;
	}
	Ok(())
}

async fn display_pdf(mut webview: WebviewMeta, pdf: &[u8]) {
	let _status = crate::STATUS.push("Rendering PDF");
	TELEMETRY.statement_pdf.spark();
	webview.webview.set_html(&format!(
		"<html><head><script src=\"{}\"></script><script>{}</script></head><body id=\"body\" style=\"padding: 0;\"></body></html>",
		evscode::asset("pdf-2.2.228.min.js"),
		include_str!("pdf.js")
	));
	// This webview script sends a message indicating that it is ready to receive messages. See [`evscode::Webview::post_message`] docs.
	webview.listener.next().await;
	webview.webview.post_message(evscode::json::object! {
		"pdf_data_base64" => pdf,
	});
}

#[evscode::command(title = "ICIE Statement", key = "alt+8")]
async fn statement() -> R<()> {
	TELEMETRY.statement.spark();
	let manifest = Manifest::load().await?;
	let statement = manifest.req_statement()?;
	let webview = evscode::Webview::new("icie.statement", "ICIE Statement", 2)
		.enable_scripts()
		.enable_find_widget()
		.retain_context_when_hidden()
		.preserve_focus()
		.create();
	match statement {
		Statement::HTML { html } => {
			TELEMETRY.statement_html.spark();
			webview.webview.set_html(html)
		},
		Statement::PDF { pdf } => {
			let pdf = pdf.clone();
			evscode::spawn(async move {
				display_pdf(webview, &pdf).await;
				Ok(())
			})
		},
	}
	Ok(())
}

#[evscode::command(title = "ICIE Launch nearby", key = "alt+backspace")]
async fn nearby() -> R<()> {
	TELEMETRY.launch_nearby.spark();
	let root = evscode::workspace_root()?;
	let parent = root.parent().wrap("current directory has no parent")?;
	let mut nearby = parent
		.read_dir()
		.wrap("could not read parent directory")?
		.filter_map(|entry| {
			let entry = entry.ok()?;
			if entry.file_type().ok()?.is_dir() { Some(entry) } else { None }
		})
		.map(|entry| {
			let path = entry.path();
			let title = match path.strip_prefix(parent) {
				Ok(rel) => rel.to_str().unwrap(),
				Err(_) => path.to_str().unwrap(),
			}
			.to_owned();
			(path, title)
		})
		.collect::<Vec<_>>();
	nearby.sort_by_key(|nearby| nearby.1.clone());
	let select = QuickPick::new()
		.items(nearby.into_iter().map(|nearby| quick_pick::Item::new(nearby.0.to_str().unwrap().to_owned(), nearby.1)))
		.show()
		.await
		.ok_or_else(E::cancel)?;
	evscode::open_folder(select, false);
	Ok(())
}

#[evscode::command(title = "ICIE Web Task")]
async fn web_task() -> R<()> {
	TELEMETRY.launch_web_task.spark();
	let manifest = Manifest::load().await?;
	evscode::open_external(manifest.req_task_url()?).await?;
	Ok(())
}

#[evscode::command(title = "ICIE Web Contest")]
async fn web_contest() -> R<()> {
	TELEMETRY.launch_web_contest.spark();
	let manifest = Manifest::load().await?;
	let (url, backend) = interpret_url(manifest.req_task_url()?)?;
	let Resource::Task(task) = require_task(url)?.resource;
	let url = backend.backend.contest_url(&backend.backend.task_contest(&task).wrap("task is not attached to any contest")?);
	evscode::open_external(&url).await?;
	Ok(())
}
