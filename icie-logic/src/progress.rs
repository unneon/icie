use crate::{
	error::{self, R}, Impulse, Reaction, ICIE
};

pub(crate) struct Progress<'a> {
	id: String,
	icie: &'a ICIE,
}
impl<'a> Progress<'a> {
	pub fn start<'b, 'c>(title: Option<&'b str>, id: &'c str, icie: &'a ICIE) -> R<Progress<'a>> {
		icie.send(Reaction::ProgressStart {
			id: String::from(id),
			title: title.map(String::from),
		});
		let progress = Progress { id: String::from(id), icie };
		progress.wait_for_ready()?;
		Ok(progress)
	}

	pub fn update(&self, increment: Option<f64>, message: Option<&str>) -> R<()> {
		self.icie.send(Reaction::ProgressUpdate {
			id: self.id.clone(),
			increment,
			message: message.map(String::from),
		});
		self.wait_for_ready()?;
		Ok(())
	}

	pub fn end(self) {
		self.icie.send(Reaction::ProgressEnd { id: self.id });
	}

	fn wait_for_ready(&self) -> R<()> {
		loop {
			match self.icie.recv() {
				Impulse::ProgressReady { ref id } if id == &self.id => break Ok(()),
				impulse => Err(error::unexpected(impulse))?,
			}
		}
	}
}
