use crate::{
	compile::{Codegen, Location, Message, Standard, Status, WINDOWS_MINGW_PATH}, executable::{Environment, Executable}, service::Service, telemetry::TELEMETRY, util, util::{fs, OS}
};
use evscode::R;
use once_cell::sync::Lazy;
use regex::Regex;
use util::path::Path;

struct Compiler {
	executable: Executable,
	mingw_path: Option<Path>,
}

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
	telemetry_install: &TELEMETRY.clang_install,
	telemetry_not_installed: &TELEMETRY.clang_not_installed,
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
	telemetry_install: &TELEMETRY.mingw_install,
	telemetry_not_installed: &TELEMETRY.mingw_not_installed,
	tutorial_url_windows: Some("https://github.com/pustaczek/icie/blob/master/docs/WINDOWS.md"),
};

pub async fn compile(
	sources: &[&Path],
	output_path: &Path,
	standard: Standard,
	codegen: Codegen,
	custom_flags: &[String],
) -> R<Status>
{
	let compiler = find_compiler().await?;
	let executable = Executable::new(output_path.to_owned());
	let args = collect_compiler_flags(sources, output_path, standard, codegen, custom_flags);
	let environment = get_compiler_environment(&compiler);
	let run = compiler.executable.run("", &args, &environment).await?;
	let (errors, warnings) = parse_clang_output(&run.stderr);
	check_macos_not_installed(&run.stderr).await?;
	Ok(Status { run, executable, errors, warnings })
}

async fn find_compiler() -> R<Compiler> {
	match OS::query()? {
		OS::Linux | OS::MacOS => {
			let executable = CLANG.find_executable().await?;
			Ok(Compiler { executable, mingw_path: None })
		},
		OS::Windows => find_compiler_mingw().await,
	}
}

async fn find_compiler_mingw() -> R<Compiler> {
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
			return Ok(Compiler { executable: Executable::new(location), mingw_path: Some(mingw) });
		}
	}
	Err(MINGW.not_installed().await?)
}

fn collect_compiler_flags<'a>(
	sources: &[&'a Path],
	output_path: &'a Path,
	standard: Standard,
	codegen: Codegen,
	custom_flags: &'a [String],
) -> Vec<&'a str>
{
	let mut args = Vec::new();
	args.push(standard.flag_clang());
	// -Wconversion displays warnings on lossy implicit conversions between i32/i64, u32/u64 and
	// others. These are awful to debug because no small tests trigger them, and using exclusively
	// i64 can hurt performance too much. -Wno-sign-conversions disables warnings on i32 to u32
	// conversions, because that happens every time a vector is indexed with an int.
	args.extend(&["-Wall", "-Wextra", "-Wconversion", "-Wshadow", "-Wno-sign-conversion"]);
	args.extend(codegen.flags_clang());
	args.extend(flags_os_specific());
	args.extend(custom_flags.iter().map(String::as_str));
	args.extend(sources.iter().copied().map(Path::as_str));
	args.push("-o");
	args.push(output_path);
	args
}

fn flags_os_specific() -> &'static [&'static str] {
	match OS::query() {
		// Sanitizers don't work because -lubsan is not found. There does not seem to be a fix.
		// Static linking makes it possible to avoid adding MinGW DLLs to PATH.
		Ok(OS::Windows) => &["-fno-sanitize=all", "-static"],
		_ => &[],
	}
}

fn get_compiler_environment(compiler: &Compiler) -> Environment {
	Environment {
		time_limit: None,
		// Windows g++ relies on some DLLs that are not in PATH. Since adding stuff to path
		// would have to be done by the user, it's better to just jest CWD to MinGW binaries
		// directory. This does not have to be done for compiled executables, because we add the
		// -static flag when compiling on Windows.
		cwd: compiler.mingw_path.as_ref().map(|mingw| mingw.join("bin")),
	}
}

fn parse_clang_output(stderr: &str) -> (Vec<Message>, Vec<Message>) {
	static COMPILATION_ERROR: Lazy<Regex> =
		Lazy::new(|| Regex::new("(.*):([0-9]+):([0-9]+): (error|warning|fatal error): (.*)\\n").unwrap());
	static LINKING_ERROR: Lazy<Regex> = Lazy::new(|| Regex::new(".*(undefined reference to .*)").unwrap());

	let mut errors = Vec::new();
	let mut warnings = Vec::new();
	for cap in COMPILATION_ERROR.captures_iter(stderr) {
		let path = Path::from_native(cap[1].to_owned());
		let line = cap[2].parse().unwrap();
		let column = cap[3].parse().unwrap();
		let severity = &cap[4];
		let message = cap[5].to_owned();
		let location = Some(Location { path, line, column });
		let is_error = severity == "error" || severity == "fatal error";
		let severity_list = if is_error { &mut errors } else { &mut warnings };
		severity_list.push(Message { message, location });
	}
	for cap in LINKING_ERROR.captures_iter(stderr) {
		let message = cap[1].to_owned();
		errors.push(Message { message, location: None });
	}
	(errors, warnings)
}

async fn check_macos_not_installed(stderr: &str) -> R<()> {
	if stderr.starts_with("xcode-select: note: no developer tools were found") {
		Err(CLANG.not_installed().await?)
	} else {
		Ok(())
	}
}
