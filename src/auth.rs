use crate::{telemetry::TELEMETRY, util::is_installed};
use evscode::{E, R};

pub async fn get_force_ask(site: &str) -> R<(String, String)> {
	TELEMETRY.auth_ask.spark();
	let message = format!("Username at {}", site);
	let username = evscode::InputBox::new().prompt(&message).ignore_focus_out().show().await.ok_or_else(E::cancel)?;
	let message = format!("Password for {} at {}", username, site);
	let password = evscode::InputBox::new().prompt(&message).password().ignore_focus_out().show().await.ok_or_else(E::cancel)?;
	let kr = Keyring::new("credentials", site);
	if !kr.set(
		&json::object! {
			"username" => username.as_str(),
			"password" => password.as_str(),
		}
		.dump(),
	) {
		E::error("failed to save password to a secure keyring, so it will not be remembered")
			.warning()
			.action_if(is_installed("kwalletd5")?, "How to fix (KWallet)", help_fix_kwallet())
			.emit();
	}
	Ok((username, password))
}

pub async fn get_cached_or_ask(site: &str) -> R<(String, String)> {
	let kr = Keyring::new("credentials", site);
	match kr.get() {
		Some(encoded) => {
			let creds = json::parse(&encoded).unwrap();
			Ok((creds["username"].as_str().unwrap().to_owned(), creds["password"].as_str().unwrap().to_owned()))
		},
		None => get_force_ask(site).await,
	}
}

pub fn get_if_cached(site: &str) -> Option<String> {
	Keyring::new("session", site).get()
}

pub fn save_cache(site: &str, value: &str) {
	Keyring::new("session", site).set(value); // ignore save fail
}

pub fn has_any_saved(site: &str) -> bool {
	Keyring::new("session", site).get().is_some() || Keyring::new("credentials", site).get().is_some()
}

#[evscode::command(title = "ICIE Password reset")]
async fn reset() -> R<()> {
	TELEMETRY.auth_reset.spark();
	let url = evscode::InputBox::new()
		.prompt("Enter any contest/task URL from the site for which you want to reset the password")
		.placeholder("https://codeforces.com/contest/.../problem/...")
		.ignore_focus_out()
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let site = crate::net::interpret_url(&url)?.0.site;
	Keyring::new("credentials", &site).delete();
	Keyring::new("session", &site).delete();
	Ok(())
}

async fn help_fix_kwallet() -> R<()> {
	evscode::open_external("https://github.com/pustaczek/icie/issues/14#issuecomment-516982482").await
}

struct Keyring {
	kind: &'static str,
	site: String,
}
impl Keyring {
	fn new(kind: &'static str, site: &str) -> Keyring {
		Keyring { kind, site: site.to_owned() }
	}

	fn get(&self) -> Option<String> {
		let entry = format!("@{} {}", self.kind, self.site);
		let kr = keyring::Keyring::new("icie", &entry);
		match kr.get_password() {
			Ok(value) => Some(value),
			Err(keyring::KeyringError::NoPasswordFound) => None,
			Err(e) => {
				TELEMETRY.auth_keyring_error.spark();
				log::error!("keyring errored, details: {:#?}", e);
				None
			},
		}
	}

	fn set(&self, value: &str) -> bool {
		let entry = format!("@{} {}", self.kind, self.site);
		let kr = keyring::Keyring::new("icie", &entry);
		if let Err(e) = kr.set_password(value) {
			TELEMETRY.auth_keyring_error.spark();
			log::error!("keyring errored, details: {:#?}", e);
			false
		} else {
			true
		}
	}

	fn delete(&self) {
		let entry = format!("@{} {}", self.kind, self.site);
		let kr = keyring::Keyring::new("icie", &entry);
		if let Err(e) = kr.delete_password() {
			TELEMETRY.auth_keyring_error.spark();
			log::error!("keyring errored, details: {:#?}", e);
		}
	}
}
