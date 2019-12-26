//! Dialog for selecting files or directories.

use std::collections::HashMap;

/// Builder for configuring dialogs. Use [`OpenDialog::new`] to create.
#[must_use]
pub struct Builder {
	files: bool,
	folders: bool,
	default: Option<String>,
	filters: Option<HashMap<String, Vec<String>>>,
	action_label: Option<String>,
}

impl Builder {
	/// Switch to selecting directories instead of files.
	pub fn directory(mut self) -> Self {
		self.files = false;
		self.folders = true;
		self
	}

	/// Set a value selected by default.
	pub fn default(mut self, p: &str) -> Self {
		self.default = Some(p.to_owned());
		self
	}

	/// Add a filter that allows selecting files based on extension set.
	/// The name will be displayed in the UI and should indicate what kind of files has these
	/// extensions.
	pub fn filter(
		mut self,
		name: impl AsRef<str>,
		extensions: impl IntoIterator<Item=impl AsRef<str>>,
	) -> Self
	{
		if self.filters.is_none() {
			self.filters = Some(HashMap::new());
		}
		self.filters.as_mut().unwrap().insert(
			name.as_ref().to_owned(),
			extensions.into_iter().map(|s| s.as_ref().to_owned()).collect(),
		);
		self
	}

	/// Change the default label on the "Open" button
	pub fn action_label(mut self, label: impl AsRef<str>) -> Self {
		self.action_label = Some(label.as_ref().to_owned());
		self
	}

	/// Open the dialog.
	pub async fn show(self) -> Option<String> {
		vscode_sys::window::show_open_dialog(vscode_sys::window::OpenDialogOptions {
			can_select_files: self.files,
			can_select_folders: self.folders,
			can_select_many: false,
			filters: self.filters,
			open_label: self.action_label,
		})
		.await
		.map(|chosen| chosen.into_iter().next().unwrap())
	}
}

/// File open dialog provided by the VS Code API
///
/// See [module documentation](index.html) for details
pub struct OpenDialog {
	_a: (),
}

impl OpenDialog {
	/// Create a new builder to configure the dialog
	pub fn new() -> Builder {
		Builder { files: true, folders: false, default: None, filters: None, action_label: None }
	}
}
