use crate::{error::Severity, future::BoxedFuture, meta::ConfigEntry, runtime::spawn, stdlib::message::Action, E, R};
use futures::{
	channel::{mpsc, oneshot}, stream::StreamExt, FutureExt
};
use json::{object, JsonValue};
use lazy_static::lazy_static;
use log::LevelFilter;
use std::{
	collections::HashMap, path::PathBuf, sync::{
		atomic::{AtomicU64, Ordering}, Mutex
	}
};
use tokio::{
	io::{stdin, AsyncBufReadExt, BufReader}, runtime::current_thread
};

pub fn execute(pkg: &'static mut crate::meta::Package) {
	set_panic_hook();
	let logger = crate::internal::logger::VSCodeLoger { blacklist: pkg.log_filters.iter().map(|(id, fil)| (*id, *fil)).collect() };
	log::set_boxed_logger(Box::new(logger)).expect("evscode::execute failed to set logger");
	log::set_max_level(LevelFilter::Trace);
	let mut runtime = current_thread::Runtime::new().unwrap();
	*CONFIG_ENTRIES.lock().unwrap() = Some(&pkg.configuration);
	*RUNTIME_HANDLE.lock().unwrap() = Some(Box::leak(Box::new(runtime.handle())));
	let on_activate = pkg.on_activate.take();
	let on_deactivate = pkg.on_deactivate.take();
	runtime.spawn(comms_loop(pkg, on_activate, on_deactivate));
	runtime.run().expect("executor run error");
}

async fn comms_loop(
	pkg: &'static crate::meta::Package,
	mut on_activate: Option<BoxedFuture<'static, R<()>>>,
	mut on_deactivate: Option<BoxedFuture<'static, R<()>>>,
) {
	let mut lines = BufReader::new(stdin()).lines();
	while let Some(line) = lines.next().await {
		let line = line.expect("evscode::execute line read errored");
		let mut impulse = json::parse(&line).expect("evscode::execute malformed json");
		if impulse["tag"] == "async" {
			let aid = impulse["aid"].as_u64().expect("evscode::execute impulse .tag['async'] has no .aid[u64]");
			let value = impulse["value"].take();
			let mut future_lock = VSCODE_FUTURES.lock().expect("evscode::execute VSCODE_FUTURES PoisonError");
			if let Some(entry) = future_lock.remove(&aid) {
				let _ = entry.send(value);
			} else {
				let mut stream_lock = VSCODE_STREAMS.lock().expect("evscode::execute VSCODE_STREAMS PoisonError");
				if let Some(entry) = stream_lock.get_mut(&aid) {
					let _ = entry.unbounded_send(value);
				}
			}
		} else if impulse["tag"] == "trigger" {
			let id = impulse["command_id"].as_str().expect("evscode::execute .tag['trigger'] has no .command_id[str]");
			let command = match pkg.commands.iter().find(|command| command.id.to_string() == id) {
				Some(command) => command,
				None => panic!("evscode::execute unknown command {:?}, known: {:?}", id, pkg.commands.iter().map(|cmd| cmd.id).collect::<Vec<_>>()),
			};
			spawn((command.trigger)());
		} else if impulse["tag"] == "config" {
			let tree = &impulse["tree"];
			let mut errors = Vec::new();
			for config in &pkg.configuration {
				let mut v = tree;
				for part in config.id.to_string().split('.').skip(1) {
					v = &v[part];
				}
				if let Err(e) = config.reference.update(v.clone()) {
					errors.push(format!("{}.{} ({})", pkg.identifier, config.id, e));
				}
			}
			if !errors.is_empty() {
				crate::E::error(errors.join(", ")).context("some configuration entries are invalid, falling back to defaults").emit();
			}
		} else if impulse["tag"] == "meta" {
			*ROOT_WORKSPACE.lock().unwrap() = impulse["workspace"].as_str().map(PathBuf::from);
			*ROOT_EXTENSION.lock().unwrap() = Some(PathBuf::from(impulse["extension"].as_str().unwrap()));
			if let Some(on_activate) = on_activate.take() {
				spawn(on_activate);
			}
		} else if impulse["tag"] == "dispose" {
			if let Some(on_deactivate) = on_deactivate.take() {
				spawn(on_deactivate.map(|r| {
					if let Err(e) = r {
						error_show(e);
					}
					kill();
					Ok(())
				}));
			} else {
				kill();
			}
		} else {
			send_object(object! {
				"tag" => "console_error",
				"message" => json::stringify(impulse),
			});
		}
	}
}

