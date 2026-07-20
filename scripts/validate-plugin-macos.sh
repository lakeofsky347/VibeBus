#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
plugin_root=${1:-"$repo_root/plugins/vibebus"}

exec /usr/bin/python3 "$script_dir/validate_plugin_macos.py" "$plugin_root"
