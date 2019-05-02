pub fn rr(in_path: &str) -> evscode::R<()> {
	evscode::InfoMessage::new("#trigger icie.debug.rr").spawn();
	Ok(())
}
