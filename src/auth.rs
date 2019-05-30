use evscode::{E, R};

pub fn site_credentials(site: &str) -> R<(String, String)> {
	let entry = entry_credentials(site);
	let kr = keyring_credentials(&entry);
	match kr.get_password() {
		Ok(creds) => {
			let creds = json::parse(&creds).map_err(|e| E::from_std(e).context("credentials were saved in an invalid format"))?;
			Ok((creds["username"].as_str().unwrap().to_owned(), creds["password"].as_str().unwrap().to_owned()))
		},
		Err(e) => {
			let has_errored = match &e {
				keyring::KeyringError::NoPasswordFound => false,
				_ => true,
			};
			if has_errored {
				evscode::Message::new("failed to use a secure keyring, password will not be remembered")
					.warning()
					.build()
					.spawn();
				log::warn!("keyring error details: {:#?}", e);
			}
			let username = evscode::InputBox::new()
				.prompt(format!("Username at {}", site))
				.ignore_focus_out()
				.build()
				.wait()
				.ok_or_else(E::cancel)?;
			let password = evscode::InputBox::new()
				.prompt(format!("Password for {} at {}", username, site))
				.password()
				.ignore_focus_out()
				.build()
				.wait()
				.ok_or_else(E::cancel)?;
			if !has_errored
				&& kr
					.set_password(
						&json::object! {
							"username" => username.as_str(),
							"password" => password.as_str(),
						}
						.dump(),
					)
					.is_err()
			{
				evscode::Message::new("failed to use a secure keyring, password will not be remembered")
					.warning()
					.build()
					.spawn();
				log::warn!("keyring error details: {:#?}", e);
			}
			Ok((username, password))
		},
	}
}

#[evscode::command(title = "ICIE Password reset")]
fn reset() -> R<()> {
	let url = evscode::InputBox::new()
		.prompt("Enter any task URL from the site for which you want to reset the password")
		.placeholder("https://codeforces.com/contest/.../problem/...")
		.ignore_focus_out()
		.build()
		.wait()
		.ok_or_else(E::cancel)?;
	let url = unijudge::TaskUrl::deconstruct(&url).map_err(E::from_failure)?;
	let entry = entry_credentials(&url.site);
	let kr = keyring_credentials(&entry);
	match kr.delete_password() {
		Ok(()) => Ok(()),
		Err(keyring::KeyringError::NoPasswordFound) => Ok(()),
		Err(e) => Err(E::from_std(e).context("failed to use the keyring")),
	}
}

fn entry_credentials(site: &str) -> String {
	format!("@credentials {}", site)
}

fn keyring_credentials(entry: &str) -> keyring::Keyring {
	keyring::Keyring::new("icie", entry)
}
