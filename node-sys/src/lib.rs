macro_rules! wasm_abi_serde {
	($t:ty) => {
		impl wasm_bindgen::convert::IntoWasmAbi for $t {
			type Abi = <wasm_bindgen::JsValue as wasm_bindgen::convert::IntoWasmAbi>::Abi;

			fn into_abi(self) -> Self::Abi {
				wasm_bindgen::JsValue::from_serde(&self).unwrap().into_abi()
			}
		}

		impl wasm_bindgen::describe::WasmDescribe for $t {
			fn describe() {
				<wasm_bindgen::JsValue as wasm_bindgen::describe::WasmDescribe>::describe()
			}
		}
	};
}

// TODO: Change all synchronous calls to asynchronous.

pub mod buffer {

	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = buffer)]
	extern "C" {

		pub type Buffer;

		#[wasm_bindgen(static_method_of = Buffer)]
		pub fn from(buffer: js_sys::Uint8Array) -> Buffer;

		#[wasm_bindgen(method, getter)]
		pub fn buffer(this: &Buffer) -> js_sys::ArrayBuffer;

	}
}

pub mod child_process {

	use crate::stream::{Readable, Writable};
	use serde::{Serialize, Serializer};
	use std::collections::HashMap;
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = child_process)]
	extern "C" {

		pub type ChildProcess;

		#[wasm_bindgen(method)]
		pub fn kill(this: &ChildProcess, signal: i32);

		#[wasm_bindgen(method, js_name = on)]
		pub fn on_2(this: &ChildProcess, event: &str, callback: &JsValue);

		#[wasm_bindgen(method, getter)]
		pub fn stdin(this: &ChildProcess) -> Option<Writable>;

		#[wasm_bindgen(method, getter)]
		pub fn stdout(this: &ChildProcess) -> Option<Readable>;

		#[wasm_bindgen(method, getter)]
		pub fn stderr(this: &ChildProcess) -> Option<Readable>;

		pub fn spawn(command: &str, args: js_sys::Array, options: Options) -> ChildProcess;

	}

	#[derive(Serialize)]
	pub struct Options<'a> {
		pub cwd: Option<&'a str>,
		pub env: Option<HashMap<String, String>>,
		pub argv0: Option<&'a str>,
		/// Respectively stdin, stdout, stderr.
		pub stdio: Option<[Stdio; 3]>,
		pub uid: Option<i64>,
		pub gid: Option<i64>,
		pub shell: Option<Shell<'a>>,
		#[serde(rename = "windowsVerbatimArguments")]
		pub windows_verbatim_arguments: Option<bool>,
		#[serde(rename = "windowsHide")]
		pub windows_hide: Option<bool>,
	}
	wasm_abi_serde!(Options<'_>);

	pub enum Stdio {
		Pipe,
		Ignore,
		Inherit,
	}
	impl Serialize for Stdio {
		fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> where {
			s.serialize_str(match self {
				Stdio::Pipe => "pipe",
				Stdio::Ignore => "ignore",
				Stdio::Inherit => "inherit",
			})
		}
	}

	#[derive(Serialize)]
	#[serde(untagged)]
	pub enum Shell<'a> {
		Use(bool),
		Custom(&'a str),
	}
}

pub mod console {

	use wasm_bindgen::prelude::*;

	#[wasm_bindgen]
	extern "C" {

		#[wasm_bindgen(js_namespace = console)]
		pub fn debug(message: &str);
		#[wasm_bindgen(js_namespace = console)]
		pub fn error(message: &str);
		#[wasm_bindgen(js_namespace = console)]
		pub fn info(message: &str);
		#[wasm_bindgen(js_namespace = console)]
		pub fn log(message: &str);
		#[wasm_bindgen(js_namespace = console)]
		pub fn warn(message: &str);

	}
}

/// Node.js [fs](https://nodejs.org/api/fs.html)
pub mod fs {

