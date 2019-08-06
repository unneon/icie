use crate::{error::Severity, E, R};
use json::object;
use lazy_static::lazy_static;
use log::LevelFilter;
use std::{
	collections::HashMap, io::BufRead, path::PathBuf, sync::{
		atomic::{AtomicU64, Ordering}, mpsc::Sender, Mutex
	}
};

pub fn execute(pkg: &crate::meta::Package) {
	set_panic_hook();
	let logger = crate::internal::logger::VSCodeLoger { blacklist: pkg.log_filters.iter().map(|(id, fil)| (*id, *fil)).collect() };
	unsafe {
		crate::internal::logger::LOGGER_SLOT = Some(logger);
	}
	log::set_logger(unsafe { crate::internal::logger::LOGGER_SLOT.as_ref().unwrap() }).expect("evscode::execute failed to set logger");
	log::set_max_level(LevelFilter::Trace);
	for line in std::io::stdin().lock().lines() {
		let line = line.expect("evscode::execute line read errored");
		let impulse = json::parse(&line).expect("evscode::execute malformed json");
		if impulse["tag"] == "async" {
			let aid = &impulse["aid"].as_u64();
			let aid = aid.expect("evscode::execute impulse .tag['async'] has no .aid[u64]");
			let value: json::JsonValue = impulse["value"].clone();
			let lck = ASYNC_OPS.lock().expect("evscode::execute ASYNC_OPS PoisonError");
			if let Some(tx) = lck.get(&aid) {
				tx.send(crate::future::Packet::new(aid, value)).expect("evscode::execute async SendError");
			}
		} else if impulse["tag"] == "trigger" {
			let id = &impulse["command_id"].as_str().expect("evscode::execute .tag['trigger'] has no .command_id[str]");
			let command = match pkg.commands.iter().find(|command| &format!("{}.{}", pkg.identifier, command.inner_id) == id) {
				Some(command) => command,
				None => panic!(
					"evscode::execute unknown command {:?}, known: {:?}",
					id,
					pkg.commands.iter().map(|cmd| format!("{}.{}", pkg.identifier, cmd.inner_id)).collect::<Vec<_>>()
				),
			};
			let trigger = command.trigger;
			spawn(trigger);
		} else if impulse["tag"] == "config" {
			let tree = &impulse["tree"];
			let mut errors = Vec::new();
			for config in &pkg.configuration {
				let mut v = tree;
				for part in config.id.split('.') {
					v = &v[part];
				}
				if let Err(e) = config.reference.update(v.clone()) {
					errors.push(format!("{}.{} ({})", pkg.identifier, config.id, e));
				}
			}
			if !errors.is_empty() {
				error_show(crate::E::error(errors.join(", ")).context("some configuration entries are invalid, falling back to defaults"));
			}
		} else if impulse["tag"] == "meta" {
			*WORKSPACE_ROOT.lock().unwrap() = impulse["workspace"].as_str().map(PathBuf::from);
			*EXTENSION_ROOT.lock().unwrap() = Some(PathBuf::from(impulse["extension"].as_str().unwrap()));
			if let Some(on_activate) = &pkg.on_activate {
				spawn(*on_activate);
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

pub fn spawn(f: impl FnOnce() -> R<()>+Send+'static) {
	std::thread::spawn(move || match f() {
		Ok(()) => (),
		Err(e) => error_show(e),
	});
}

pub fn error_show(e: crate::E) {
	let should_show = match e.severity {
		Severity::Error => true,
		Severity::Cancel => false,
		Severity::Warning => true,
		Severity::Workflow => true,
	};
	if should_show {
		{
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
		}
		let should_suggest_report = match e.severity {
			Severity::Error => true,
			Severity::Cancel => false,
			Severity::Warning => true,
			Severity::Workflow => false,
		};
		let message =
			format!("{}{}", e.human(), if should_suggest_report { "; [report issue?](https://github.com/pustaczek/icie/issues/new)" } else { "" });
		let mut msg = crate::Message::new(message).error();
		for (i, action) in e.actions.iter().enumerate() {
			msg = msg.item(i.to_string(), action.title.as_str(), false);
		}
		if let Severity::Warning = e.severity {
			msg = msg.warning();
		}
		let msg = msg.build().spawn();
		if !e.actions.is_empty() {
			std::thread::spawn(move || {
				let choice = msg.wait();
				if let Some(choice) = choice {
					let i: usize = choice.parse().expect("evscode::spawn_trigger invalid action selected");
					let action = &e.actions[i];
					spawn(action.trigger);
				}
			});
		}
	}
}

pub(crate) static ASYNC_ID_FACTORY: IDFactory = IDFactory::new();
pub(crate) static HANDLE_FACTORY: IDFactory = IDFactory::new();

lazy_static! {
	pub(crate) static ref ASYNC_OPS: Mutex<HashMap<u64, Sender<crate::future::Packet>>> = Mutex::new(HashMap::new());
}

lazy_static! {
	pub(crate) static ref WORKSPACE_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);
	pub(crate) static ref EXTENSION_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);
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
