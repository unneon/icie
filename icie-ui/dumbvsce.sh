#!/usr/bin/env bash

HIDING=$(mktemp -d)
TARGET=../icie-wrap/native/target

mv "$TARGET/release" "$HIDING"

vsce $@

mv "$HIDING/release" "$TARGET"

rmdir "$HIDING"
