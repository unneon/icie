use crate::{
	meta::{ConfigEntry, LazyOnceFuture, Package}, E
};
use once_cell::sync::OnceCell;
use std::{cell::RefCell, panic::PanicInfo};
use wasm_bindgen::{closure::Closure, JsValue};

mod package_json;

#[doc(hidden)]
pub fn activate(ctx: &vscode_sys::ExtensionContext, mut pkg: Package) {
	std::panic::set_hook(Box::new(panic_hook));
	let on_activate = pkg.on_activate.take();
	let on_deactivate = pkg.on_deactivate.take();
	let telemetry_reporter = vscode_extension_telemetry_sys::TelemetryReporter::new(
		pkg.identifier,
		pkg.version,
		pkg.telemetry_key,
	);
	PACKAGE.set(pkg).map_err(|_| ()).unwrap();
	let pkg = PACKAGE.get().unwrap();
	EXTENSION_CONTEXT.with(|ext_ctx| ext_ctx.set((*ctx).clone()).map_err(|_| ()).unwrap());
	EXTENSION_PATH.set(ctx.get_extension_path()).map_err(|_| ()).unwrap();
	CONFIG_ENTRIES.set(pkg.configuration.clone()).map_err(|_| ()).unwrap();
	crate::stdlib::STATUS.with(|s| s.replace(Some(vscode_sys::window::create_status_bar_item())));
	crate::stdlib::TELEMETRY_REPORTER.with(|tr| tr.replace(Some(telemetry_reporter)));
	for command in &pkg.commands {
		let command_id = command.id.to_string();
		let closure = Box::leak(Box::new(Closure::wrap(Box::new(move || {
			crate::spawn((command.trigger)());
		}) as Box<dyn FnMut()>)));
		vscode_sys::commands::register_command(&command_id, &closure);
	}
	if let Some(on_activate) = on_activate {
		crate::spawn(on_activate());
	}
	if let Some(on_deactivate) = on_deactivate {
		ON_DEACTIVATE.with(|od| od.replace(Some(on_deactivate)));
	}
}

#[doc(hidden)]
pub async fn deactivate() {
	if let Some(on_deactivate) =
		ON_DEACTIVATE.with(|on_deactivate| on_deactivate.borrow_mut().take())
	{
		match on_deactivate().await {
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

pub(crate) static PACKAGE: OnceCell<Package> = OnceCell::new();
pub(crate) static CONFIG_ENTRIES: OnceCell<Vec<ConfigEntry>> = OnceCell::new();
thread_local! {
	pub(crate) static EXTENSION_CONTEXT: OnceCell<JsValue> = OnceCell::new();
}
pub(crate) static EXTENSION_PATH: OnceCell<String> = OnceCell::new();
thread_local! {
	pub(crate) static ON_DEACTIVATE: RefCell<
		Option<Box<LazyOnceFuture>>,
	> = RefCell::new(None);
}