	use crate::buffer::Buffer;
	use serde::Serialize;
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = fs)]
	extern "C" {

		pub type Stats;

		pub fn access(path: &str, callback: JsValue);

		pub fn mkdir(path: &str, options: MkdirOptions, callback: JsValue);

		pub fn readdir(path: &str, options: ReaddirOptions, callback: JsValue);

		#[wasm_bindgen(js_name = readFile)]
		pub fn read_file(path: &str, options: ReadFileOptions, callback: JsValue);

		pub fn stat(path: &str, options: StatOptions, callback: JsValue);

		pub fn unlink(path: &str, callback: JsValue);

		#[wasm_bindgen(js_name = unlinkSync)]
		pub fn unlink_sync(path: &str);

		#[wasm_bindgen(js_name = writeFile)]
		pub fn write_file(file: &str, data: Buffer, options: WriteFileOptions, callback: JsValue);

		/// Node.js [fs.writeFileSync](https://nodejs.org/api/fs.html#fs_fs_writefilesync_file_data_options)
		#[wasm_bindgen(js_name = writeFileSync, catch)]
		pub fn write_file_sync(file: &str, data: &str) -> Result<(), JsValue>;

	}

	#[derive(Serialize)]
	pub struct MkdirOptions {
		pub mode: Option<u32>,
	}
	wasm_abi_serde!(MkdirOptions);

	#[derive(Serialize)]
	pub struct ReaddirOptions<'a> {
		pub encoding: Option<&'a str>,
		#[serde(rename = "withFileTypes")]
		pub with_file_types: Option<bool>,
	}
	wasm_abi_serde!(ReaddirOptions<'_>);

	/// Options for [fs.readFileSync](https://nodejs.org/api/fs.html#fs_fs_readfilesync_path_options)
	#[derive(Serialize)]
	pub struct ReadFileOptions<'a> {
		pub encoding: Option<&'a str>,
		pub flag: &'a str,
	}
	wasm_abi_serde!(ReadFileOptions<'_>);

	#[derive(Serialize)]
	pub struct StatOptions {
		pub bigint: bool,
	}
	wasm_abi_serde!(StatOptions);

	#[derive(Serialize)]
	pub struct WriteFileOptions<'a> {
		pub encoding: Option<&'a str>,
		pub mode: Option<u32>,
		pub flag: Option<&'a str>,
	}
	wasm_abi_serde!(WriteFileOptions<'_>);
}

/// Node.js [os](https://nodejs.org/api/os.html)
pub mod os {

	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = os)]
	extern "C" {

		/// Node.js [os.homedir](https://nodejs.org/api/os.html#os_os_homedir)
		pub fn homedir() -> String;

		pub fn tmpdir() -> String;

	}
}

/// Node.js [path](https://nodejs.org/api/path.html)
pub mod path {

	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = path)]
	extern "C" {

		pub fn basename(path: &str) -> String;

		#[wasm_bindgen(js_name = basename)]
		pub fn basename_with_ext(path: &str, ext: &str) -> String;

		#[wasm_bindgen(js_name = delimiter)]
		pub static DELIMITER: js_sys::JsString;

		pub fn dirname(path: &str) -> String;

		pub fn extname(path: &str) -> String;

		pub fn join(a: &str, b: &str) -> String;

		pub fn normalize(path: &str) -> String;

		pub fn relative(from: &str, to: &str) -> String;

	}
}

/// Node.js [process](https://nodejs.org/api/process.html)
pub mod process {

	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = process)]
	extern "C" {

		#[wasm_bindgen(js_name = arch)]
		pub static ARCH: String;

		#[wasm_bindgen(js_name = env)]
		pub static ENV: JsValue;

		pub fn hrtime() -> js_sys::Array;

		#[wasm_bindgen(js_name = platform)]
		pub static PLATFORM: String;

	}
}

pub mod stream {

	use crate::buffer::Buffer;
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = stream)]
	extern "C" {

		pub type Readable;

		#[wasm_bindgen(method, js_name = on)]
		pub fn on_0(this: &Readable, event: &str, callback: &Closure<dyn FnMut()>);

		#[wasm_bindgen(method)]
		pub fn read(this: &Readable) -> Option<Buffer>;

		pub type Writable;

		#[wasm_bindgen(method)]
		pub fn end(this: &Writable, chunk: &Buffer, encoding: (), callback: JsValue);

		#[wasm_bindgen(method)]
		pub fn write(this: &Writable, chunk: &Buffer, encoding: (), callback: JsValue);

	}
}

pub mod timers {

	use wasm_bindgen::prelude::*;

	#[wasm_bindgen(module = timers)]
	extern "C" {

		#[wasm_bindgen(js_name = setTimeout)]
		pub fn set_timeout(callback: JsValue, delay: f64);

	}
}
