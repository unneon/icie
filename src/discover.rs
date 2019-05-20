use crate::ci::{self, fit::Fitness};
use json::object;

#[evscode::command(title = "ICIE Discover", key = "alt+9")]
fn open() -> evscode::R<()> {
	let handle = WEBVIEW.handle()?;
	let lck = handle.lock()?;
	lck.reveal(1);
	Ok(())
}

fn webview_create() -> evscode::R<evscode::Webview> {
	Ok(evscode::Webview::new("icie.discover", "ICIE Discover", 1)
		.enable_scripts()
		.retain_context_when_hidden()
		.create())
}

fn webview_manage(handle: evscode::webview_singleton::Handle) -> evscode::R<()> {
	let (stream, worker_tx) = {
		let view = handle.lock()?;
		view.set_html(render_discover());
		let (worker_tx, worker_rx) = std::sync::mpsc::channel();
		let worker_reports = evscode::LazyFuture::new_worker(move |carrier| worker_thread(carrier, worker_rx));
		let stream = view
			.listener()
			.map(|n| Ok(ManagerMessage::Note(ManagerNote::from(n))))
			.join(worker_reports.map(|r| r.map(ManagerMessage::Report)))
			.cancel_on(view.disposer());
		(stream, worker_tx)
	};

	let mut best_fitness = None;
	let mut paused = false;

	for msg in stream {
		let view = handle.lock()?;
		match msg? {
			ManagerMessage::Note(note) => match note {
				ManagerNote::Start => {
					paused = false;
					worker_tx.send(WorkerOrder::Start)?;
					view.post_message(json::object! { "tag" => "discovery_state", "running" => true, "reset" => false });
				},
				ManagerNote::Pause => {
					paused = true;
					worker_tx.send(WorkerOrder::Pause)?;
					view.post_message(json::object! { "tag" => "discovery_state", "running" => false, "reset" => false });
				},
				ManagerNote::Reset => {
					best_fitness = None;
					paused = false;
					worker_tx.send(WorkerOrder::Reset)?;
					view.post_message(json::object! { "tag" => "discovery_state", "running" => false, "reset" => true });
				},
				ManagerNote::Save { input } => {
					if !paused {
						paused = true;
						worker_tx.send(WorkerOrder::Pause)?;
					}
					view.post_message(json::object! { "tag" => "discovery_state", "running" => false, "reset" => false });
					evscode::spawn(move || add_test_input(input));
				},
			},
			ManagerMessage::Report(report) => match report {
				Ok(row) => {
					let is_failed = row.solution.verdict != ci::test::Verdict::Accepted;
					let new_best = is_failed && best_fitness.map(|bf| bf < row.fitness).unwrap_or(true);
					if new_best {
						best_fitness = Some(row.fitness);
					}
					view.post_message(json::object! {
						"tag" => "discovery_row",
						"number" => row.number,
						"outcome" => match row.solution.verdict {
							ci::test::Verdict::Accepted => "accept",
							ci::test::Verdict::WrongAnswer => "wrong_answer",
							ci::test::Verdict::RuntimeError => "runtime_error",
							ci::test::Verdict::TimeLimitExceeded => "time_limit_exceeded",
							ci::test::Verdict::IgnoredNoOut => "ignored_no_out",
						},
						"fitness" => row.fitness,
						"input" => if new_best { Some(row.input) } else { None },
					});
				},
				Err(e) => {
					best_fitness = None;
					paused = false;
					view.post_message(json::object! { "tag" => "discovery_state", "running" => false, "reset" => true });
					evscode::internal::executor::error_show(e);
				},
			},
		}
	}
	Ok(())
}

fn worker_thread(carrier: evscode::future::Carrier<WorkerReport>, orders: std::sync::mpsc::Receiver<WorkerOrder>) -> evscode::R<()> {
	loop {
		match orders.recv() {
			Ok(WorkerOrder::Start) => (),
			Ok(WorkerOrder::Pause) | Ok(WorkerOrder::Reset) => continue,
			Err(std::sync::mpsc::RecvError) => break,
		};
		match worker_run(&carrier, &orders) {
			Ok(()) => (),
			Err(e) => {
				carrier.send(Err(e));
			},
		}
	}
	Ok(())
}

