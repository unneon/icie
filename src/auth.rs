use crate::{telemetry::TELEMETRY, util::is_installed};
use evscode::{E, R};
use wasm_bindgen_futures::JsFuture;

// TODO: check how errors work w/o libsecret/gnome-keyring

#[derive(serde::Deserialize, serde::Serialize)]
struct Credentials {
	username: String,
	password: String,
}

pub async fn get_force_ask(site: &str) -> R<(String, String)> {
	TELEMETRY.auth_ask.spark();
	let message = format!("Username at {}", site);
	let username = evscode::InputBox::new()
		.prompt(&message)
		.ignore_focus_out()
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let message = format!("Password for {} at {}", username, site);
	let password = evscode::InputBox::new()
		.prompt(&message)
		.password()
		.ignore_focus_out()
		.show()
		.await
		.ok_or_else(E::cancel)?;
	let kr = Keyring::new("credentials", site);
	if !kr
		.set(
			&serde_json::to_string(&Credentials {
				username: username.clone(),
				password: password.clone(),
			})
			.unwrap(),
		)
		.await
	{
		E::error("failed to save password to a secure keyring, so it will not be remembered")
			.warning()
			.action_if(is_installed("kwalletd5").await?, "How to fix (KWallet)", help_fix_kwallet())
			.emit();
	}
	Ok((username, password))
}

pub async fn get_cached_or_ask(site: &str) -> R<(String, String)> {
	let kr = Keyring::new("credentials", site);
	match kr.get().await {
		Some(encoded) => {
			let creds: Credentials = serde_json::from_str(&encoded).unwrap();
			Ok((creds.username, creds.password))
		},
		None => get_force_ask(site).await,
	}
}

pub async fn get_if_cached(site: &str) -> Option<String> {
	Keyring::new("session", site).get().await
}

pub async fn save_cache(site: &str, value: &str) {
	Keyring::new("session", site).set(value).await; // ignore save fail
}

pub async fn has_any_saved(site: &str) -> bool {
	Keyring::new("session", site).get().await.is_some()
		|| Keyring::new("credentials", site).get().await.is_some()
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
	Keyring::new("credentials", &site).delete().await;
	Keyring::new("session", &site).delete().await;
	Ok(())
}

async fn help_fix_kwallet() -> R<()> {
	evscode::open_external("https://github.com/pustaczek/icie/issues/14#issuecomment-516982482")
		.await
}

struct Keyring {
	kind: &'static str,
	site: String,
}
impl Keyring {
	fn new(kind: &'static str, site: &str) -> Keyring {
		Keyring { kind, site: site.to_owned() }
	}

	async fn get(&self) -> Option<String> {
		let entry = format!("@{} {}", self.kind, self.site);
		match JsFuture::from(keytar_sys::get_password("ICIE", &entry)).await {
			Ok(val) => val.as_string(),
			Err(e) => {
				log::error!("keyring errored, details: {:#?}", e);
				None
			},
		}
	}

	async fn set(&self, value: &str) -> bool {
		let entry = format!("@{} {}", self.kind, self.site);
		match JsFuture::from(keytar_sys::set_password("ICIE", &entry, value)).await {
			Ok(_) => true,
			Err(e) => {
				log::error!("keyring errored, details: {:#?}", e);
				false
			},
		}
	}

	async fn delete(&self) {
		let entry = format!("@{} {}", self.kind, self.site);
		match JsFuture::from(keytar_sys::delete_password("ICIE", &entry)).await {
			Ok(_) => {},
			Err(e) => {
				log::error!("keyring errored, details: {:#?}", e);
			},
		}
	}
}
