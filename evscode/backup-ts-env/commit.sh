#!/usr/bin/env bash
BACKUP="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

# cp -r "$BACKUP"/files/* "$1"
# cd "$1"
# jq -M '.devDependencies += {"@types/node":"*", "typescript":"*"}' package.json | sponge package.json
# npm install
# tsc -p ./

cd "$1"
tsc -p ./
cp ./src/extension.ts "$BACKUP/files/src/extension.ts"
cp ./out/extension.js "$BACKUP/../src/glue.js"
