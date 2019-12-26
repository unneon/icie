use crate::{
	error::R, meta::{ConfigEntry, Package}, BoxFuture, E
};
use log::LevelFilter;
use std::{cell::RefCell, panic::PanicInfo};
use wasm_bindgen::{closure::Closure, JsValue};

mod package_json;

#[doc(hidden)]
pub fn activate(ctx: &vscode_sys::ExtensionContext, pkg: Package) {
	let logger = crate::logger::VSCodeLoger {
		blacklist: pkg.log_filters.iter().map(|(id, fil)| (*id, *fil)).collect(),
	};
	log::set_boxed_logger(Box::new(logger)).expect("evscode::execute failed to set logger");
	log::set_max_level(LevelFilter::Trace);
	std::panic::set_hook(Box::new(panic_hook));
	let telemetry_reporter = vscode_extension_telemetry_sys::TelemetryReporter::new(
		pkg.identifier,
		pkg.version,
		pkg.telemetry_key,
	);
	EXTENSION_CONTEXT.with(|ec| ec.replace(Some((*ctx).clone())));
	EXTENSION_PATH.with(|ep| ep.replace(Some(ctx.get_extension_path())));
	CONFIG_ENTRIES.with(|ce| ce.replace(Some(pkg.configuration.clone())));
	crate::stdlib::STATUS.with(|s| s.replace(Some(vscode_sys::window::create_status_bar_item())));
	crate::stdlib::TELEMETRY_REPORTER.with(|tr| tr.replace(Some(telemetry_reporter)));
	for command in pkg.commands {
		let command_id = command.id.to_string();
		let closure = Box::leak(Box::new(Closure::wrap(Box::new(move || {
			crate::spawn((command.trigger)());
		}) as Box<dyn FnMut()>)));
		vscode_sys::commands::register_command(&command_id, &closure);
	}
	if let Some(on_activate) = pkg.on_activate {
		crate::spawn(on_activate);
	}
	if let Some(on_deactivate) = pkg.on_deactivate {
		ON_DEACTIVATE.with(|od| od.replace(Some(on_deactivate)));
	}
}

#[doc(hidden)]
pub async fn deactivate() {
	let on_deactivate = ON_DEACTIVATE.with(|od| od.borrow_mut().take());
	if let Some(on_deactivate) = on_deactivate {
		match on_deactivate.await {
			Ok(()) => (),
			Err(e) => e.emit(),
		}
	}
}

#[doc(hidden)]
pub fn generate_package_json(path: &str, pkg: Package) {
	let pkg = Box::leak(Box::new(pkg));
	let package_json = package_json::construct_package_json(pkg);
	node_sys::fs::write_file_sync(path, &serde_json::to_string_pretty(&package_json).unwrap())
		.unwrap();
}

fn panic_hook(info: &PanicInfo) {
	let payload = if let Some(payload) = info.payload().downcast_ref::<&str>() {
		(*payload).to_owned()
	} else if let Some(payload) = info.payload().downcast_ref::<String>() {
		payload.clone()
	} else {
		"???".to_owned()
	};
	let location = info.location().map_or("???".to_owned(), |location| {
		format!("{}:{}:{}", location.file(), location.line(), location.column())
	});
	E::error(format!("ICIE panicked, {} at {}", payload, location)).emit();
}

thread_local! {
	pub(crate) static CONFIG_ENTRIES: RefCell<Option<Vec<ConfigEntry>>> = RefCell::new(None);
	pub(crate) static EXTENSION_CONTEXT: RefCell<Option<JsValue>> = RefCell::new(None);
	pub(crate) static EXTENSION_PATH: RefCell<Option<String>> = RefCell::new(None);
	pub(crate) static ON_DEACTIVATE: RefCell<Option<BoxFuture<'static, R<()>>>> = RefCell::new(None);
}
