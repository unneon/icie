#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate neon;
extern crate icie_logic;

use neon::prelude::*;
use neon::result::Throw;

lazy_static! {
	static ref ICIE: icie_logic::ICIE = {
		icie_logic::ICIE::spawn()
	};
}

struct MessageRecvTask;
impl Task for MessageRecvTask {
	type Output = icie_logic::Reaction;
	type Error = ();
	type JsEvent = JsObject;

	fn perform(&self) -> Result<<Self as Task>::Output, <Self as Task>::Error> {
		let message = ICIE.recv();
		Ok(message)
	}

	fn complete(self, mut cx: TaskContext, result: Result<<Self as Task>::Output, <Self as Task>::Error>) -> Result<Handle<<Self as Task>::JsEvent>, Throw> {
		let obj = JsObject::new(&mut cx);
		let tag = match result.unwrap() {
			icie_logic::Reaction::Status { message } => {
				let message = message.map(|message| cx.string(message).as_value(&mut cx)).unwrap_or(cx.null().as_value(&mut cx));
				obj.set(&mut cx, "message", message)?;
				"status"
			},
			icie_logic::Reaction::InfoMessage { message } => {
				let message = cx.string(message);
				obj.set(&mut cx, "message", message)?;
				"info_message"
			},
			icie_logic::Reaction::ErrorMessage { message } => {
				let message = cx.string(message);
				obj.set(&mut cx, "message", message)?;
				"error_message"
			},
			icie_logic::Reaction::QuickPick { items } => {
				let array = JsArray::new(&mut cx, items.len() as u32);
				for (i, item) in items.into_iter().enumerate() {
					let el = cx.empty_object();
					let label = cx.string(item.label);
					el.set(&mut cx, "label", label)?;
					if let Some(description) = item.description {
						let description = cx.string(description);
						el.set(&mut cx, "description", description)?;
					}
					if let Some(detail) = item.detail {
						let detail = cx.string(detail);
						el.set(&mut cx, "detail", detail)?;
					}
					let id = cx.string(item.id);
					el.set(&mut cx, "id", id)?;
					array.set(&mut cx,i as u32, el)?;
				}
				obj.set(&mut cx, "items", array)?;
				"quick_pick"
			},
		};
		let tag = cx.string(tag);
		obj.set(&mut cx, "tag", tag)?;
		Ok(obj)
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
		"ping" => icie_logic::Impulse::Ping,
		"quick_pick" => icie_logic::Impulse::QuickPick {
			response: {
				let response = obj.get(&mut cx, "response")?;
				if !response.is_a::<JsNull>() {
					let response: Handle<JsString> = response.downcast_or_throw(&mut cx)?;
					Some(response.value())
				} else {
					None
				}
			}
		},
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