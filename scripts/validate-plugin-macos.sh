#!/bin/sh
set -eu
export PYTHONDONTWRITEBYTECODE=1

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "usage: validate-plugin-macos.sh PLUGIN_ROOT [MARKETPLACE_PATH]" >&2
    exit 2
fi
plugin_root=$1

if [ "$#" -eq 2 ]; then
    exec /usr/bin/python3 "$script_dir/validate_plugin_macos.py" "$plugin_root" "$2"
fi
exec /usr/bin/python3 "$script_dir/validate_plugin_macos.py" "$plugin_root"
