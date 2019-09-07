use crate::{
	dir, init, manifest::Manifest, net::{interpret_url, require_task}, util
};
use evscode::{error::ResultExt, quick_pick, QuickPick, Webview, E, R};
use unijudge::{Backend, Resource, Statement};

pub fn activate() -> R<()> {
	let _status = crate::STATUS.push("Launching");
	evscode::runtime::spawn(crate::newsletter::check);
	layout_setup()?;
	init::contest::check_for_manifest()?;
	Ok(())
}

pub fn layout_setup() -> R<()> {
	let _status = crate::STATUS.push("Opening files");
	if let (Ok(_), Ok(manifest), Ok(solution)) = (evscode::workspace_root(), Manifest::load(), dir::solution()) {
		evscode::open_editor(&solution).cursor(util::find_cursor_place(&solution)).view_column(1).open().wait();
		if manifest.statement.is_some() {
			statement()?;
		}
		// refocus the cursor, because apparently preserve_focus is useless
		evscode::open_editor(&solution).cursor(util::find_cursor_place(&solution)).view_column(1).open().wait();
	}
	Ok(())
}

fn display_pdf(webview: Webview, pdf: &[u8]) {
	let pdf = pdf.to_owned();
	evscode::runtime::spawn(move || {
		let _status = crate::STATUS.push("Rendering PDF");
		webview.set_html(format!(
			"<html><head><script src=\"{}\"></script><script>{}</script></head><body id=\"body\" style=\"padding: 0;\"></body></html>",
			evscode::asset("pdf-2.2.228.min.js"),
			include_str!("pdf.js")
		));
		// This webview script sends a message indicating that it is ready to receive messages. See [`evscode::Webview::post_message`] docs.
		webview.listener().wait();
		webview.post_message(evscode::json::object! {
			"pdf_data_base64" => pdf,
		});
		Ok(())
	});
}

#[evscode::command(title = "ICIE Statement", key = "alt+8")]
fn statement() -> R<()> {
	let manifest = Manifest::load()?;
	let statement = manifest.req_statement()?;
	let webview = evscode::Webview::new("icie.statement", "ICIE Statement", 2)
		.enable_scripts()
		.enable_find_widget()
		.retain_context_when_hidden()
		.preserve_focus()
		.create();
	match statement {
		Statement::HTML { html } => webview.set_html(html),
		Statement::PDF { pdf } => display_pdf(webview, pdf),
	}
	Ok(())
}

#[evscode::command(title = "ICIE Launch nearby", key = "alt+backspace")]
fn nearby() -> R<()> {
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
		.items(nearby.into_iter().map(|nearby| quick_pick::Item::new(nearby.0.to_str().unwrap(), nearby.1)))
		.build()
		.wait()
		.ok_or_else(E::cancel)?;
	evscode::open_folder(select, false);
	Ok(())
}

#[evscode::command(title = "ICIE Web Task")]
fn web_task() -> R<()> {
	let manifest = Manifest::load()?;
	evscode::open_external(manifest.req_task_url()?).wait()?;
	Ok(())
}

#[evscode::command(title = "ICIE Web Contest")]
fn web_contest() -> R<()> {
	let manifest = Manifest::load()?;
	let (url, backend) = interpret_url(manifest.req_task_url()?)?;
	let Resource::Task(task) = require_task(url)?.resource;
	let url = backend.backend.contest_url(&backend.backend.task_contest(&task).wrap("task is not attached to any contest")?);
	evscode::open_external(url).wait()?;
	Ok(())
}
