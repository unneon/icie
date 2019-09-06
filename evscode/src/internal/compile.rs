use crate::meta::{Activation, Package};
use json::{object, JsonValue};
use std::{
	fs, io::{self, Read}, path::{Path, PathBuf}, process::{Command, Stdio}, time::Duration
};

pub struct Toolchain<'a> {
	meta: &'a Package,
	manifest: &'a Path,
	package: PathBuf,
	exec_linux: &'a Path,
}

impl<'a> Toolchain<'a> {
	pub fn new(meta: &'a Package, manifest: &'a Path, exec_linux: &'a Path) -> Toolchain<'a> {
		let target_linux = exec_linux.parent().expect("evscode::Toolchain::new exec_linux has no parent");
		let package = target_linux.join("evscode");
		Toolchain { meta, manifest, package, exec_linux }
	}

	pub fn compile(&self) -> io::Result<()> {
		let ctx = BuildContext::new(&self.package);
		ctx.string("Generating", "metadata", "data/meta.json", render_meta(self.meta))?;
		ctx.json("Generating", "manifest", "package.json", construct_package_json(self.meta))?;
		ctx.file("Copying", "readme", "README.md", self.manifest.join("README.md"))?;
		ctx.file("Copying", "changelog", "CHANGELOG.md", self.manifest.join("CHANGELOG.md"))?;
		ctx.string("Copying", "glue", "out/extension.js", include_str!("../glue.js"))?;
		ctx.file("Copying", "executable", "data/bin/linux", self.exec_linux)?;
		ctx.rsync("Copying", "assets", "data/assets/", self.manifest.join("assets/"))?;
		ctx.task("Running", "`npm install`", None, Command::new("npm").arg("install").current_dir(&self.package), "npm install", &["package.json"])?;
		Ok(())
	}

	pub fn launch(&self) -> io::Result<()> {
		let ctx = BuildContext::new(&self.package);
		ctx.task(
			"Launching",
			&format!("`code --extensionDevelopmentPath {}`", self.package.display()),
			None,
			Command::new("code").arg("--extensionDevelopmentPath").arg(&self.package),
			"code --extensionDevelopmentPath",
			&[],
		)?;
		Ok(())
	}

	pub fn package(&self) -> io::Result<()> {
		let ctx = BuildContext::new(&self.package);
		ctx.task(
			"Packaging",
			"`vsce package`",
			Some(&format!("{}-{}.vsix", self.meta.identifier, self.meta.version)),
			Command::new("vsce").arg("package").current_dir(&self.package),
			"vsce package",
			&[],
		)?;
		Ok(())
	}

	pub fn publish(&self) -> io::Result<()> {
		let ctx = BuildContext::new(&self.package);
		ctx.task("Publishing", "`vsce publish`", None, Command::new("vsce").arg("publish").current_dir(&self.package), "vsce publish", &[])?;
		Ok(())
	}
}

enum BuildResult {
	Built,
	Ignored,
	Error(std::io::Error),
}
impl std::ops::Try for BuildResult {
	type Error = BuildResult;
	type Ok = ();

	fn into_result(self) -> Result<(), BuildResult> {
		match self {
			BuildResult::Built => Ok(()),
			BuildResult::Ignored => Err(self),
			BuildResult::Error(_) => Err(self),
		}
	}

	fn from_error(v: BuildResult) -> Self {
		v
	}

