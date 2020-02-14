use crate::{
	dir, init, logger, manifest::Manifest, net::{interpret_url, require_task}, telemetry::TELEMETRY, util::{self, fs, path::Path}
};
use evscode::{error::ResultExt, quick_pick, webview::WebviewMeta, QuickPick, E, R};
use futures::StreamExt;
use serde::Serialize;
use unijudge::{Backend, Resource, Statement};

pub async fn activate() -> R<()> {
	let _status = crate::STATUS.push("Launching");
	logger::initialize()?;
	evscode::spawn(crate::newsletter::check());
	layout_setup().await?;
	init::contest::check_for_manifest().await?;
	Ok(())
}

pub async fn deactivate() -> R<()> {
	// Deactivate does not really seem to work at all? Issue [47881][1] suggest that also most JS
	// APIs break when this happens.
	// [1]: https://github.com/Microsoft/vscode/issues/47881
	Ok(())
}

pub async fn layout_setup() -> R<()> {
	let _status = crate::STATUS.push("Opening");
	if let Ok(manifest) = Manifest::load().await {
		if let Ok(solution) = dir::solution() {
			let _ = evscode::open_editor(&solution)
				.cursor(util::find_cursor_place(&solution).await)
				.view_column(1)
				.open()
				.await;
		}
		if manifest.statement.is_some() {
			statement().await?;
		}
		if let Ok(solution) = dir::solution() {
			// refocus the cursor, because apparently preserve_focus is useless
			let _ = evscode::open_editor(&solution)
				.cursor(util::find_cursor_place(&solution).await)
				.view_column(1)
				.open()
				.await;
		}
	}
	Ok(())
}

async fn display_pdf(mut webview: WebviewMeta, pdf: &[u8]) {
	let _status = crate::STATUS.push("Rendering PDF");
	TELEMETRY.statement_pdf.spark();
	webview.webview.set_html(&format!(
		"<html><head><script src=\"{}\"></script><script>{}</script></head><body id=\"body\" \
		 style=\"padding: 0;\"></body></html>",
		evscode::asset("pdf-2.2.228.min.js"),
		include_str!("pdf.js")
	));
	// This webview script sends a message indicating that it is ready to receive messages. See
	// [`evscode::Webview::post_message`] docs.
	webview.listener.next().await;
	webview.webview.post_message(StatementData { pdf_data_base64: pdf }).await;
}

#[derive(Serialize)]
struct StatementData<'a> {
	pdf_data_base64: &'a [u8],
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
	let root = Path::from_native(evscode::workspace_root()?);
	let parent = root.parent();
	let mut nearby = fs::read_dir(&parent)
		.await?
		.into_iter()
		.map(|path| {
			let title = match path.strip_prefix(&parent) {
				Ok(rel) => rel.to_str().unwrap().to_owned(),
				Err(_) => path.to_str().unwrap().to_owned(),
			};
			(path, title)
		})
		.collect::<Vec<_>>();
	nearby.sort_by_key(|nearby| nearby.1.clone());
	let select = QuickPick::new()
		.items(
			nearby.into_iter().map(|nearby| quick_pick::Item::new(nearby.0.to_string(), nearby.1)),
		)
		.show()
		.await
		.ok_or_else(E::cancel)?;
	evscode::open_folder(&select, false).await;
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
	let url = backend.backend.contest_url(
		&backend.backend.task_contest(&task).wrap("task is not attached to any contest")?,
	);
	evscode::open_external(&url).await?;
	Ok(())
}
