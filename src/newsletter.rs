use evscode::R;

pub fn check() -> R<()> {
	let last = LAST_ACKNOWLEDGED_VERSION.get().wait()?;
	if last.as_ref().map(String::as_ref) != Some(LAST_IMPORTANT_UPDATE.version) {
		let choice = evscode::Message::new(format!(
			"Hey, ICIE {} has some cool new features, like: {}; check them out!",
			LAST_IMPORTANT_UPDATE.version, LAST_IMPORTANT_UPDATE.features
		))
		.item("changelog", "See full changelog", false)
		.item("ok", "Ok", false)
		.build()
		.wait();
		if let Some(choice) = choice {
			if choice == "changelog" {
				evscode::open_external("https://github.com/pustaczek/icie/blob/master/CHANGELOG.md").wait()?;
			} else {
				LAST_ACKNOWLEDGED_VERSION.set(&LAST_IMPORTANT_UPDATE.version.to_owned());
			}
		}
	}
	Ok(())
}

struct Update {
	version: &'static str,
	features: &'static str,
}

const LAST_IMPORTANT_UPDATE: Update =
	Update { version: "0.6.2", features: "CodeChef support, reopening statements with Alt+8, shortcuts to contest/task websites" };

const LAST_ACKNOWLEDGED_VERSION: evscode::State<String> =
	evscode::State::new("icie.newsletter.lastAcknowledgedVersion", evscode::state::Scope::Global);
