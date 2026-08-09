#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
helper="$script_dir/plugin_package.py"
output_root=$(/usr/bin/python3 "$helper" output-root \
    --repo-root "$repo_root" --output-root "${1:-$repo_root/dist}")
signing_identity=${VIBEBUS_CODESIGN_IDENTITY:--}
version=$(/usr/bin/python3 "$helper" version --repo-root "$repo_root" --cargo cargo)
target_directory=$(/usr/bin/python3 "$helper" target-directory --repo-root "$repo_root" --cargo cargo)

case "$(uname -m)" in
    arm64) package_arch=arm64 ;;
    x86_64) package_arch=x64 ;;
    *) echo "Unsupported macOS architecture: $(uname -m)" >&2; exit 1 ;;
esac

cargo build --release --locked --package vibebus --bin vibebus
/usr/bin/python3 "$helper" stage \
    --repo-root "$repo_root" \
    --output-root "$output_root" \
    --platform macos \
    --arch "$package_arch" \
    --binary "$target_directory/release/vibebus" \
    --cargo cargo >/dev/null

staging_root="$output_root/staging/VibeBus-$version-macos-$package_arch"
staged_plugin="$staging_root/plugins/vibebus"
plugin_binary="$staged_plugin/bin/vibebus"
marketplace="$staging_root/.agents/plugins/marketplace.json"
if [ "$signing_identity" = "-" ]; then
    signature_kind=adhoc
    /usr/bin/codesign --force --identifier dev.vibebus.cli --sign - "$plugin_binary" >/dev/null
else
    signature_kind=identity
    /usr/bin/codesign --force --identifier dev.vibebus.cli --options runtime --timestamp \
        --sign "$signing_identity" "$plugin_binary" >/dev/null
fi

/usr/bin/python3 "$helper" validate \
    --repo-root "$repo_root" \
    --plugin-root "$staged_plugin" \
    --marketplace-path "$marketplace" \
    --platform macos \
    --arch "$package_arch" \
    --cargo cargo >/dev/null

portable="$output_root/VibeBus-$version-macos-$package_arch.tar.gz"
plugin_zip="$output_root/VibeBus-Codex-plugin-$version-macos-$package_arch.zip"
/usr/bin/python3 "$helper" archive \
    --repo-root "$repo_root" \
    --output-root "$output_root" \
    --staging-root "$staging_root" \
    --portable "$portable" \
    --plugin "$plugin_zip" \
    --portable-format tar.gz >/dev/null
/usr/bin/python3 "$helper" validate-archive \
    --repo-root "$repo_root" --archive "$portable" --kind portable \
    --platform macos --arch "$package_arch" --cargo cargo >/dev/null
/usr/bin/python3 "$helper" validate-archive \
    --repo-root "$repo_root" --archive "$plugin_zip" --kind plugin \
    --platform macos --arch "$package_arch" --cargo cargo >/dev/null

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
