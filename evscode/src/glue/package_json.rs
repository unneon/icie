use crate::meta::{Activation, GalleryTheme, Package};
use serde::{Serialize, Serializer};
use std::{
	collections::{BTreeMap, HashMap}, fmt
};

pub fn construct_package_json(pkg: &Package) -> PackageJson {
	PackageJson {
		name: pkg.identifier.to_owned(),
		version: pkg.version.to_owned(),
		publisher: pkg.publisher.to_owned(),
		engines: Engines { vscode: pkg.vscode_version.to_owned() },
		display_name: pkg.name.to_owned(),
		description: pkg.description.to_owned(),
		categories: pkg.categories.iter().map(|s| (*s).to_owned()).collect(),
		keywords: pkg.keywords.iter().map(|s| (*s).to_owned()).collect(),
		gallery_banner: GalleryBanner {
			color: pkg.gallery.color,
			theme: match pkg.gallery.theme {
				GalleryTheme::Dark => "dark",
				GalleryTheme::Light => "light",
			},
		},
		license: pkg.license.to_owned(),
		repository: pkg.repository.to_owned(),
		main: "icie.js".to_owned(),
		contributes: Contributes {
			commands: SortedVec::new(
				pkg.commands.iter().map(|command| ContributesCommands {
					command: command.id.to_string(),
					title: command.title.to_owned(),
				}),
				|cmd| cmd.command.clone(),
			),
			keybindings: SortedVec::new(
				pkg.commands.iter().filter_map(|command| {
					command.key.clone().map(|key| ContributesKeybindings {
						command: command.id.to_string(),
						key: key.to_owned(),
					})
				}),
				|cmd| cmd.command.clone(),
			),
			configuration: ContributesConfiguration {
				r#type: "object".to_owned(),
				title: pkg.name.to_owned(),
				properties: pkg
					.configuration
					.iter()
					.map(|ce| {
						let mut entry = (ce.schema)();
						entry["description"] = ce.description.into();
						(ce.id.to_string(), entry)
					})
					.collect(),
			},
		},
		activation_events: collect_activation_events(pkg)
			.into_iter()
			.map(|ev| ev.package_json_format())
			.collect(),
		markdown: "github",
		qna: "marketplace",
		dependencies: pkg
			.node_dependencies
			.iter()
			.map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
			.collect(),
		icon: "icon.png",
	}
}

#[derive(Debug, Serialize)]
pub struct PackageJson {
	name: String,
	version: String,
	publisher: String,
	engines: Engines,
	#[serde(rename = "displayName")]
	display_name: String,
	description: String,
	categories: Vec<String>,
	keywords: Vec<String>,
	#[serde(rename = "galleryBanner")]
	gallery_banner: GalleryBanner,
	license: String,
	repository: String,
	main: String,
	contributes: Contributes,
	#[serde(rename = "activationEvents")]
	activation_events: Vec<String>,
	markdown: &'static str,
	qna: &'static str,
	dependencies: HashMap<String, String>,
	icon: &'static str,
}

#[derive(Debug, Serialize)]
struct Engines {
	vscode: String,
}

#[derive(Debug, Serialize)]
struct GalleryBanner {
	color: &'static str,
	theme: &'static str,
}

#[derive(Debug, Serialize)]
struct Contributes {
	commands: SortedVec<ContributesCommands>,
	keybindings: SortedVec<ContributesKeybindings>,
	configuration: ContributesConfiguration,
}

#[derive(Debug, Serialize)]
struct ContributesCommands {
	command: String,
	title: String,
}

#[derive(Debug, Serialize)]
struct ContributesKeybindings {
	command: String,
	key: String,
}

#[derive(Debug, Serialize)]
struct ContributesConfiguration {
	r#type: String,
	title: String,
	properties: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ContributesConfigurationProperties {}

struct SortedVec<T> {
	inner: Vec<T>,
}

impl<T> SortedVec<T> {
	fn new<K: Ord>(i: impl Iterator<Item=T>, key: impl FnMut(&T) -> K) -> SortedVec<T> {
		let mut inner: Vec<T> = i.collect();
		inner.sort_by_key(key);
		SortedVec { inner }
	}
}

impl<T: Serialize> Serialize for SortedVec<T> {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.inner.serialize(serializer)
	}
}

impl<T: fmt::Debug> fmt::Debug for SortedVec<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<Vec<T> as fmt::Debug>::fmt(&self.inner, f)
	}
}

fn collect_activation_events(pkg: &Package) -> Vec<Activation<String>> {
	let mut events = Vec::new();
	for command in &pkg.commands {
		events.push(Activation::OnCommand { command: command.id });
	}
	events.extend(pkg.extra_activations.iter().map(|ev| ev.own()));
	events
}
