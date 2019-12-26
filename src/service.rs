use crate::{
	executable::Executable, term, util::{is_installed, OS}
};
use evscode::{error::ResultExt, E, R};

pub struct Service {
	pub human_name: &'static str,
	pub exec_linuxmac: Option<&'static str>,
	pub exec_windows: Option<&'static str>,
	pub package_apt: Option<&'static str>,
	pub package_brew: Option<&'static str>,
	pub package_pacman: Option<&'static str>,
	pub tutorial_url_windows: Option<&'static str>,
}

impl Service {
	pub async fn find_executable(&self) -> R<Executable> {
		self.find_command().await.map(Executable::new_name)
	}

	pub async fn find_command(&self) -> R<String> {
		let command = self
			.get_exec()
			.wrap(format!("{} is not supported on your platform", self.human_name))?;
		if !is_installed(command).await? {
			return Err(self.not_installed().await?);
		}
		Ok(command.to_owned())
	}

	pub async fn not_installed(&self) -> R<E> {
		let mut e = E::error(format!("{} is not installed", self.human_name));
		match OS::query()? {
			OS::Linux => {
				if let Some(package) = self.package_apt {
					if is_installed("apt").await? {
						e = e.action("🔐 Auto-install (apt)".to_owned(), apt_install(package));
					}
				}
				if let Some(package) = self.package_pacman {
					if is_installed("pacman").await? {
						e = e.action("🔐 Auto-install (pacman)".to_owned(), pacman_s(package));
					}
				}
			},
			OS::Windows => {
				if let Some(tutorial) = self.tutorial_url_windows {
					e = e.action("📄 How to install?".to_owned(), tutorial_show(tutorial));
				}
			},
			OS::MacOS => {
				if let Some(package) = self.package_brew {
					if is_installed("brew").await? {
						e = e.action("🔐 Auto install (brew)".to_owned(), brew_install(package));
					}
				}
			},
		}
		Ok(e)
	}

	fn get_exec(&self) -> Option<&'static str> {
		match OS::query() {
			Ok(OS::Linux) | Ok(OS::MacOS) => self.exec_linuxmac,
			Ok(OS::Windows) => self.exec_windows,
			Err(_) => None,
		}
	}
}

async fn apt_install(package: &'static str) -> R<()> {
	term::install(package, &["pkexec", "apt", "install", "-y", package])
}

async fn brew_install(package: &'static str) -> R<()> {
	term::install(package, &["brew", "install", package])
}

async fn pacman_s(package: &'static str) -> R<()> {
	term::install(package, &["pkexec", "pacman", "-S", package])
}

async fn tutorial_show(url: &'static str) -> R<()> {
	evscode::open_external(url).await
}
