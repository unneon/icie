use crate::{ci::exec::Executable, term, util};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
	path::{Path, PathBuf}, process::{Command, Stdio}
};

#[derive(Debug)]
pub enum Codegen {
	Debug,
	Release,
	Profile,
}

pub static CODEGEN_LIST: &'static [Codegen] = &[Codegen::Debug, Codegen::Release, Codegen::Profile];

impl Codegen {
	pub fn flags(&self) -> &'static [&'static str] {
		match self {
			Codegen::Debug => &["-g", "-D_GLIBCXX_DEBUG", "-fno-sanitize-recover=undefined", "-fsanitize=undefined"] as &'static [&'static str],
			Codegen::Release => &["-Ofast"],
			Codegen::Profile => &["-g", "-O2", "-fno-inline-functions"],
		}
	}
}

#[derive(Debug)]
pub struct Message {
	pub line: usize,
	pub column: usize,
	pub message: String,
	pub path: PathBuf,
}

#[derive(Debug)]
pub struct Status {
	pub success: bool,
	pub executable: Executable,
	pub errors: Vec<Message>,
	pub warnings: Vec<Message>,
}

pub trait Standard {
	fn as_gcc_flag(&self) -> &'static str;
}

pub static ALLOWED_EXTENSIONS: &'static [&'static str] = &["cpp", "cxx", "cc"];

pub fn compile(sources: &[&Path], out: &Path, standard: &impl Standard, codegen: &Codegen, custom_flags: &[&str]) -> evscode::R<Status> {
	if !util::is_installed("clang++")? {
		return Err(evscode::E::error("Clang is not installed").action_if(util::is_installed("apt")?, "🔐 Auto-install", install_clang));
	}
	let executable = Executable::new(out.to_path_buf());
	let mut cmd = Command::new("clang++");
	cmd.arg(standard.as_gcc_flag());
	cmd.args(&["-Wall", "-Wextra", "-Wconversion", "-Wshadow", "-Wno-sign-conversion"]);
	cmd.args(codegen.flags());
	cmd.args(custom_flags);
	cmd.args(sources);
	cmd.arg("-o");
	cmd.arg(&executable.path);
	cmd.stdin(Stdio::null());
	cmd.stdout(Stdio::null());
	cmd.stderr(Stdio::piped());
	let kid = cmd.spawn()?;
	let output = kid.wait_with_output()?;
	let success = output.status.success();
	let stderr = String::from_utf8(output.stderr).unwrap();
	let mut errors = Vec::new();
	let mut warnings = Vec::new();
	for cap in ERROR_RE.captures_iter(&stderr) {
		let line = cap[2].parse().unwrap();
		let column = cap[3].parse().unwrap();
		let severity = &cap[4];
		let message = cap[5].to_owned();
		let path = PathBuf::from(&cap[1]);
		(if severity == "error" { &mut errors } else { &mut warnings }).push(Message { line, column, message, path });
	}
	Ok(Status {
		success,
		executable,
		errors,
		warnings,
	})
}

fn install_clang() -> evscode::R<()> {
	term::install("Clang", &["pkexec", "apt", "install", "-y", "clang"])
}

lazy_static! {
	static ref ERROR_RE: Regex = Regex::new("(.*):(\\d+):(\\d+): (error|warning): (.*)\\n").unwrap();
}
