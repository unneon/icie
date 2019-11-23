use crate::{
	build::{Codegen, Location, Message, Standard, Status}, executable::{Environment, Executable}, term, util
};
use evscode::{E, R};
use lazy_static::lazy_static;
use regex::Regex;
use std::path::{Path, PathBuf};

pub async fn compile(sources: &[&Path], out: &Path, standard: Standard, codegen: Codegen, custom_flags: &[&str]) -> R<Status> {
	if !util::is_installed("clang++").await? {
		return Err(E::error("Clang is not installed").action_if(util::is_installed("apt").await?, "🔐 Auto-install", install_clang()));
	}
	let executable = Executable::new(out.to_path_buf());
	let mut args = Vec::new();
	args.push(flag_standard(standard));
	args.extend(&["-Wall", "-Wextra", "-Wconversion", "-Wshadow", "-Wno-sign-conversion"]);
	args.extend(flags_codegen(codegen));
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

fn flag_standard(standard: Standard) -> &'static str {
	match standard {
		Standard::Cpp03 => "-std=c++03",
		Standard::Cpp11 => "-std=c++11",
		Standard::Cpp14 => "-std=c++14",
		Standard::Cpp17 => "-std=c++17",
		Standard::FutureCpp20 => "-std=c++2a",
	}
}
pub fn flags_codegen(codegen: Codegen) -> &'static [&'static str] {
	match codegen {
		Codegen::Debug => &["-g", "-D_GLIBCXX_DEBUG", "-fno-sanitize-recover=undefined", "-fsanitize=undefined"] as &'static [&'static str],
		Codegen::Release => &["-Ofast"],
		Codegen::Profile => &["-g", "-O2", "-fno-inline-functions"],
	}
}

lazy_static! {
	static ref ERROR_RE: Regex = Regex::new("(.*):(\\d+):(\\d+): (error|warning|fatal error): (.*)\\n").unwrap();
	static ref LINK_RE: Regex = Regex::new(".*(undefined reference to .*)").unwrap();
}
