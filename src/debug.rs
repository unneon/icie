use crate::{build, term, util};
use std::{
	fs::File, path::PathBuf, process::{Command, Stdio}
};

pub fn gdb(in_path: PathBuf, source: Option<PathBuf>) -> evscode::R<()> {
	if cfg!(not(unix)) {
		return Err(evscode::E::error("GDB debugging is only supported on Linux"));
	}
	if !util::is_installed("gdb")? {
		return Err(evscode::E::error("GDB is not installed").action("🔐 Install", install_gdb));
	}
	term::debugger(
		"gdb",
		&[
			"-q",
			build::exec_path(source).to_str().unwrap(),
			"-ex",
			&format!("set args < {}", util::bash_escape(in_path.to_str().unwrap())),
		],
	)
}

pub fn rr(in_path: PathBuf, source: Option<PathBuf>) -> evscode::R<()> {
	if cfg!(not(unix)) {
		return Err(evscode::E::error("RR debugging is only supported on Linux"));
	}
	if !util::is_installed("rr")? {
		return Err(evscode::E::error("RR is not installed").action("🔐 Install", install_rr));
	}
	let record_out = Command::new("rr")
		.arg("record")
		.arg(build::exec_path(source))
		.stdin(File::open(in_path)?)
		.stdout(Stdio::null())
		.stderr(Stdio::piped())
		.output()?;
	if std::str::from_utf8(&record_out.stderr)?.contains("/proc/sys/kernel/perf_event_paranoid") {
		return Err(
			evscode::E::error("RR is not configured properly (this is to be expected), kernel.perf_event_paranoid must be <= 1")
				.action("🔐 Auto-configure", configure_kernel_perf_event_paranoid),
		);
	}
	term::debugger("rr", &["replay", "--", "-q"])
}

fn install_gdb() -> evscode::R<()> {
	term::install("GDB", "pkexec", &["apt", "install", "-y", "gdb"])
}
fn install_rr() -> evscode::R<()> {
	term::install("RR", "pkexec", &["apt", "install", "-y", "rr"])
}
fn configure_kernel_perf_event_paranoid() -> evscode::R<()> {
	Ok(term::internal(
		"ICIE Auto-configure RR",
		"echo 'kernel.perf_event_paranoid=1' | pkexec tee -a /etc/sysctl.conf && echo 1 | pkexec tee -a /proc/sys/kernel/perf_event_paranoid",
	))
}
