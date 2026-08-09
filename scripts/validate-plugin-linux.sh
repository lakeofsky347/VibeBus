#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "usage: validate-plugin-linux.sh PLUGIN_ROOT [MARKETPLACE_PATH]" >&2
    exit 2
fi
plugin_root=$1
marketplace=${2:-"$(dirname "$(dirname "$plugin_root")")/.agents/plugins/marketplace.json"}
exec python3 "$script_dir/plugin_package.py" validate \
    --repo-root "$repo_root" \
    --plugin-root "$plugin_root" \
    --marketplace-path "$marketplace" \
    --platform linux \
    --cargo cargo
