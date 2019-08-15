use crate::{dir, init, manifest::Manifest, util};
use evscode::{quick_pick, QuickPick, E, R};

pub fn activate() -> R<()> {
	let _status = crate::STATUS.push("Launching");
	evscode::runtime::spawn(crate::newsletter::check);
	layout_setup();
	init::contest::check_for_manifest()?;
	Ok(())
}

pub fn layout_setup() {
	let _status = crate::STATUS.push("Opening files");
	if let (Ok(_), Ok(manifest), Ok(solution)) = (evscode::workspace_root(), Manifest::load(), dir::solution()) {
		evscode::open_editor(&solution).cursor(util::find_cursor_place(&solution)).view_column(1).open();
		if let Some(statement) = manifest.statement {
			let webview = evscode::Webview::new("icie.statement", "ICIE Statement", 2)
				.enable_scripts()
				.enable_find_widget()
				.retain_context_when_hidden()
				.preserve_focus()
				.create();
			webview.set_html(statement.html);
		}
	}
}

#[evscode::command(title = "ICIE Launch nearby", key = "alt+backspace")]
fn nearby() -> R<()> {
	let root = evscode::workspace_root()?;
	let parent = root.parent().ok_or_else(|| E::error("current directory has no parent"))?;
	let mut nearby = parent
		.read_dir()
		.map_err(|e| E::from_std(e).context("could not read parent directory"))?
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
