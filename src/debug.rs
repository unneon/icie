use crate::{
	build, executable::{Environment, Executable}, telemetry::TELEMETRY, term, test::time_limit, util, util::fs
};
use evscode::{E, R};
use std::path::PathBuf;

pub async fn gdb(in_path: PathBuf, source: Option<PathBuf>) -> R<()> {
	TELEMETRY.debug_gdb.spark();
	if !util::is_installed("gdb").await? {
		return Err(E::error("GDB is not installed").action_if(util::is_installed("apt").await?, "🔐 Auto-install", install_gdb()));
	}
	term::debugger("GDB", &in_path, &[
		"gdb",
		"-q",
		build::exec_path(source)?.to_str().unwrap(),
		"-ex",
		&format!("set args < {}", util::bash_escape(in_path.to_str().unwrap())),
	])
}

pub async fn rr(in_path: PathBuf, source: Option<PathBuf>) -> R<()> {
	TELEMETRY.debug_rr.spark();
	if !util::is_installed("rr").await? {
		return Err(E::error("RR is not installed").action_if(util::is_installed("apt").await?, "🔐 Auto-install", install_rr()));
	}
	let input = fs::read_to_string(&in_path).await?;
	let exec_path = build::exec_path(source)?;
	let rr = Executable::new_name("rr".to_owned());
	let args = ["record", exec_path.to_str().unwrap()];
	let environment = Environment { time_limit: time_limit() };
	let record_out = rr.run(&input, &args, &environment).await?;
	if record_out.stderr.contains("/proc/sys/kernel/perf_event_paranoid") {
		return Err(E::error("RR is not configured properly (this is to be expected), kernel.perf_event_paranoid must be <= 1")
			.action("🔐 Auto-configure", configure_kernel_perf_event_paranoid()));
	}
	term::debugger("RR", &in_path, &["rr", "replay", "--", "-q"])
}

async fn install_gdb() -> R<()> {
	term::install("GDB", &["pkexec", "apt", "install", "-y", "gdb"])
}
async fn install_rr() -> R<()> {
	term::install("RR", &["pkexec", "apt", "install", "-y", "rr"])
}
async fn configure_kernel_perf_event_paranoid() -> R<()> {
	term::Internal::raw(
		"ICIE Auto-configure RR",
		"echo 'kernel.perf_event_paranoid=1' | pkexec tee -a /etc/sysctl.conf && echo 1 | pkexec tee -a /proc/sys/kernel/perf_event_paranoid",
	)
}
