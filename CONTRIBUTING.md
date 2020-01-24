# Building from source

## Dependencies

Firstly, the build system was only tested on Linux. Using Windows Subsystem for Linux or MacOS should also work, but who knows. The built extension is cross-platform no matter where you're compiling.

You will need a recent nightly build of the Rust programming language. The recommended installation method is [rustup](https://rustup.rs/). Make sure to select the nightly toolchain during installation. If you already have installed stable Rust via rustup or you have an outdated version, see [official documentation on updating and managing toolchains](https://doc.rust-lang.org/edition-guide/rust-2018/rustup-for-managing-rust-versions.html). The project really requires a recent version, so be sure to run `rustup update` before compiling.

After that, install two tool sets required for WebAssembly development with `cargo install wasm-bindgen-cli wasm-pack`.
Yet another tool is wasm-opt, which can be installed on recent Ubuntus with `sudo apt install binaryen` or compiled from [sources](https://github.com/WebAssembly/binaryen)(clone, cmake . && make && sudo make install).

To just run the extensions, VS Code needs to be installed so that code command works.
Aside from that, if you want to build a .vsix, then stuff for VS Code extension build system is also required. Install a relatively recent version of node and npm(node 4.x does not work, node 8.x does), which may not be available e.g. in official Ubuntu 16.04 packages. After that, install vsce with `sudo npm install -g vsce`.

## Building

[Clone](https://help.github.com/en/articles/cloning-a-repository) the repository and run `BUILDSH_RELEASE=1 ./build.sh package`. The built .vsix can be found in `target/evscode` directory. In VS Code, go to Extensions and use "Install from VSIX..." option.

# Development

In order to launch a debug build, run `./build.sh run`. To quickly check if your changes compile, run `cargo check --target=wasm32-unknown-unknown`.

Most of the logic resides in the src/ directory. The exceptions are network interactions and VS Code interactions, which live respectively in the unijudge*/ and evscode*/ directory families.
Inside src/, adding new commands or config options will be registered automatically.

To see Rust VS Code API docs, run `cargo doc --open -p evscode`.
If you want to use a part of the [official API](https://code.visualstudio.com/api/references/vscode-api) that's not supported yet, then you need to add the JS FFI declarations to vscode-sys/ and a Rust wrapper in evscode/.
This may be challenging, so feel free to ask me for help with this(or any other!) part.

To add support for other competitive programming sites, add a new unijudge-something/ directory and fill it with code similar to unijudge-spoj.
After that, add its metadata to src/net.rs and Cargo.toml, and ICIE will start using it.

To add your changes back to the plugin, open a [pull request](https://help.github.com/en/articles/creating-a-pull-request).
