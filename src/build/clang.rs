use crate::{
	executable::{Environment, Executable}, term, util
};
use evscode::{E, R};
use lazy_static::lazy_static;
use regex::Regex;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Codegen {
	Debug,
	Release,
	Profile,
}

pub static CODEGEN_LIST: &[Codegen] = &[Codegen::Debug, Codegen::Release, Codegen::Profile];

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
pub struct Location {
	pub path: PathBuf,
	pub line: usize,
	pub column: usize,
}

#[derive(Debug)]
pub struct Message {
	pub message: String,
	pub location: Option<Location>,
}

#[derive(Debug)]
pub struct Status {
	pub success: bool,
	pub executable: Executable,
	pub errors: Vec<Message>,
	pub warnings: Vec<Message>,
	pub stderr: String,
}

pub trait Standard {
	fn as_gcc_flag(&self) -> &'static str;
}

pub static ALLOWED_EXTENSIONS: &[&str] = &["cpp", "cxx", "cc"];

pub async fn compile(sources: &[&Path], out: &Path, standard: &impl Standard, codegen: &Codegen, custom_flags: &[&str]) -> R<Status> {
	if !util::is_installed("clang++").await? {
		return Err(E::error("Clang is not installed").action_if(util::is_installed("apt").await?, "🔐 Auto-install", install_clang()));
	}
	let executable = Executable::new(out.to_path_buf());
	let mut args = Vec::new();
	args.push(standard.as_gcc_flag());
	args.extend(&["-Wall", "-Wextra", "-Wconversion", "-Wshadow", "-Wno-sign-conversion"]);
	args.extend(codegen.flags());
	args.extend(custom_flags);
	args.extend(sources.iter().map(|p| p.to_str().unwrap()));
	args.push("-o");
	args.push(&executable.command);
	let clang = Executable::new_name("clang++".to_owned());
	let run = clang.run("", &args, &Environment { time_limit: None }).await?;
	let success = run.success();
	let mut errors = Vec::new();
	let mut warnings = Vec::new();
	for cap in (&ERROR_RE as &Regex).captures_iter(&run.stderr) {
		let line = cap[2].parse().unwrap();
		let column = cap[3].parse().unwrap();
		let severity = &cap[4];
		let message = cap[5].to_owned();
		let path = PathBuf::from(&cap[1]);
		(if severity == "error" || severity == "fatal error" { &mut errors } else { &mut warnings })
			.push(Message { message, location: Some(Location { path, line, column }) });
	}
	for cap in (&LINK_RE as &Regex).captures_iter(&run.stderr) {
		let message = cap[1].to_owned();
		errors.push(Message { message, location: None });
	}
	let stderr = run.stderr;
	Ok(Status { success, executable, errors, warnings, stderr })
}

async fn install_clang() -> R<()> {
	term::install("Clang", &["pkexec", "apt", "install", "-y", "clang"])
}

lazy_static! {
	static ref ERROR_RE: Regex = Regex::new("(.*):(\\d+):(\\d+): (error|warning|fatal error): (.*)\\n").unwrap();
	static ref LINK_RE: Regex = Regex::new(".*(undefined reference to .*)").unwrap();
}
