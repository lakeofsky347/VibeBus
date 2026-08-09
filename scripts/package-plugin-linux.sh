#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
helper="$script_dir/plugin_package.py"
output_root=$(python3 "$helper" output-root \
    --repo-root "$repo_root" --output-root "${1:-$repo_root/dist}")
version=$(python3 "$helper" version --repo-root "$repo_root" --cargo cargo)
target_directory=$(python3 "$helper" target-directory --repo-root "$repo_root" --cargo cargo)

case "$(uname -m)" in
    aarch64|arm64) package_arch=arm64 ;;
    x86_64|amd64) package_arch=x64 ;;
    *) echo "Unsupported Linux architecture: $(uname -m)" >&2; exit 1 ;;
esac

cargo build --release --locked --package vibebus --bin vibebus
python3 "$helper" stage \
    --repo-root "$repo_root" \
    --output-root "$output_root" \
    --platform linux \
    --arch "$package_arch" \
    --binary "$target_directory/release/vibebus" \
    --cargo cargo >/dev/null

staging_root="$output_root/staging/VibeBus-$version-linux-$package_arch"
python3 "$helper" validate \
    --repo-root "$repo_root" \
    --plugin-root "$staging_root/plugins/vibebus" \
    --marketplace-path "$staging_root/.agents/plugins/marketplace.json" \
    --platform linux \
    --arch "$package_arch" \
    --cargo cargo >/dev/null

printf '{"ok":true,"version":"%s","platform":"linux-%s","staging":"%s"}\n' \
    "$version" "$package_arch" "$staging_root"
