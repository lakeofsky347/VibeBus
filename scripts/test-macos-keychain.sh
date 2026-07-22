#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
binary=${VIBEBUS_TEST_BINARY:-"$repo_root/target/debug/vibebus"}
if [ ! -x "$binary" ]; then
    cargo build --locked
fi
exec /usr/bin/python3 "$script_dir/test_macos_keychain.py" "$binary"