	fn from_ok(_: ()) -> Self {
		BuildResult::Built
	}
}
impl From<io::Error> for BuildResult {
	fn from(e: io::Error) -> Self {
		BuildResult::Error(e)
	}
}

const FUSER_EXEC_DELAY: Duration = Duration::from_millis(500);

pub struct BuildContext<'a> {
	base: &'a Path,
}
impl<'a> BuildContext<'a> {
	pub fn new(base: &'a Path) -> BuildContext<'a> {
		BuildContext { base }
	}

	pub fn string(&self, header: &str, description: &str, dest: impl AsRef<str>, contents: impl AsRef<[u8]>) -> io::Result<()> {
		self.run(header, description, Some(dest.as_ref()), || {
			let path = self.base.join(dest.as_ref());
			self.prepare(&path, contents.as_ref())?;
			fs::write(path, contents.as_ref())?;
			BuildResult::Built
		})
	}

	pub fn json(&self, header: &str, description: &str, dest: impl AsRef<str>, contents: json::JsonValue) -> io::Result<()> {
		self.run(header, description, Some(dest.as_ref()), || {
			let dest = self.base.join(dest.as_ref());
			if dest.exists() {
				let old_raw = std::fs::read_to_string(&dest)?;
				match json::parse(&old_raw) {
					Ok(ref old) if *old == contents => return BuildResult::Ignored,
					_ => (),
				}
			}
			assure_dir(dest.parent().unwrap())?;
			fs::write(&dest, json::stringify_pretty(contents, 4))?;
			BuildResult::Built
		})
	}

	pub fn file(&self, header: &str, description: &str, dest: impl AsRef<str>, source: impl AsRef<Path>) -> io::Result<()> {
		self.run(header, description, Some(dest.as_ref()), || {
			let path = self.base.join(dest.as_ref());
			self.prepare(&path, fs::read(source.as_ref())?)?;
			match fs::copy(source.as_ref(), &path) {
				Ok(_) => BuildResult::Built,
				Err(ref e) if e.kind() == io::ErrorKind::Other => {
					std::process::Command::new("fuser").arg("-ks").arg(&path).status()?;
					std::thread::sleep(FUSER_EXEC_DELAY);
					fs::copy(source.as_ref(), &path)?;
					BuildResult::Built
				},
				Err(e) => BuildResult::Error(e),
			}
		})
	}

	pub fn maybe_file(&self, header: &str, description: &str, dest: impl AsRef<str>, should: bool, source: impl AsRef<Path>) -> io::Result<()> {
		self.run(header, description, Some(dest.as_ref()), || {
			let path = self.base.join(dest.as_ref());
			if should {
				self.prepare(&path, fs::read(source.as_ref())?)?;
				match fs::copy(source.as_ref(), &path) {
					Ok(_) => BuildResult::Built,
					Err(ref e) if e.kind() == io::ErrorKind::Other => {
						std::process::Command::new("fuser").arg("-ks").arg(&path).status()?;
						fs::copy(source.as_ref(), &path)?;
						BuildResult::Built
					},
					Err(e) => BuildResult::Error(e),
				}
			} else if path.exists() {
				fs::remove_file(path)?;
				BuildResult::Built
			} else {
				BuildResult::Ignored
			}
		})
	}

	pub fn task(&self, header: &str, description: &str, dest: Option<&str>, cmd: &mut Command, desc: &str, dependencies: &[&str]) -> io::Result<()> {
		let updates = dependencies.iter().map(|dep| Ok(self.base.join(dep).metadata()?.modified()?)).collect::<io::Result<Vec<_>>>()?;
		let last_update = updates.into_iter().max();
		let trimmed = self.make_identifier(desc);
		let marker = self.base.join(format!(".{}.buildmark", trimmed));
		self.run(header, description, dest, || {
			if let Some(last_update) = last_update {
				if marker.exists() && marker.metadata()?.modified()? >= last_update {
					return BuildResult::Ignored;
				}
			}
			let stat = cmd.status()?;
			if stat.success() {
				fs::write(&marker, [])?;
				BuildResult::Built
			} else {
				BuildResult::Error(io::Error::from(io::ErrorKind::Other))
			}
		})
	}

	pub fn rsync(&self, header: &str, description: &str, dest: impl AsRef<str>, source: impl AsRef<Path>) -> io::Result<()> {
		let source = source.as_ref();
		self.run(header, description, Some(dest.as_ref()), || {
			let dest = self.base.join(dest.as_ref());
			if !source.exists() {
				if dest.exists() {
					fs::remove_dir_all(dest)?;
					BuildResult::Built
				} else {
					BuildResult::Ignored
				}
			} else {
				let kid = Command::new("rsync").arg("-ari").arg("--delete").arg(&source).arg(&dest).stdout(Stdio::piped()).spawn()?;
				let mut out_buf = Vec::new();
				kid.stdout.expect("evscode::BuildContext::rsync stdout absent").read_to_end(&mut out_buf)?;
				if out_buf.is_empty() { BuildResult::Ignored } else { BuildResult::Built }
			}
		})
	}

	fn prepare(&self, dest: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> BuildResult {
		if dest.as_ref().exists() {
			let current = fs::read(dest.as_ref())?;
			if current == contents.as_ref() {
				return BuildResult::Ignored;
			}
		}
		if let Some(p) = dest.as_ref().parent() {
			assure_dir(p)?;
		}
		BuildResult::Built
	}

	fn make_identifier(&self, s: &str) -> String {
		let mut buf = String::new();
		for c in s.chars() {
			if c.is_alphanumeric() {
				buf.push(c);
			}
		}
		assert!(!buf.is_empty());
		buf
	}

	fn run(&self, header: &str, description: &str, dest: Option<&str>, f: impl FnOnce() -> BuildResult) -> io::Result<()> {
		assert!(header.len() <= 12);
		eprintln!("\x1B[1;32m{: >12}\x1B[0m evscode {}{}", header, description, match dest {
			Some(dest) => format!(" ({})", self.base.join(dest).display()),
			None => String::new(),
		});
		match f() {
			BuildResult::Built => Ok(()),
			BuildResult::Ignored => Ok(()),
			BuildResult::Error(e) => Err(e),
		}
	}
}

fn assure_dir(path: &Path) -> io::Result<()> {
	if !path.exists() {
		if let Some(p) = path.parent() {
			assure_dir(p)?;
		}
		fs::create_dir(path)?;
	}
	Ok(())
}

fn construct_package_json(pkg: &Package) -> json::JsonValue {
	object! {
		"name" => pkg.identifier,
		"version" => pkg.version,
		"publisher" => pkg.publisher,
		"engines" => object! {
			"vscode" => "^1.33.0",
		},
		"displayName" => pkg.name,
		"description" => pkg.description,
		"categories" => json_slice_strs(pkg.categories),
		"keywords" => json_slice_strs(pkg.keywords),
		"license" => pkg.license,
		"repository" => pkg.repository,
		"main" => "./out/extension",
		"contributes" => object! {
			"commands" => sorted_svk(&pkg.commands, |cmd| cmd.id).map(|command| {
				object! {
					"command" => command.id.to_string(),
					"title" => command.title,
				}
			}).collect::<Vec<_>>(),
			"keybindings" => sorted_svk(&pkg.commands, |cmd| cmd.id).filter_map(|command| {
				command.key.clone().map(|key| {
					object! {
						"command" => command.id.to_string(),
						"key" => key,
					}
				})
			}).collect::<Vec<_>>(),
			"configuration" => object! {
				"type" => "object",
				"title" => pkg.name,
				"properties" => collect_json_obj(sorted_svk(&pkg.configuration, |ce| ce.id).map(|ce| {
					let mut entry = (ce.schema)();
					entry["description"] = ce.description.into();
					(ce.id.to_string(), entry)
				})),
			}
		},
		"activationEvents" => collect_activation_events(pkg).into_iter().map(|ev| ev.package_json_format()).collect::<Vec<_>>(),
		"badges" => json::array! [],
		"markdown" => "github",
		"qna" => "marketplace",
		"devDependencies" => object! {
			"vscode" => "^1.1.33",
		},
		"scripts" => object! {
			"postinstall" => "node ./node_modules/vscode/bin/install",
		},
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

fn render_meta(pkg: &Package) -> String {
	let obj = object! {
		"id" => pkg.identifier,
		"name" => pkg.name,
		"repository" => pkg.repository,
		"commands" => pkg.commands.iter().map(|cmd| cmd.id.to_string()).collect::<Vec<_>>(),
	};
	json::stringify_pretty(obj, 4)
}

fn json_slice_strs(ss: &[&str]) -> json::JsonValue {
	json::JsonValue::Array(ss.iter().map(|ss| json::JsonValue::String(ss.to_string())).collect())
}

fn sorted_svk<T, K: Ord>(slice: &[T], mut key: impl FnMut(&T) -> K) -> impl Iterator<Item=&T> {
	let mut vec = slice.iter().collect::<Vec<_>>();
	vec.sort_by_key(|x| key(*x));
	vec.into_iter()
}

fn collect_json_obj(i: impl Iterator<Item=(String, JsonValue)>) -> json::object::Object {
	let mut obj = json::object::Object::new();
	for (key, value) in i {
		obj.insert(&key, value);
	}
	obj
}
