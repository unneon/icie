use crate::util::{node_hrtime, path::Path, sleep};
use evscode::{E, R};
use futures::{
	channel::{mpsc, oneshot}, future::join3, FutureExt, StreamExt
};
use node_sys::child_process::Stdio;
use std::{
	future::Future, sync::atomic::{AtomicBool, Ordering::SeqCst}, time::Duration
};
use wasm_bindgen::{closure::Closure, JsCast, JsValue, __rt::core::pin::Pin};

#[derive(Debug, Eq, PartialEq)]
pub enum ExitKind {
	Normal,
	TimeLimitExceeded,
}

#[derive(Debug)]
pub struct Run {
	pub stdout: String,
	pub stderr: String,
	pub exit_code: Option<i32>,
	pub exit_kind: ExitKind,
	pub time: Duration,
}
impl Run {
	pub fn success(&self) -> bool {
		self.exit_code == Some(0) && self.exit_kind == ExitKind::Normal
	}
}

#[derive(Debug)]
pub struct Environment {
	pub time_limit: Option<Duration>,
	pub cwd: Option<Path>,
}

#[derive(Debug, Clone)]
pub struct Executable {
	pub command: String,
}

impl Executable {
	pub fn new(path: Path) -> Executable {
		Executable { command: path.to_str().unwrap().to_owned() }
	}

	pub fn new_name(command: String) -> Executable {
		Executable { command }
	}

	pub async fn run(&self, input: &str, args: &[&str], environment: &Environment) -> R<Run> {
		let js_args = js_sys::Array::new();
		for arg in args {
			js_args.push(&JsValue::from_str(arg));
		}
		let input_buffer =
			node_sys::buffer::Buffer::from(js_sys::Uint8Array::from(input.as_bytes()));
		let cwd = environment
			.cwd
			.clone()
			.or_else(|| evscode::workspace_root().ok().map(Path::from_native));
		let kid = node_sys::child_process::spawn(
			&self.command,
			js_args,
			node_sys::child_process::Options {
				cwd: cwd.as_ref().map(|p| p.to_str().unwrap()),
				env: None,
				argv0: None,
				stdio: Some([Stdio::Pipe, Stdio::Pipe, Stdio::Pipe]),
				uid: None,
				gid: None,
				shell: None,
				windows_verbatim_arguments: None,
				windows_hide: None,
			},
		);
		let t1 = node_hrtime();
		// This is not the proper way to check whether an error has happened, but doing otherwise
		// would be ugly. Blame Node for not making a proper asynchronous spawn or throwing an
		// exception.
		if kid.stdin().is_none() {
			let (tx, rx) = oneshot::channel();
			kid.on_2("error", &Closure::once_into_js(|err: js_sys::Error| tx.send(err).unwrap()));
			return Err(E::from(rx.await.unwrap()).context("running solution executable failed"));
		}
		kid.stdin().unwrap().end(&input_buffer, (), Closure::once_into_js(|| {}));
		let capture_stdout = capture_node_stream(kid.stdout().unwrap());
		let capture_stderr = capture_node_stream(kid.stderr().unwrap());
		let execution_finished = AtomicBool::new(false);
		let timed_out = AtomicBool::new(false);
		let drive_exec = async {
			let exit_code = wait_process(&kid).await;
			let t2 = node_hrtime();
			execution_finished.store(true, SeqCst);
			(exit_code, t2)
		};
		let drive_exec = soft_timeout(drive_exec, environment.time_limit, || {
			if !execution_finished.load(SeqCst) {
				timed_out.store(true, SeqCst);
				kid.kill(9);
			}
		});
		let ((exit_code, t2), stdout, stderr) =
			join3(drive_exec, capture_stdout, capture_stderr).await;
		let exit_kind =
			if timed_out.load(SeqCst) { ExitKind::TimeLimitExceeded } else { ExitKind::Normal };
		let stdout = String::from_utf8_lossy(&stdout).into_owned();
		let stderr = String::from_utf8_lossy(&stderr).into_owned();
		Ok(Run { stdout, stderr, exit_code, exit_kind, time: t2 - t1 })
	}
}

async fn wait_process(kid: &node_sys::child_process::ChildProcess) -> Option<i32> {
	let (tx, rx) = oneshot::channel();
	let mut tx = Some(tx);
	kid.on_2(
		"exit",
		&Closure::once_into_js(move |code: JsValue, _signal: JsValue| {
			tx.take().unwrap().send(code.as_f64().map(|code| code as i32)).unwrap()
		}),
	);
	rx.await.unwrap()
}

async fn capture_node_stream(readable: node_sys::stream::Readable) -> Vec<u8> {
	let (tx, mut rx) = mpsc::unbounded();
	let tx2 = tx.clone();
	let end_handler = Closure::wrap(Box::new(move || {
		let _ = tx2.unbounded_send(None);
	}) as Box<dyn FnMut()>);
	readable.on_0("end", &end_handler);
	let readable2 = readable.clone().dyn_into::<node_sys::stream::Readable>().unwrap();
	let readable_handler = Closure::wrap(Box::new(move || {
		while let Some(raw_buf) = readable.read() {
			let js_buf = js_sys::Uint8Array::new(&raw_buf.buffer());
			let mut rust_buf = vec![0u8; js_buf.length() as usize];
			js_buf.copy_to(&mut rust_buf);
			let _ = tx.unbounded_send(Some(rust_buf));
		}
	}) as Box<dyn FnMut()>);
	readable2.on_0("readable", &readable_handler);
	let mut buf = Vec::new();
	while let Some(Some(chunk)) = rx.next().await {
		buf.extend_from_slice(&chunk);
	}
	buf
}

async fn soft_timeout<X>(
	task: impl Future<Output=X>,
	timeout: Option<Duration>,
	on_timeout: impl FnOnce(),
) -> X
{
	let mut task = Box::pin(task).fuse();
	let mut timeout = if let Some(timeout) = timeout {
		Box::pin(sleep(timeout)) as Pin<Box<dyn Future<Output=()>>>
	} else {
		Box::pin(futures::future::pending()) as Pin<Box<dyn Future<Output=()>>>
	}
	.fuse();
	futures::select! {
		x = task => x,
		() = timeout => {
			on_timeout();
			task.await
		},
	}
}
