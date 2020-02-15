use crate::{
	dir, util::{letter_case::Case, path::Path}
};
use evscode::{E, R};
use unijudge::TaskDetails;

pub async fn design_task_name(root: &Path, meta: Option<&TaskDetails>) -> R<Path> {
	Ok(match meta {
		Some(meta) => root.join(&format!(
			"{}-{}",
			Case::Upper.apply(&meta.id),
			Case::Kebab.apply(&meta.title)
		)),
		None => query(root).await?,
	})
}

pub async fn design_contest_name(contest_title: &str) -> R<Path> {
	Ok(dir::PROJECT_DIRECTORY.get().join(&Case::Kebab.apply(contest_title)))
}

async fn query(basic: &Path) -> R<Path> {
	Ok(Path::from_native(
		evscode::InputBox::new()
			.ignore_focus_out()
			.prompt("New project directory")
			.value(basic.to_str().unwrap())
			.value_selection(basic.to_str().unwrap().len(), basic.to_str().unwrap().len())
			.show()
			.await
			.ok_or_else(E::cancel)?,
	))
}
