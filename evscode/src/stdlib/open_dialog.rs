//! Dialog for selecting files or directories.

use crate::{future::Pong, internal::executor::send_object};
use json::{object, JsonValue};
use std::{
	collections::HashMap, marker::PhantomData, path::{Path, PathBuf}
};

/// Builder for configuring dialogs. Use [`OpenDialog::new`] to create.
#[must_use]
pub struct Builder<C: Count> {
	files: bool,
	folders: bool,
	default: Option<PathBuf>,
	filters: Option<HashMap<String, Vec<String>>>,
	action_label: Option<String>,
	_count: PhantomData<C>,
}

impl Builder<Single> {
	/// Allow selecting multiple entries.
	pub fn many(self) -> Builder<Many> {
		Builder {
			files: self.files,
			folders: self.folders,
			default: self.default,
			filters: self.filters,
			action_label: self.action_label,
			_count: PhantomData,
		}
	}
}
impl<C: Count> Builder<C> {
	/// Switch to selecting directories instead of files.
	pub fn directory(mut self) -> Self {
		self.files = false;
		self.folders = true;
		self
	}

	/// Set a value selected by default.
	pub fn default(mut self, p: impl AsRef<Path>) -> Self {
		self.default = Some(p.as_ref().to_owned());
		self
	}

	/// Add a filter that allows selecting files based on extension set.
	/// The name will be displayed in the UI and should indicate what kind of files has these extensions.
	pub fn filter(mut self, name: impl AsRef<str>, extensions: impl IntoIterator<Item=impl AsRef<str>>) -> Self {
		if self.filters.is_none() {
			self.filters = Some(HashMap::new());
		}
		self.filters.as_mut().unwrap().insert(name.as_ref().to_owned(), extensions.into_iter().map(|s| s.as_ref().to_owned()).collect());
		self
	}

	/// Change the default label on the "Open" button
	pub fn action_label(mut self, label: impl AsRef<str>) -> Self {
		self.action_label = Some(label.as_ref().to_owned());
		self
	}
}
impl Builder<Single> {
	/// Open the dialog.
	pub async fn show(self) -> Option<PathBuf> {
		let pong = Pong::new();
		send_request(self.files, self.folders, false, self.default, self.filters, self.action_label, pong.aid());
		parse_response(pong.await).and_then(|arr| arr.into_iter().next())
	}
}
impl Builder<Many> {
	/// Open the dialog.
	pub async fn show(self) -> Option<Vec<PathBuf>> {
		let pong = Pong::new();
		send_request(self.files, self.folders, true, self.default, self.filters, self.action_label, pong.aid());
		parse_response(pong.await)
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
	pub fn new() -> Builder<Single> {
		Builder { files: true, folders: false, default: None, filters: None, action_label: None, _count: PhantomData }
	}
}

#[doc(hidden)]
pub trait Count {}

#[doc(hidden)]
pub struct Single;

#[doc(hidden)]
pub struct Many;

impl Count for Single {
}
impl Count for Many {
}

fn send_request(
	files: bool,
	folders: bool,
	many: bool,
	default: Option<PathBuf>,
	filters: Option<HashMap<String, Vec<String>>>,
	label: Option<String>,
	aid: u64,
) {
	send_object(object! {
		"tag" => "open_dialog",
		"canSelectFiles" => files,
		"canSelectFolders" => folders,
		"canSelectMany" => many,
		"defaultFile" => default.map(|p| p.to_str().unwrap().to_owned()),
		"filters" => filters.map(|hm| hm.into_iter().map(|(k, v)| (k, json::from(v))).collect::<HashMap<String, JsonValue>>()),
		"openLabel" => label,
		"aid" => aid,
	})
}

fn parse_response(raw: JsonValue) -> Option<Vec<PathBuf>> {
	if raw.is_null() { None } else { Some(raw.members().map(|p| PathBuf::from(p.as_str().unwrap())).collect()) }
}
