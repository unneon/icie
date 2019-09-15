use crate::telemetry::TELEMETRY;
use evscode::R;

pub async fn check() -> R<()> {
	let last = LAST_ACKNOWLEDGED_VERSION.get().await?;
	if last.as_ref().map(String::as_ref) != Some(LAST_IMPORTANT_UPDATE.version) {
		TELEMETRY.newsletter_show.spark();
		let message = format!(
			"Hey, ICIE {} has some cool new features, like: {}; check them out!",
			LAST_IMPORTANT_UPDATE.version, LAST_IMPORTANT_UPDATE.features
		);
		let choice =
			evscode::Message::new(&message).item("changelog".to_owned(), "See full changelog", false).item("ok".to_owned(), "Ok", false).show().await;
		if let Some(choice) = choice {
			if choice == "changelog" {
				TELEMETRY.newsletter_changelog.spark();
				evscode::open_external("https://github.com/pustaczek/icie/blob/master/CHANGELOG.md").await?;
			} else {
				TELEMETRY.newsletter_dismiss.spark();
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
