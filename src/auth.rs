pub fn site_credentials(site: &str) -> evscode::R<(String, String)> {
	let entry_name = format!("@credentials {}", site);
	let kr = keyring::Keyring::new("icie", &entry_name);
	match kr.get_password() {
		Ok(creds) => {
			let creds = json::parse(&creds)?;
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
				.ok_or_else(|| evscode::E::cancel())?;
			let password = evscode::InputBox::new()
				.prompt(format!("Password for {} at {}", username, site))
				.password()
				.ignore_focus_out()
				.build()
				.wait()
				.ok_or_else(|| evscode::E::cancel())?;
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
