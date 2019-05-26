use evscode::R;

pub fn activate() -> R<()> {
	let _status = crate::STATUS.push("Launching");
	if evscode::workspace_root().is_ok() {
		let solution = crate::dir::solution()?;
		if solution.exists() {
			crate::util::nice_open_editor(solution)?;
		}
	}
	Ok(())
}
