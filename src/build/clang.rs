use crate::{
	build::{Codegen, Location, Message, Standard, Status, WINDOWS_MINGW_PATH}, executable::{Environment, Executable}, service::Service, util, util::{fs, OS}
};
use evscode::R;
use lazy_static::lazy_static;
use regex::Regex;
use util::path::Path;

const CLANG: Service = Service {
	human_name: "Clang",
	exec_linuxmac: Some("clang++"),
	exec_windows: None,
	package_apt: Some("clang"),
	// On macOS, Clang is supposed to be installed along with some part of Xcode.
	// Also trying to run the command will display a dialog asking the user to install it.
	// In this very specific situation, macOS seems pretty nice.
	package_brew: None,
	package_pacman: Some("clang"),
	tutorial_url_windows: None,
};

// Searching for MinGW is more complex than searching for Linux/macOS executables, so this is just
// to display a nice error message with a tutorial link.
const MINGW: Service = Service {
	human_name: "MinGW",
	exec_linuxmac: None,
	exec_windows: None,
	package_apt: None,
	package_brew: None,
	package_pacman: None,
	tutorial_url_windows: Some("https://github.com/pustaczek/icie/blob/master/docs/WINDOWS.md"),
};

pub async fn compile(
	sources: &[&Path],
	out: &Path,
	standard: Standard,
	codegen: Codegen,
	custom_flags: &[&str],
) -> R<Status>
{
	let compiler = find_compiler().await?;
	let executable = Executable::new(out.to_owned());
	let mut args = Vec::new();
	args.push(flag_standard(standard));
	args.extend(&["-Wall", "-Wextra", "-Wconversion", "-Wshadow", "-Wno-sign-conversion"]);
	args.extend(flags_codegen(codegen));
	args.extend(os_flags());
	args.extend(custom_flags);
	args.extend(sources.iter().map(|p| p.to_str().unwrap()));
	args.push("-o");
	args.push(&executable.command);
	let run = compiler
		.executable
		.run("", &args, &Environment {
			time_limit: None,
			// Windows g++ relies on some DLLs that are not in PATH. Since adding stuff to path
			// would have to be done by the user, it's better to just jest CWD to MinGW binaries
			// directory. This does not have to be done for compiled executables, because we add the
			// -static flag when compiling on Windows.
			cwd: compiler.mingw_path.map(|mingw| mingw.join("bin")),
		})
		.await?;
	let success = run.success();
	let mut errors = Vec::new();
	let mut warnings = Vec::new();
	for cap in (&ERROR_RE as &Regex).captures_iter(&run.stderr) {
		let line = cap[2].parse().unwrap();
		let column = cap[3].parse().unwrap();
		let severity = &cap[4];
		let message = cap[5].to_owned();
		let path = Path::from_native(cap[1].to_owned());
		(if severity == "error" || severity == "fatal error" {
			&mut errors
		} else {
			&mut warnings
		})
		.push(Message { message, location: Some(Location { path, line, column }) });
	}
	for cap in (&LINK_RE as &Regex).captures_iter(&run.stderr) {
		let message = cap[1].to_owned();
		errors.push(Message { message, location: None });
	}
	let stderr = run.stderr;
	if stderr.starts_with("xcode-select: note: no developer tools were found") {
		return Err(CLANG.not_installed().await?);
	}
	Ok(Status { success, executable, errors, warnings, stderr })
}

struct Compiler {
	executable: Executable,
	mingw_path: Option<Path>,
}

async fn find_compiler() -> R<Compiler> {
	match OS::query()? {
		OS::Linux | OS::MacOS => {
			Ok(Compiler { executable: CLANG.find_executable().await?, mingw_path: None })
		},
		OS::Windows => {
			let mingw_custom_path = WINDOWS_MINGW_PATH.get();
			// Various MinGW installers install this in various paths. CodeBlocks installs it in
			// "C:\Program Files (x64)\CodeBlocks\MinGW", but it's version does not work anyway so
			// there is no point in supporting it. mingw-builds try to install it in user directory
			// be default, but that's irritating to find so the tutorial asks them to install it in
			// "C:\MinGW" (which mingw-builds changes to "C:\MinGW\mingw32").
			let mingw_locations = if mingw_custom_path.is_empty() {
				vec!["C:\\MinGW\\mingw32", "C:\\MinGW"]
			} else {
				vec![mingw_custom_path.as_str()]
			};
			for mingw in mingw_locations {
				let mingw = Path::from_native(mingw.to_owned());
				let location = mingw.join("bin").join("g++.exe");
				if fs::exists(&location).await? {
					return Ok(Compiler {
						executable: Executable::new(location),
						mingw_path: Some(mingw),
					});
				}
			}
			Err(MINGW.not_installed().await?)
		},
	}
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
		Codegen::Debug => {
			&["-g", "-D_GLIBCXX_DEBUG", "-fno-sanitize-recover=undefined", "-fsanitize=undefined"]
		},
		Codegen::Release => &["-Ofast"],
		Codegen::Profile => &["-g", "-O2", "-fno-inline-functions"],
	}
}

fn os_flags() -> &'static [&'static str] {
	match OS::query() {
		// Sanitizers don't work because -lubsan is not found. There does not seem to be a fix.
		// Static linking makes it possible to avoid adding MinGW DLLs to PATH.
		Ok(OS::Windows) => &["-fno-sanitize=all", "-static"],
		_ => &[],
	}
}

lazy_static! {
	static ref ERROR_RE: Regex =
		Regex::new("(.*):(\\d+):(\\d+): (error|warning|fatal error): (.*)\\n").unwrap();
	static ref LINK_RE: Regex = Regex::new(".*(undefined reference to .*)").unwrap();
}
