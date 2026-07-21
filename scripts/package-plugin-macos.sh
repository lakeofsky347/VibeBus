#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
output_root=${1:-"$repo_root/dist"}
signing_identity=${VIBEBUS_CODESIGN_IDENTITY:--}
manifest="$repo_root/plugins/vibebus/.codex-plugin/plugin.json"
version=$(/usr/bin/python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["version"])' "$manifest")
machine=$(uname -m)
case "$machine" in
    arm64) package_arch=arm64 ;;
    x86_64) package_arch=x64 ;;
    *) echo "Unsupported macOS architecture: $machine" >&2; exit 1 ;;
esac

case "$output_root" in
    /*) ;;
    *) output_root="$repo_root/$output_root" ;;
esac
output_root=$(/usr/bin/python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).resolve())' "$output_root")
case "$output_root" in
    "$repo_root"/*) ;;
    *) echo "Output directory must be inside the repository: $output_root" >&2; exit 1 ;;
esac

cargo build --release --locked --bin vibebus
plugin_binary="$repo_root/plugins/vibebus/bin/vibebus"
mkdir -p "$(dirname "$plugin_binary")"
cp "$repo_root/target/release/vibebus" "$plugin_binary"
chmod 755 "$plugin_binary"
if [ "$signing_identity" = "-" ]; then
    signature_kind=adhoc
    codesign --force --identifier dev.vibebus.cli --sign - "$plugin_binary" >/dev/null
else
    signature_kind=identity
    codesign --force --identifier dev.vibebus.cli --options runtime --timestamp \
        --sign "$signing_identity" "$plugin_binary" >/dev/null
fi

mkdir -p "$output_root/staging"
staging_root="$output_root/staging/VibeBus-$version-macos-$package_arch"
if [ -e "$staging_root" ]; then
    rm -rf -- "$staging_root"
fi
mkdir -p "$staging_root/plugins"
cp -R "$repo_root/.agents" "$staging_root/.agents"
cp -R "$repo_root/plugins/vibebus" "$staging_root/plugins/vibebus"
cp "$repo_root/LICENSE" "$staging_root/LICENSE"
cp "$repo_root/README.md" "$staging_root/README.md"
cp "$staging_root/plugins/vibebus/.mcp.macos.json" "$staging_root/plugins/vibebus/.mcp.json"
rm -f -- "$staging_root/plugins/vibebus/.mcp.macos.json"
rm -f -- "$staging_root/plugins/vibebus/bin/vibebus.exe"

"$script_dir/validate-plugin-macos.sh" "$staging_root/plugins/vibebus" >/dev/null

portable="$output_root/VibeBus-$version-macos-$package_arch.tar.gz"
plugin_zip="$output_root/VibeBus-Codex-plugin-$version-macos-$package_arch.zip"
rm -f -- "$portable" "$plugin_zip"
tar -C "$staging_root" -czf "$portable" .agents plugins LICENSE README.md
/usr/bin/ditto -c -k --norsrc --keepParent \
    "$staging_root/plugins/vibebus" "$plugin_zip"

portable_sha=$(shasum -a 256 "$portable" | awk '{print $1}')
plugin_sha=$(shasum -a 256 "$plugin_zip" | awk '{print $1}')
checksums="$output_root/SHA256SUMS-macos-$package_arch.txt"
printf '%s  %s\n%s  %s\n' \
    "$portable_sha" "$(basename "$portable")" \
    "$plugin_sha" "$(basename "$plugin_zip")" > "$checksums"

manifest_path="$output_root/release-manifest-macos-$package_arch.json"
generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
portable_bytes=$(stat -f '%z' "$portable")
plugin_bytes=$(stat -f '%z' "$plugin_zip")
printf '{\n  "version": "%s",\n  "platform": "macos-%s",\n  "signature": "%s",\n  "generatedAt": "%s",\n  "artifacts": [\n    {"name": "%s", "bytes": %s, "sha256": "%s"},\n    {"name": "%s", "bytes": %s, "sha256": "%s"}\n  ],\n  "checksums": "%s"\n}\n' \
    "$version" "$package_arch" "$signature_kind" "$generated_at" \
    "$(basename "$portable")" "$portable_bytes" "$portable_sha" \
    "$(basename "$plugin_zip")" "$plugin_bytes" "$plugin_sha" \
    "$(basename "$checksums")" > "$manifest_path"

printf '{"ok":true,"version":"%s","platform":"macos-%s","staging":"%s","portable":"%s","plugin":"%s","checksums":"%s","manifest":"%s"}\n' \
    "$version" "$package_arch" "$staging_root" "$portable" "$plugin_zip" "$checksums" "$manifest_path"
