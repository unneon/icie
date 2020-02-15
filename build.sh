#!/usr/bin/env bash

# An utility function for printing build steps.
# If last exit code was not 0, assumes the step was already ran.
function X {
	echo -e "⭐ \033[1;37m${@}\033[0m"
	${@}
	if [[ "${?}" != "0" ]] ; then
		exit 1
	fi
}

# Dynamically generate package.json, check if it changed and possibly run npm install.
function handle_package_json {
	# Prepare a version of the package that can be used outside of VS Code.
	# This needs to because import vscode does not work anywhere else, but we do not use it at all.
	cp "${dir_wasmpack}/icie_bg.js" "${dir_wasmpack}/icie_bg.wasm" "${dir_wasmpack}/package.json" "${dir_genpackagejson}/"
	cat "${dir_wasmpack}/icie.js" | sed 's/^.*require.*vscode.*$//gm' | sed 's/^.*require.*keytar.*$//g'  > "${dir_genpackagejson}/icie.js"

	# Generate fresh package.json.
	X node -e "require('${dir_genpackagejson}/').internal_generate_package_json('${dir_build}/package.json')"

	# Prepare to run npm install if need be.
	cd "${dir_vscode}/"

	# Check if the new package json differs from the old one.
	test -f "${dir_vscode}/package.json" && diff <(jq -cS . "${dir_build}/package.json") <(jq -cS . "${dir_vscode}/package.json") > /dev/null
	package_json_diff="$?"

	# Copy the new version over.
	cp "${dir_build}/package.json" "${dir_vscode}/"

	# Run npm install if package.json changed.
	[[ "${package_json_diff}" != "0" ]]
	X npm install
}

function generate_vscodeignore {
	f="${dir_vscode}/.vscodeignore"
	rm -f "$f"
	echo build >> "$f"
}

# Prepare some useful paths.
# Root is project root and vscode is the directory where the package will be generated.
# Wasmpack is a directory for creating some temporary files.
dir_root=`realpath "${0}" | xargs dirname`
dir_vscode="${dir_root}/target/evscode"
dir_build="${dir_vscode}/build"
dir_wasmpack="${dir_build}/wasmpack"
dir_genpackagejson="${dir_build}/genpackagejson"

# Check whether we should run in release mode
# Adding -g costs 0.1MB .vsix size and nothing in startup time, but makes backtraces work.
if [ "${BUILDSH_RELEASE}" ] ; then
	wasmpack_profile=""
	wasmopt_profile="-g -O3"
else
	wasmpack_profile="--dev"
	wasmopt_profile="-g -O0"
fi

# Set up necessary directories.
mkdir -p "${dir_root}/" "${dir_vscode}/" "${dir_build}/" "${dir_wasmpack}/" "${dir_genpackagejson}/"

# Compile Rust code with wasm-pack.
# The artifacts live in target/wasm32-unknown-unknown, but wasm-pack processes and copies some to ${dir_wasmpack}.
# Do not quote profile, so it's not interpreted as an empty flag.
cd "${dir_root}/"
X wasm-pack build -d "${dir_wasmpack}" -t nodejs -m no-install ${wasmpack_profile}

# Copy the generated WebAssembly and glue files to the target directory.
# This step may change if WASM some proposals progress further.
cp "${dir_wasmpack}/icie_bg.js" "${dir_wasmpack}/icie_bg.wasm" "${dir_vscode}/"

# Copy and optimize the generated WebAssembly.
X wasm-opt ${wasmopt_profile} -o "${dir_vscode}/icie_bg.wasm" "${dir_wasmpack}/icie_bg.wasm"

# Manually copy and patch icie.js files, because reqwest relies on web-sys so we substitute it with node-sys.
# However, this requires importing the relevant stuff manually.
# Also, there is some annoying type checking so just remove it.
printf "global.fetch = require('node-fetch');\nconst { Headers, Request, Response } = fetch;\n\n" > "${dir_vscode}/icie.js"
cat "${dir_wasmpack}/icie.js" >> "${dir_vscode}/icie.js"
sed -i 's/getObject(arg0) instanceof Window;/true;/mg' "${dir_vscode}/icie.js"

# Copy resources to the target directory.
cp "${dir_root}/README.md" "${dir_root}/CHANGELOG.md" "${dir_root}/icon.png" "${dir_vscode}/"

# Copy runtime assets to the target directory
X rsync -r "${dir_root}/assets" "${dir_vscode}"

handle_package_json
generate_vscodeignore

# Handle CLI commands.
if [[ "${1}" == "run" ]] ; then
	X code --extensionDevelopmentPath "${dir_vscode}"
elif [[ "${1}" == "package" ]] ; then
	cd "${dir_vscode}/"
	X vsce package
fi
