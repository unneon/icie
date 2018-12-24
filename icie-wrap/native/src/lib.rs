#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate neon;
extern crate icie_logic;

use neon::{prelude::*, result::Throw};

lazy_static! {
	static ref ICIE: icie_logic::Handle = { icie_logic::Handle::spawn() };
}

struct MessageRecvTask;
impl Task for MessageRecvTask {
	type Error = ();
	type JsEvent = JsObject;
	type Output = icie_logic::Reaction;

	fn perform(&self) -> Result<<Self as Task>::Output, <Self as Task>::Error> {
		let message = ICIE.recv();
		Ok(message)
	}

	fn complete(self, mut cx: TaskContext, result: Result<<Self as Task>::Output, <Self as Task>::Error>) -> Result<Handle<<Self as Task>::JsEvent>, Throw> {
		let obj = JsObject::new(&mut cx);
		let tag = match result.unwrap() {
			icie_logic::Reaction::Status { message } => {
				maybe_set_string(&obj, "message", message, &mut cx);
				"status"
			},
			icie_logic::Reaction::InfoMessage { message } => {
				maybe_set_string(&obj, "message", message, &mut cx);
				"info_message"
			},
			icie_logic::Reaction::ErrorMessage { message } => {
				maybe_set_string(&obj, "message", message, &mut cx);
				"error_message"
			},
			icie_logic::Reaction::QuickPick { items } => {
				let array = JsArray::new(&mut cx, items.len() as u32);
				for (i, item) in items.into_iter().enumerate() {
					let el = cx.empty_object();
					maybe_set_string(&el, "label", item.label, &mut cx);
					maybe_set_string(&el, "description", item.description, &mut cx);
					maybe_set_string(&el, "detail", item.detail, &mut cx);
					maybe_set_string(&el, "id", item.id, &mut cx);
					array.set(&mut cx, i as u32, el)?;
				}
				obj.set(&mut cx, "items", array)?;
				"quick_pick"
			},
			icie_logic::Reaction::InputBox { options } => {
				maybe_set_string(&obj, "prompt", options.prompt, &mut cx);
				maybe_set_string(&obj, "placeHolder", options.placeholder, &mut cx);
				set_bool(&obj, "password", options.password, &mut cx);
				set_bool(&obj, "ignoreFocusOut", options.ignore_focus_out, &mut cx);
				"input_box"
			},
			icie_logic::Reaction::ConsoleLog { message } => {
				maybe_set_string(&obj, "message", message, &mut cx);
				"console_log"
			},
			icie_logic::Reaction::SaveAll => "save_all",
			icie_logic::Reaction::OpenFolder { path, in_new_window } => {
				maybe_set_string(&obj, "path", path.to_str().unwrap().to_owned(), &mut cx);
				set_bool(&obj, "in_new_window", in_new_window, &mut cx);
				"open_folder"
			},
			icie_logic::Reaction::ConsoleError { message } => {
				maybe_set_string(&obj, "message", message, &mut cx);
				"console_error"
			},
		};
		let tag = cx.string(tag);
		obj.set(&mut cx, "tag", tag)?;
		Ok(obj)
	}
}

fn maybe_set_string<S: Into<Option<String>>>(obj: &Handle<JsObject>, field: &str, value: S, cx: &mut TaskContext) {
	if let Some(value) = value.into() {
		let value = cx.string(value);
		obj.set(cx, field, value).unwrap();
	}
}
fn set_bool(obj: &Handle<JsObject>, field: &str, value: bool, cx: &mut TaskContext) {
	let value = cx.boolean(value);
	obj.set(cx, field, value).unwrap();
}
fn get_string_or_bool(obj: &Handle<JsObject>, field: &str, cx: &mut CallContext<JsObject>) -> Option<String> {
	let raw = obj.get(cx, field).unwrap();
	if !raw.is_a::<JsNull>() {
		let value: Handle<JsString> = raw.downcast().unwrap();
		Some(value.value())
	} else {
		None
	}
}

pub fn message_recv(mut cx: FunctionContext) -> JsResult<JsUndefined> {
	let f = cx.argument::<JsFunction>(0)?;
	MessageRecvTask.schedule(f);
	Ok(cx.undefined())
}

pub fn message_send(mut cx: FunctionContext) -> JsResult<JsString> {
	let obj = cx.argument::<JsObject>(0)?;
	let tag = obj.get(&mut cx, "tag")?.to_string(&mut cx)?.value();
	let impulse = match tag.as_str() {
		"quick_pick" => icie_logic::Impulse::QuickPick {
			response: get_string_or_bool(&obj, "response", &mut cx),
		},
		"input_box" => icie_logic::Impulse::InputBox {
			response: get_string_or_bool(&obj, "response", &mut cx),
		},
		"workspace_info" => icie_logic::Impulse::WorkspaceInfo {
			root_path: get_string_or_bool(&obj, "root_path", &mut cx),
		},
		"trigger_build" => icie_logic::Impulse::TriggerBuild,
		"trigger_test" => icie_logic::Impulse::TriggerTest,
		"saved_all" => icie_logic::Impulse::SavedAll,
		"trigger_init" => icie_logic::Impulse::TriggerInit,
		"trigger_submit" => icie_logic::Impulse::TriggerSubmit,
		_ => return Ok(cx.string("Unrecognized tag!")),
	};
	ICIE.send(impulse);
	Ok(cx.string(format!("Message sent successfully {:?}", tag)))
}

register_module!(mut m, {
	m.export_function("message_recv", message_recv)?;
	m.export_function("message_send", message_send)?;
	Ok(())
});
