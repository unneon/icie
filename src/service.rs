use crate::{
	executable::Executable, term, util::{is_installed, OS}
};
use evscode::{error::ResultExt, BoxFuture, E, R};

pub struct Service {
	pub human_name: &'static str,
	pub exec_linux: Option<&'static str>,
	pub exec_windows: Option<&'static str>,
	pub package_apt: Option<&'static str>,
	pub package_pacman: Option<&'static str>,
}

impl Service {
	pub async fn find_executable(&self) -> R<Executable> {
		self.find_command().await.map(Executable::new_name)
	}

	pub async fn find_command(&self) -> R<String> {
		let command = self.get_exec().wrap(format!("{} is not supported on your platform", self.human_name))?;
		if !is_installed(command).await? {
			let mut e = E::error(format!("{} is not installed", self.human_name));
			let mut valid_actions: Vec<(_, BoxFuture<R<()>>)> = Vec::new();
			if let Some(package) = self.package_apt {
				if is_installed("apt").await? {
					valid_actions.push(("apt", Box::pin(apt_install(package))));
				}
			}
			if let Some(package) = self.package_pacman {
				if is_installed("pacman").await? {
					valid_actions.push(("pacman", Box::pin(pacman_s(package))));
				}
			}
			let action_count = valid_actions.len();
			for (manager, action) in valid_actions {
				let title = if action_count <= 1 { "🔐 Auto-install".to_owned() } else { format!("🔐 Auto-install ({})", manager) };
				e = e.action(title, action);
			}
			return Err(e);
		}
		Ok(command.to_owned())
	}

	fn get_exec(&self) -> Option<&'static str> {
		match OS::query() {
			Ok(OS::Linux) => self.exec_linux,
			Ok(OS::Windows) => self.exec_windows,
			Err(_) => None,
		}
	}
}

async fn apt_install(package: &'static str) -> R<()> {
	term::install(package, &["pkexec", "apt", "install", "-y", package])
}

async fn pacman_s(package: &'static str) -> R<()> {
	term::install(package, &["pkexec", "pacman", "-S", package])
}
