target_profile = debug

build_asset_exe = $(build_asset_dir)/bin-linux
build_ts_js = $(source_typescript:icie-ui/src/%.ts=icie-ui/out/%.js)
build_npm_install = icie-ui/.npm-install-flag
build_package_readme = icie-ui/README.md
build_asset_dir = icie-ui/assets
build_exe = icie-stdio/target/$(target_profile)/icie-stdio

source_readme = README.md
source_rust = $(source_rust_logic) $(source_rust_stdio)
source_rust_logic = $(wildcard icie-logic/Cargo.toml icie-logic/Cargo.lock icie-logic/src/*.rs icie-logic/src/**/*.rs)
source_rust_stdio = $(wildcard icie-stdio/Cargo.toml icie-stdio/Cargo.lock icie-stdio/src/*.rs icie-logic/src/**/*.rs)
source_js_meta = icie-ui/package.json
source_typescript = $(wildcard icie-ui/src/*.ts icie-ui/src/**/*.ts)
source_typescript_meta = icie-ui/tsconfig.json

tool_cargo = ~/.cargo/bin/cargo

.PHONY: all
all: $(build_asset_exe) $(build_ts_js) $(build_package_readme)

$(build_asset_exe): $(build_exe) $(build_asset_dir)
	cp $(build_exe) $(build_asset_exe)
$(build_ts_js): $(source_typescript) $(source_typescript_meta) $(build_npm_install)
	cd icie-ui && npm run tsc
$(build_npm_install): $(source_js_meta)
	cd icie-ui && npm install
	touch $(build_npm_install)
$(build_package_readme): $(source_readme)
	cp $(source_readme) $(build_package_readme)
$(build_asset_dir):
	mkdir $(build_asset_dir)
# TODO do not hardcode --build
$(build_exe): $(source_rust)
	cd icie-stdio && $(tool_cargo) build
