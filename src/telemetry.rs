use crate::net::BACKENDS;
use std::{
	sync::{
		atomic::{AtomicU64, Ordering}, Mutex
	}, time::Instant
};

pub fn send_usage() {
	evscode::telemetry(
		"usage",
		&[],
		(&[
			("auth_ask", TELEMETRY.auth_ask.get()),
			("auth_keyring_error", TELEMETRY.auth_keyring_error.get()),
			("auth_reset", TELEMETRY.auth_reset.get()),
			("build_all", TELEMETRY.build_all.get()),
			("build_manual", TELEMETRY.build_manual.get()),
			("checker_exists", TELEMETRY.checker_exists.get()),
			("debug_gdb", TELEMETRY.debug_gdb.get()),
			("debug_rr", TELEMETRY.debug_rr.get()),
			("discover_start", TELEMETRY.discover_start.get()),
			("error_unijudge", TELEMETRY.error_unijudge.get()),
			("init_countdown", TELEMETRY.init_countdown.get()),
			("init_countdown_ok", TELEMETRY.init_countdown_ok.get()),
			("init_scan", TELEMETRY.init_scan.get()),
			("init_scan_ok", TELEMETRY.init_scan_ok.get()),
			("init_url", TELEMETRY.init_url.get()),
			("init_url_contest", TELEMETRY.init_url_contest.get()),
			("init_url_existing", TELEMETRY.init_url_existing.get()),
			("init_url_task", TELEMETRY.init_url_task.get()),
			("launch_nearby", TELEMETRY.launch_nearby.get()),
			("launch_web_contest", TELEMETRY.launch_web_contest.get()),
			("launch_web_task", TELEMETRY.launch_web_task.get()),
			("net_connect", TELEMETRY.net_connect.get()),
			("newsletter_show", TELEMETRY.newsletter_show.get()),
			("newsletter_changelog", TELEMETRY.newsletter_changelog.get()),
			("paste_qistruct", TELEMETRY.paste_qistruct.get()),
			("paste_quick", TELEMETRY.paste_quick.get()),
			("paste_quick_ok", TELEMETRY.paste_quick_ok.get()),
			("statement", TELEMETRY.statement.get()),
			("statement_html", TELEMETRY.statement_html.get()),
			("statement_pdf", TELEMETRY.statement_pdf.get()),
			("submit_f12", TELEMETRY.submit_f12.get()),
			("submit_send", TELEMETRY.submit_send.get()),
			("submit_nolang", TELEMETRY.submit_nolang.get()),
			("template_instantiate", TELEMETRY.template_instantiate.get()),
			("template_load", TELEMETRY.template_load.get()),
			("template_load_builtin", TELEMETRY.template_load_builtin.get()),
			("template_load_custom", TELEMETRY.template_load_custom.get()),
			("term_install", TELEMETRY.term_install.get()),
			("test_add", TELEMETRY.test_add.get()),
			("test_alt0", TELEMETRY.test_alt0.get()),
			("test_current", TELEMETRY.test_current.get()),
			("test_input", TELEMETRY.test_input.get()),
			("test_run", TELEMETRY.test_run.get()),
		] as &[(&str, f64)])
			.iter()
			.cloned()
			.chain((&[("session_duration", get_session_duration())]).iter().cloned())
			.chain(BACKENDS.iter().map(|backend| (backend.telemetry_id, backend.counter.get())))
			.chain(
				evscode::runtime::config_entries()
					.iter()
					.map(|config_entry| (config_entry.telemetry_id.as_str(), config_entry.telemetry_config_delta())),
			),
	);
}

pub struct Counter {
	val: AtomicU64,
}
impl Counter {
	pub fn spark(&self) {
		self.val.fetch_add(1, Ordering::SeqCst);
	}

	pub const fn new() -> Counter {
		Counter { val: AtomicU64::new(0) }
	}

	fn get(&self) -> f64 {
		self.val.load(Ordering::SeqCst) as f64
	}
}

lazy_static::lazy_static! {
	pub static ref START_TIME: Mutex<Option<Instant>> = Mutex::new(None);
}

fn get_session_duration() -> f64 {
	let t = Instant::now();
	(t - START_TIME.lock().unwrap().unwrap_or(t)).as_secs_f64()
}

pub struct Events {
	pub auth_ask: Counter,
	pub auth_keyring_error: Counter,
	pub auth_reset: Counter,
	pub build_all: Counter,
	pub build_manual: Counter,
	pub checker_exists: Counter,
	pub debug_gdb: Counter,
	pub debug_rr: Counter,
	pub discover_start: Counter,
	pub error_unijudge: Counter,
	pub init_countdown: Counter,
	pub init_countdown_ok: Counter,
	pub init_scan: Counter,
	pub init_scan_ok: Counter,
	pub init_url: Counter,
	pub init_url_contest: Counter,
	pub init_url_existing: Counter,
	pub init_url_task: Counter,
	pub launch_nearby: Counter,
	pub launch_web_contest: Counter,
	pub launch_web_task: Counter,
	pub net_connect: Counter,
	pub newsletter_show: Counter,
	pub newsletter_changelog: Counter,
	pub paste_qistruct: Counter,
	pub paste_quick: Counter,
	pub paste_quick_ok: Counter,
	pub statement: Counter,
	pub statement_html: Counter,
	pub statement_pdf: Counter,
	pub submit_f12: Counter,
	pub submit_send: Counter,
	pub submit_nolang: Counter,
	pub template_instantiate: Counter,
	pub template_load: Counter,
	pub template_load_builtin: Counter,
	pub template_load_custom: Counter,
	pub term_install: Counter,
	pub test_add: Counter,
	pub test_alt0: Counter,
	pub test_current: Counter,
	pub test_input: Counter,
	pub test_run: Counter,
}

pub static TELEMETRY: Events = Events {
	auth_ask: Counter::new(),
	auth_keyring_error: Counter::new(),
	auth_reset: Counter::new(),
	build_all: Counter::new(),
	build_manual: Counter::new(),
	checker_exists: Counter::new(),
	debug_gdb: Counter::new(),
	debug_rr: Counter::new(),
	discover_start: Counter::new(),
	error_unijudge: Counter::new(),
	init_countdown: Counter::new(),
	init_countdown_ok: Counter::new(),
	init_scan: Counter::new(),
	init_scan_ok: Counter::new(),
	init_url: Counter::new(),
	init_url_contest: Counter::new(),
	init_url_existing: Counter::new(),
	init_url_task: Counter::new(),
	launch_nearby: Counter::new(),
	launch_web_contest: Counter::new(),
	launch_web_task: Counter::new(),
	net_connect: Counter::new(),
	newsletter_show: Counter::new(),
	newsletter_changelog: Counter::new(),
	paste_qistruct: Counter::new(),
	paste_quick: Counter::new(),
	paste_quick_ok: Counter::new(),
	statement: Counter::new(),
	statement_html: Counter::new(),
	statement_pdf: Counter::new(),
	submit_f12: Counter::new(),
	submit_send: Counter::new(),
	submit_nolang: Counter::new(),
	template_instantiate: Counter::new(),
	template_load: Counter::new(),
	template_load_builtin: Counter::new(),
	template_load_custom: Counter::new(),
	term_install: Counter::new(),
	test_add: Counter::new(),
	test_alt0: Counter::new(),
	test_current: Counter::new(),
	test_input: Counter::new(),
	test_run: Counter::new(),
};
