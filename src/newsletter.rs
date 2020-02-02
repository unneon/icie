use crate::telemetry::TELEMETRY;
use evscode::R;

pub async fn check() -> R<()> {
	let last = LAST_ACKNOWLEDGED_VERSION.get()?;
	if last.as_deref() != Some(LAST_IMPORTANT_UPDATE.version) {
		TELEMETRY.newsletter_show.spark();
		let message = format!(
			"Hey, ICIE {} has some cool new features, like: {}; check them out!",
			LAST_IMPORTANT_UPDATE.version, LAST_IMPORTANT_UPDATE.features
		);
		let choice = evscode::Message::new(&message).item((), "See changelog", false).show().await;
		let acknowledge = LAST_IMPORTANT_UPDATE.version.to_owned();
		LAST_ACKNOWLEDGED_VERSION.set(&acknowledge).await;
		if choice.is_some() {
			TELEMETRY.newsletter_changelog.spark();
			evscode::open_external("https://github.com/pustaczek/icie/blob/master/CHANGELOG.md")
				.await?;
		} else {
			TELEMETRY.newsletter_dismiss.spark();
		}
	}
	Ok(())
}

struct Update {
	version: &'static str,
	features: &'static str,
}

const LAST_IMPORTANT_UPDATE: Update =
	Update { version: "0.7", features: "Windows and macOS support" };

const LAST_ACKNOWLEDGED_VERSION: evscode::State<String> =
	evscode::State::new("icie.newsletter.lastAcknowledgedVersion", evscode::state::Scope::Global);