pub fn send_object(obj: json::JsonValue) {
	let fmt = json::stringify(obj);
	println!("{}", fmt);
}

pub fn error_show(e: crate::E) {
	let should_show = match e.severity {
		Severity::Error => true,
		Severity::Cancel => false,
		Severity::Warning => true,
		Severity::Workflow => true,
	};
	if should_show {
		let mut log_msg = String::new();
		for reason in &e.reasons {
			log_msg += &format!("{}\n", reason);
		}
		for detail in &e.details {
			log_msg += &format!("{}\n", detail);
		}
		log_msg += &format!("\nContains {} extended log entries\n\n{:?}", e.extended.len(), e.backtrace);
		log::error!("{}", log_msg);
		for extended in &e.extended {
			log::info!("{}", extended);
		}
		let should_suggest_report = match e.severity {
			Severity::Error => true,
			Severity::Cancel => false,
			Severity::Warning => true,
			Severity::Workflow => false,
		};
		spawn(async move {
			let message =
				format!("{}{}", e.human(), if should_suggest_report { "; [report issue?](https://github.com/pustaczek/icie/issues)" } else { "" });
			let items = e
				.actions
				.iter()
				.enumerate()
				.map(|(i, action)| Action { id: i.to_string(), title: &action.title, is_close_affordance: false })
				.collect::<Vec<_>>();
			let mut msg = crate::Message::new(&message).error().items(items);
			if let Severity::Warning = e.severity {
				msg = msg.warning();
			}
			let choice = msg.show().await;
			if let Some(choice) = choice {
				let i: usize = choice.parse().unwrap();
				let action = e.actions.into_iter().nth(i).unwrap();
				action.trigger.await?;
			}
			Ok(())
		});
	}
}

fn kill() {
	send_object(object! {
		"tag" => "kill",
	});
}

pub(crate) static ASYNC_ID_FACTORY: IDFactory = IDFactory::new();
pub(crate) static HANDLE_FACTORY: IDFactory = IDFactory::new();

pub(crate) type PacketChannelAwait = oneshot::Sender<JsonValue>;
pub(crate) type PacketChannelStream = mpsc::UnboundedSender<JsonValue>;

lazy_static! {
	pub(crate) static ref CONFIG_ENTRIES: Mutex<Option<&'static [ConfigEntry]>> = Mutex::new(None);
	pub(crate) static ref RUNTIME_HANDLE: Mutex<Option<&'static current_thread::Handle>> = Mutex::new(None);
	pub(crate) static ref ROOT_WORKSPACE: Mutex<Option<PathBuf>> = Mutex::new(None);
	pub(crate) static ref ROOT_EXTENSION: Mutex<Option<PathBuf>> = Mutex::new(None);
	pub(crate) static ref VSCODE_FUTURES: Mutex<HashMap<u64, PacketChannelAwait>> = Mutex::new(HashMap::new());
	pub(crate) static ref VSCODE_STREAMS: Mutex<HashMap<u64, PacketChannelStream>> = Mutex::new(HashMap::new());
}

pub(crate) fn runtime_handle() -> &'static current_thread::Handle {
	&RUNTIME_HANDLE.lock().unwrap().unwrap()
}

fn set_panic_hook() {
	std::panic::set_hook(Box::new(move |info| {
		let payload = if let Some(payload) = info.payload().downcast_ref::<&str>() {
			(*payload).to_owned()
		} else if let Some(payload) = info.payload().downcast_ref::<String>() {
			payload.clone()
		} else {
			"...".to_owned()
		};
		let location = if let Some(location) = info.location() { format!("{}:{}", location.file(), location.line()) } else { "--:--".to_owned() };
		error_show(E::error(format!("ICIE panicked, {} at {}", payload, location)));
	}));
}

pub struct IDFactory {
	counter: AtomicU64,
}
impl IDFactory {
	pub const fn new() -> IDFactory {
		IDFactory { counter: AtomicU64::new(0) }
	}

	pub fn generate(&self) -> u64 {
		self.counter.fetch_add(1, Ordering::Relaxed)
	}
}
