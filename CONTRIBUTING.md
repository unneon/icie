# Building from source

## Dependencies

First, the build system only works on Linux. It can build both Windows and Linux binaries, athough Windows support is turned off at the moment.

You will need a recent nightly build of the Rust programming language. The recommended installation method is [rustup](https://rustup.rs/). Make sure to select the nightly toolchain during installation. If you already have installed stable Rust via rustup or you have an outdated version, see [official documentation on updating and managing toolchains](https://doc.rust-lang.org/edition-guide/rust-2018/rustup-for-managing-rust-versions.html).

Aside from that, stuff for VS Code extension build system is also required. Install a relatively recent version of node and npm(node 4.x does not work, node 8.x does), which may not be available e.g. in official Ubuntu 16.04 packages. After that, install vsce(`npm install -g vsce` I think).

Also evscode build system requires rsync to work(most likely already installed, if not install with system package manager). Windows builds require more dependencies, but they are turned off for now.

ICIE itself also depends on libdbus-1-dev(probably install with system package manager).

## Building

[Clone](https://help.github.com/en/articles/cloning-a-repository) the repository and run `cargo run --release -- --package`. The built .vsix can be found in `target/release/icie-vscode` directory. In VS Code, go to Extensions and use "Install from VSIX..." option. Plugin built on a newer distro may not work on an older distro - if you intend to distribute the package, compile it using the [Dockerfile](deploy/Dockerfile).

# Development

In order to launch a debug build, run `cargo run`(without `--release` to shorten the compile times). The plugin uses code from several repositories, so depending on what you want to modify you may have to clone them too and [override the dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#overriding-repository-url) in ICIE's Cargo.toml, which means appending the two lines from the link. The connected repositories are:

- **icie** This is the most important repository, where the all building, testing, configurating, UI and networking happens. New command/config entries will be handled by the build system automatically if they have `evscode::command` or `evscode::config` attribute. To see the docs for VS Code API, run `cargo doc --open -p evscode` - the most important stuff is in the `stdlib` module. To add support to new competitive programming sites, implement it like in `unijudge-codeforces` and add it to backend list in `src/net.rs` and `Cargo.toml`.
- [**evscode**](https://github.com/pustaczek/evscode) This is where the VS Code API is defined. Adding new stuff here is kind of a pain because I have not automated TypeScript compilation yet and it requires running the scripts from `backup-ts-env/` and launching the extension manually. I will probably change it to run on WASM when it becomes usable.

To add your changes back to the plugin, open a [pull request](https://help.github.com/en/articles/creating-a-pull-request).
