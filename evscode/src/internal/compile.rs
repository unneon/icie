use crate::meta::{Activation, Package};
use json::{object, JsonValue};
use std::{
	fs, io::{self, Read, Write}, path::{Path, PathBuf}, process::{Command, Stdio}, time::Duration
};

pub struct Toolchain<'a> {
	meta: &'a Package,
	manifest: &'a Path,
	package: PathBuf,
	exec_linux: &'a Path,
	_exec_windows: PathBuf,
	_is_example: bool,
}

impl<'a> Toolchain<'a> {
	pub fn new(pkg: &'a Package, manifest: &'a Path, exec_linux: &'a Path) -> Toolchain<'a> {
		let target_linux = exec_linux.parent().expect("evscode::Toolchain::new exec_linux has no parent");
		let package = target_linux.join(format!("{}-vscode", pkg.identifier));
		let _is_example = target_linux.file_name() == Some(std::ffi::OsStr::new("examples"));
		let mut target_root = target_linux.parent().expect("evscode::Toolchain::new target_linux has no parent");
		if _is_example {
			target_root = target_root.parent().expect("evscode::Toolchain::new target_linux w/ example has less than 2 parents")
		}
		let mut target_windows = target_root.join("x86_64-pc-windows-gnu/release");
		if _is_example {
			target_windows = target_windows.join("examples");
		}
		let _exec_windows = target_windows.join(format!("{}.exe", pkg.identifier));
		Toolchain { meta: pkg, manifest, package, exec_linux, _exec_windows, _is_example }
	}

	pub fn compile(&self, _cross_compile: bool) -> io::Result<()> {
		let ctx = BuildContext::new(&self.package);
		// ctx.run_multiline(
		// "[Windows]",
		// || {
		// if cross_compile {
		// BuildResult::Built
		// } else {
		// BuildResult::Skipped
		// }
		// },
		// || {
		// let example_arg = if self.is_example { vec!["--examples"] } else { vec![] };
		// let r1 = Command::new("cross")
		// .arg("build")
		// .arg("--release")
		// .arg("--target=x86_64-pc-windows-gnu")
		// .args(example_arg)
		// .current_dir(&self.manifest)
		// .status()?;
		// if r1.success() {
		// Ok(())
		// } else {
		// Err(io::Error::from(io::ErrorKind::Other))
		// }
		// },
		// )?;
		ctx.file("README.md", self.manifest.join("README.md"))?;
		ctx.file("CHANGELOG.md", self.manifest.join("CHANGELOG.md"))?;
		ctx.json("package.json", construct_package_json(self.meta))?;
		ctx.string("out/extension.js", include_str!("../glue.js"))?;
		ctx.string("data/meta.json", render_meta(self.meta))?;
		ctx.file("data/bin/linux", self.exec_linux)?;
		// ctx.maybe_file("data/bin/windows.exe", cross_compile, &self.exec_windows)?;
		ctx.rsync("data/assets/", self.manifest.join("assets/"))?;
		ctx.task(Command::new("npm").arg("install").current_dir(&self.package), "npm install", &["package.json"])?;
		Ok(())
	}

	pub fn launch(&self) -> io::Result<()> {
		let r1 = Command::new("code").arg("--extensionDevelopmentPath").arg(&self.package).status()?;
		if r1.success() { Ok(()) } else { Err(io::Error::from(io::ErrorKind::Other)) }
	}

	pub fn package(&self) -> io::Result<()> {
		let r1 = Command::new("vsce").arg("package").current_dir(&self.package).status().expect("evscode::package vsce spawn errored");
		if r1.success() { Ok(()) } else { Err(io::Error::from(io::ErrorKind::Other)) }
	}

	pub fn publish(&self) -> io::Result<()> {
		let r1 = Command::new("vsce").arg("publish").current_dir(&self.package).status()?;
		if r1.success() { Ok(()) } else { Err(io::Error::from(io::ErrorKind::Other)) }
	}
}

enum BuildResult {
	Built,
	Ignored,
	// Skipped,
	Error(std::io::Error),
}
impl std::ops::Try for BuildResult {
	type Error = BuildResult;
	type Ok = ();