fn worker_run(carrier: &evscode::future::Carrier<WorkerReport>, orders: &std::sync::mpsc::Receiver<WorkerOrder>) -> evscode::R<()> {
	let solution = crate::build::build(crate::dir::solution()?, ci::lang::Codegen::Debug)?;
	let brut = crate::build::build(crate::dir::brut()?, ci::lang::Codegen::Release)?;
	let gen = crate::build::build(crate::dir::gen()?, ci::lang::Codegen::Release)?;
	let task = ci::task::Task {
		checker: Box::new(ci::task::FreeWhitespaceChecker),
		environment: ci::exec::Environment { time_limit: None },
	};
	let mut _status = crate::STATUS.push("Discovering");
	for number in 1.. {
		match orders.try_recv() {
			Ok(WorkerOrder::Start) => (),
			Ok(WorkerOrder::Pause) => {
				drop(_status);
				loop {
					match orders.recv()? {
						WorkerOrder::Start => break,
						WorkerOrder::Pause => (),
						WorkerOrder::Reset => return Err(evscode::E::cancel()),
					}
				}
				_status = crate::STATUS.push("Discovering");
			},
			Ok(WorkerOrder::Reset) => break,
			Err(std::sync::mpsc::TryRecvError::Empty) => (),
			Err(e) => Err(e)?,
		}
		let run_gen = gen.run("", &task.environment)?;
		if !run_gen.success() {
			return Err(evscode::E::error(format!("test generator failed {:?}", run_gen)));
		}
		let input = run_gen.stdout;
		let run_brut = brut.run(&input, &task.environment)?;
		if !run_brut.success() {
			return Err(evscode::E::error(format!("brut failed {:?}", run_brut)));
		}
		let desired = run_brut.stdout;
		let outcome = ci::test::simple_test(&solution, &input, Some(&desired), &task)?;
		let fitness = ci::fit::ByteLength.evaluate(&input);
		let row = ci::discover::Row {
			number,
			solution: outcome,
			fitness,
			input,
		};
		carrier.send(Ok(row));
	}
	Ok(())
}

fn add_test_input(input: String) -> evscode::R<()> {
	let _status = crate::STATUS.push("Adding new test");
	let brut = crate::build::build(crate::dir::brut()?, ci::lang::Codegen::Release)?;
	let run = brut.run(&input, &ci::exec::Environment { time_limit: None })?;
	if !run.success() {
		return Err(evscode::E::error("brut failed when generating output for the added test"));
	}
	let desired = run.stdout;
	let dir = crate::dir::custom_tests()?;
	std::fs::create_dir_all(&dir)?;
	let used = std::fs::read_dir(&dir)?
		.into_iter()
		.map(|der| {
			der.ok()
				.and_then(|de| de.path().file_stem().map(std::ffi::OsStr::to_owned))
				.and_then(|stem| stem.to_str().map(str::to_owned))
				.and_then(|name| name.parse::<i64>().ok())
		})
		.filter_map(|o| o)
		.collect::<Vec<_>>();
	let id = crate::util::mex(1, used);
	std::fs::write(dir.join(format!("{}.in", id)), input)?;
	std::fs::write(dir.join(format!("{}.out", id)), desired)?;
	crate::test::view()?;
	Ok(())
}

fn render_discover() -> String {
	format!(
		r#"
		<html>
			<head>
				<style>{css}</style>
				{material_icons}
				<script>{js}</script>
			</head>
			<body>
				<div class="container">
					<table class="log">
						<thead>
							<tr>
								<th>Test</th>
								<th>Verdict</th>
								<th>Fitness</th>
							</tr>
						</thead>
						<tbody id="log-body">
							<tr id="current">
								<td>1</td>
								<td></td>
								<td></td>
							</tr>
						</tbody>
					</table>
					<div class="controls">
						<a id="start" class="material-icons control-button" onclick="button_start()">play_arrow</a>
						<a id="pause" class="material-icons control-button" onclick="button_pause()">pause</a>
						<br/>
						<a id="reset" class="material-icons control-button" onclick="button_clear()">clear</a>
					</div>
				</div>
				<br/>
				<div id="best-test-container" class="data">
					<div class="actions">
						<a class="action material-icons" onclick="action_save()">add</a>
					</div>
					<div id="best-test">
					</div>
				</div>
			</body>
		</html>
	"#,
		css = include_str!("discover.css"),
		material_icons = crate::util::html_material_icons(),
		js = include_str!("discover.js"),
	)
}

#[derive(Clone)]
enum ManagerNote {
	Start,
	Pause,
	Reset,
	Save { input: String },
}
#[derive(Clone)]
enum ManagerMessage {
	Note(ManagerNote),
	Report(WorkerReport),
}
enum WorkerOrder {
	Start,
	Pause,
	Reset,
}
type WorkerReport = evscode::R<ci::discover::Row>;

impl From<json::JsonValue> for ManagerNote {
	fn from(val: json::JsonValue) -> ManagerNote {
		match val["tag"].as_str().unwrap() {
			"discovery_start" => ManagerNote::Start,
			"discovery_pause" => ManagerNote::Pause,
			"discovery_reset" => ManagerNote::Reset,
			"discovery_save" => ManagerNote::Save {
				input: String::from(val["input"].as_str().unwrap()),
			},
			_ => panic!("unrecognized ManagerNote .tag"),
		}
	}
}

lazy_static::lazy_static! {
	static ref WEBVIEW: evscode::WebviewSingleton = evscode::WebviewSingleton::new(webview_create, webview_manage);
}
