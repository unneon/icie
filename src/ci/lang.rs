use crate::ci::{exec::Executable, util::R};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
	path::{Path, PathBuf}, process::{Command, Stdio}
};

#[derive(Debug)]
pub enum Codegen {
	Debug,
	Release,
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
	fn name(&self) -> &'static str;
}

pub trait Language {
	type Standard: Standard;
	fn source_extensions(&self) -> &'static [&'static str];
	fn standards(&self) -> &'static [Self::Standard];
	fn compile(&self, sources: &[&Path], out: &Path, version: &Self::Standard, profile: &Codegen) -> R<Status>;
}

pub enum CppStandard {
	Std03,
	Std11,
	Std14,
	Std17,
	Std2a,
}
impl Standard for CppStandard {
	fn name(&self) -> &'static str {
		match self {
			CppStandard::Std03 => "03",
			CppStandard::Std11 => "11",
			CppStandard::Std14 => "14",
			CppStandard::Std17 => "17",
			CppStandard::Std2a => "20",
		}
	}
}
impl CppStandard {
	fn gcc_flag(&self) -> &'static str {
		match self {
			CppStandard::Std03 => "-std=c++03",
			CppStandard::Std11 => "-std=c++11",
			CppStandard::Std14 => "-std=c++14",
			CppStandard::Std17 => "-std=c++17",
			CppStandard::Std2a => "-std=c++2a",
		}
	}
}

pub struct CPP;
impl Language for CPP {
	type Standard = CppStandard;

	fn source_extensions(&self) -> &'static [&'static str] {
		&["cpp", "cxx", "cc"]
	}

	fn standards(&self) -> &'static [CppStandard] {
		&[CppStandard::Std2a, CppStandard::Std17, CppStandard::Std14, CppStandard::Std11, CppStandard::Std03]
	}

	fn compile(&self, sources: &[&Path], out: &Path, standard: &CppStandard, codegen: &Codegen) -> R<Status> {
		let executable = Executable::new(out.to_path_buf());
		let mut cmd = Command::new("clang++");
		cmd.arg(standard.gcc_flag());
		cmd.args(&["-Wall", "-Wextra", "-Wconversion", "-Wshadow", "-Wno-sign-conversion"]);
		cmd.args(match codegen {
			Codegen::Debug => &["-g", "-D_GLIBCXX_DEBUG", "-fno-sanitize-recover=undefined", "-fsanitize=undefined"] as &'static [&'static str],
			Codegen::Release => &["-Ofast"],
		});
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
		for cap in CPP_ERROR_RE.captures_iter(&stderr) {
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
}

lazy_static! {
	static ref CPP_ERROR_RE: Regex = Regex::new("(.*):(\\d+):(\\d+): (error|warning): (.*)\\n").unwrap();
}
