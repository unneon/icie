# Building from source

## Dependencies

First, the build system only works on Linux. It can build both Windows and Linux binaries, athough Windows support is turned off at the moment.

You will need a recent nightly build of the Rust programming language. The recommended installation method is [rustup](https://rustup.rs/). Make sure to select the nightly toolchain during installation. If you already have installed stable Rust via rustup or you have an outdated version, see [official documentation on updating and managing toolchains](https://doc.rust-lang.org/edition-guide/rust-2018/rustup-for-managing-rust-versions.html).

Aside from that, stuff for VS Code extension build system is also required. Install a relatively recent version of node and npm(node 4.x does not work, node 8.x does), which may not be available e.g. in official Ubuntu 16.04 packages. After that, install vsce(`npm install -g vsce` I think).

Also evscode build system requires rsync to work(most likely already installed, if not install with system package manager). Windows builds require more dependencies, but they are turned off for now.

ICIE itself also depends on libdbus-1-dev(probably install with system package manager).

## Building

[Clone](https://help.github.com/en/articles/cloning-a-repository) the repository and run `cargo run --release -- --package`. The built .vsix can be found in `target/release/icie-vscode` directory. In VS Code, go to Extensions and use "Install from VSIX..." option. Plugin built on a newer distro may not work on an older distro - if you intend to distribute the package, compile it using using an old Ubuntu LTS or something.

# Development

In order to launch a debug build, run `cargo run`(without `--release` to shorten the compile times).
Most of the logic resides in the src/ directory. The exceptions are network interactions and VS Code interactions, which live respectively in the unijudge*/ and evscode*/ directory families.
Inside src/, adding new commands or config options will be registered automatically.

To see Rust VS Code API docs, run `cargo doc --open -p evscode`.
Currently, it is hard to add support for uncovered parts of the API(it requires manually running scripts from evscode/backup-ts-env and editing a Typescript file).
The current approach is too boilerplate-heavy and limited anyway and will be replaced with a different solution eventually.

To add support for other competitive programming sites, add a new unijudge-something/ directory and fill it with code similar to unijudge-spoj.
After that, add its metadata to src/net.rs and Cargo.toml, and ICIE will start using it.

To add your changes back to the plugin, open a [pull request](https://help.github.com/en/articles/creating-a-pull-request).
