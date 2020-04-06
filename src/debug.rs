use crate::{
	build, executable::{Environment, Executable}, service::Service, telemetry::TELEMETRY, term, test, util, util::{fs, path::Path, SourceTarget}
};
use evscode::{E, R};

pub const GDB: Service = Service {
	human_name: "GDB",
	exec_linuxmac: Some("gdb"),
	exec_windows: None,
	package_apt: Some("gdb"),
	package_brew: Some("gdb"),
	package_pacman: Some("gdb"),
	telemetry_install: &TELEMETRY.gdb_install,
	telemetry_not_installed: &TELEMETRY.gdb_not_installed,
	tutorial_url_windows: None,
};

pub const RR: Service = Service {
	human_name: "RR",
	exec_linuxmac: Some("rr"),
	exec_windows: None,
	package_apt: Some("rr"),
	package_brew: None,
	package_pacman: None,
	telemetry_install: &TELEMETRY.rr_install,
	telemetry_not_installed: &TELEMETRY.rr_not_installed,
	tutorial_url_windows: None,
};

pub async fn gdb(in_path: &Path, source: SourceTarget) -> R<()> {
	TELEMETRY.debug_gdb.spark();
	let gdb = GDB.find_command().await?;
	term::debugger("GDB", in_path, &[
		&gdb,
		"-q",
		build::executable_path(source)?.as_str(),
		"-ex",
		&format!("set args < {}", util::bash_escape(in_path.as_str())),
	])
}

pub async fn rr(in_path: &Path, source: SourceTarget) -> R<()> {
	TELEMETRY.debug_rr.spark();
	let rr = RR.find_command().await?;
	let rr_exec = Executable::new_name(rr.clone());
	let input = fs::read_to_string(in_path).await?;
	let exec_path = build::executable_path(source)?;
	let args = ["record", exec_path.as_str()];
	let environment = Environment { time_limit: test::time_limit(), cwd: None };
	let record_out = rr_exec.run(&input, &args, &environment).await?;
	if record_out.stderr.contains("/proc/sys/kernel/perf_event_paranoid") {
		return Err(E::error(
			"RR is not configured properly (this is to be expected), kernel.perf_event_paranoid must be <= 1",
		)
		.action("🔐 Auto-configure", configure_kernel_perf_event_paranoid()));
	}
	term::debugger("RR", in_path, &[&rr, "replay", "--", "-q"])
}

async fn configure_kernel_perf_event_paranoid() -> R<()> {
	term::Internal::raw(
		"ICIE Auto-configure RR",
		"echo 'kernel.perf_event_paranoid=1' | pkexec tee -a /etc/sysctl.conf && echo 1 | pkexec tee -a \
		 /proc/sys/kernel/perf_event_paranoid",
	)
}
