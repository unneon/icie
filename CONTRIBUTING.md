# Building from source

## Dependencies

First, the build system only works on Linux. It can build both Windows and Linux binaries, athough Windows support is turned off at the moment.

You will need a recent nightly build of the Rust programming language. The recommended installation method is [rustup](https://rustup.rs/). Make sure to select the nightly toolchain during installation. If you already have installed stable Rust via rustup or you have an outdated version, see [official documentation on updating and managing toolchains](https://doc.rust-lang.org/edition-guide/rust-2018/rustup-for-managing-rust-versions.html).

Aside from that, stuff for VS Code extension build system is also required. Install npm(probably using your system package manger) and vsce(`npm install -g vsce` I think).

Also evscode build system requires rsync to work(most likely already installed, if not install with system package manager). Windows builds require more dependencies, but they are turned off for now.

## Building

[Clone](https://help.github.com/en/articles/cloning-a-repository) the repository and run `cargo run --release -- --package`. The built .vsix can be found in `target/release/icie-vscode` directory. In VS Code, go to Extensions and use "Install from VSIX..." option.

# Development

In order to launch a debug build, run `cargo run`(without `--release` to shorten the compile times). The plugin uses code from 5 various repositories, so depending on what you want to modify you may have to clone them too and [override the dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#overriding-repository-url) in ICIE's Cargo.toml, which means appending the two lines from the link. The connected repositories are:

- **icie** This is the most important repository, where are the builds, tests, configuration entries and VS Code interactions live. New command/config entries will be handled by the build system automatically if they have `evscode::command` or `evscode::config` attribute. To see complete-but-very-minimal docs for VS Code API, run `cargo doc --open --no-deps -p evscode` - the most important stuff is in the `stdlib` module.
- [**evscode**](https://github.com/pustaczek/evscode) This is where the VS Code API is defined. Adding new stuff here is kind of a pain because I have not automated TypeScript compilation yet and it requires running the scripts from `backup-ts-env/` and launching the extension manually. I will probably change it to run on WASM when it becomes usable.
- [**unijudge**](https://github.com/pustaczek/unijudge) The unified API for various programming sites. In order to add support to a new site, create a crate similar to the codeforces repo and add appropriate impls to unijudge(the API will change soon when I figure out caching sessions properly).
- [**codeforces**](https://github.com/pustaczek/codeforces) The unijudge backend for Codeforces.
- [**sio2**](https://github.com/pustaczek/sio2) The unijudge backend for an obscure Polish judge system.

To add your changes back to the plugin, open a [pull request](https://help.github.com/en/articles/creating-a-pull-request).