	fn into_result(self) -> Result<(), BuildResult> {
		match self {
			BuildResult::Built => Ok(()),
			BuildResult::Ignored => Err(self),
			// BuildResult::Skipped => Err(self),
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

pub struct BuildContext<'a> {
	base: &'a Path,
}
impl<'a> BuildContext<'a> {
	pub fn new(base: &'a Path) -> BuildContext<'a> {
		eprintln!();
		BuildContext { base }
	}

	pub fn string(&self, dest: impl AsRef<str>, contents: impl AsRef<[u8]>) -> io::Result<()> {
		self.run(dest.as_ref(), || {
			let path = self.base.join(dest.as_ref());
			self.prepare(&path, contents.as_ref())?;
			fs::write(path, contents.as_ref())?;
			BuildResult::Built
		})
	}

	pub fn json(&self, dest: impl AsRef<str>, contents: json::JsonValue) -> io::Result<()> {
		self.run(dest.as_ref(), || {
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

	pub fn file(&self, dest: impl AsRef<str>, source: impl AsRef<Path>) -> io::Result<()> {
		self.run(dest.as_ref(), || {
			let path = self.base.join(dest.as_ref());
			self.prepare(&path, fs::read(source.as_ref())?)?;
			match fs::copy(source.as_ref(), &path) {
				Ok(_) => BuildResult::Built,
				Err(ref e) if e.kind() == io::ErrorKind::Other => {
					std::process::Command::new("fuser").arg("-ks").arg(&path).status()?;
					std::thread::sleep(Duration::from_millis(500));
					fs::copy(source.as_ref(), &path)?;
					BuildResult::Built
				},
				Err(e) => BuildResult::Error(e),
			}
		})
	}

	pub fn maybe_file(&self, dest: impl AsRef<str>, should: bool, source: impl AsRef<Path>) -> io::Result<()> {
		self.run(dest.as_ref(), || {
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

	pub fn task(&self, cmd: &mut Command, desc: &str, dependencies: &[&str]) -> io::Result<()> {
		let updates = dependencies.iter().map(|dep| Ok(self.base.join(dep).metadata()?.modified()?)).collect::<io::Result<Vec<_>>>()?;
		let last_update = updates.into_iter().max().expect("evscode::BuildContext::task no dependencies");
		let trimmed = self.make_identifier(desc);
		let marker = self.base.join(format!(".{}.buildmark", trimmed));
		self.run_multiline(
			"npm install",
			|| {
				if (!marker.exists()) || marker.metadata()?.modified()? < last_update { BuildResult::Built } else { BuildResult::Ignored }
			},
			|| {
				let stat = cmd.status()?;
				if stat.success() {
					fs::write(&marker, [])?;
					Ok(())
				} else {
					Err(io::Error::from(io::ErrorKind::Other))
				}
			},
		)
	}

	pub fn rsync(&self, dest: impl AsRef<str>, source: impl AsRef<Path>) -> io::Result<()> {
		let source = source.as_ref();
		self.run(dest.as_ref(), || {
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

	fn run(&self, desc: impl AsRef<str>, f: impl FnOnce() -> BuildResult) -> io::Result<()> {
		eprint!("{: >40} ...", desc.as_ref());
		io::stderr().flush()?;
		let r = f();
		let ending = match &r {
			BuildResult::Built => " \x1B[1;32mBuilt\x1B[0m",
			BuildResult::Ignored => " Ignored",
			// BuildResult::Skipped => " Skipped",
			BuildResult::Error(_) => " \x1B[1;31mError\x1B[0m",
		};
		eprintln!("{}", ending);
		match r {
			BuildResult::Built => Ok(()),
			BuildResult::Ignored => Ok(()),
			// BuildResult::Skipped => Ok(()),
			BuildResult::Error(e) => Err(e),
		}
	}

	fn run_multiline(&self, desc: impl AsRef<str>, f: impl FnOnce() -> BuildResult, g: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
		eprint!("{: >40} ...", desc.as_ref());
		io::stderr().flush()?;
		let r1 = f();
		match r1 {
			BuildResult::Built => {
				eprintln!("\n");
				let r2 = g();
				eprint!("\n\n{: >40} ...", desc.as_ref());
				match r2 {
					Ok(()) => {
						eprintln!(" \x1B[1;32mBuilt\x1B[0m");
						Ok(())
					},
					Err(e) => {
						eprintln!(" \x1B[1;31mError\x1B[0m");
						Err(e)
					},
				}
			},
			BuildResult::Ignored => {
				eprintln!(" Ignored");
				Ok(())
			},
			// BuildResult::Skipped => {
			// 	eprintln!(" Skipped");
			// 	Ok(())
			// },
			BuildResult::Error(e) => {
				eprintln!(" \x1B[1;31mError\x1B[0m");
				Err(e)
			},
		}
	}
}
impl<'a> Drop for BuildContext<'a> {
	fn drop(&mut self) {
		eprintln!();
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
			"commands" => sorted_svk(&pkg.commands, |cmd| cmd.inner_id).map(|command| {
				object! {
					"command" => format!("{}.{}", pkg.identifier, command.inner_id),
					"title" => command.title,
				}
			}).collect::<Vec<_>>(),
			"keybindings" => sorted_svk(&pkg.commands, |cmd| cmd.inner_id).filter_map(|command| {
				command.key.clone().map(|key| {
					object! {
						"command" => format!("{}.{}", pkg.identifier, command.inner_id),
						"key" => key,
					}
				})
			}).collect::<Vec<_>>(),
			"configuration" => object! {
				"type" => "object",
				"title" => pkg.name,
				"properties" => collect_json_obj(sorted_svk(&pkg.configuration, |ce| ce.id).map(|ce| {
					(format!("{}.{}", pkg.identifier, ce.id), (ce.schema)(ce.description))
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
		events.push(Activation::OnCommand { command: format!("{}.{}", pkg.identifier, command.inner_id) });
	}
	events.extend(pkg.extra_activations.iter().map(|ev| ev.own()));
	events
}

fn render_meta(pkg: &Package) -> String {
	let obj = object! {
		"id" => pkg.identifier,
		"name" => pkg.name,
		"repository" => pkg.repository,
		"commands" => pkg.commands.iter().map(|cmd| {
			format!("{}.{}", pkg.identifier, cmd.inner_id)
		}).collect::<Vec<_>>(),
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
