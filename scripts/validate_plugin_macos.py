#!/usr/bin/env python3
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(message)


def require_file(path: Path, label: str) -> None:
    if not path.is_file():
        fail(f"{label} does not exist: {path}")


def load_json(path: Path, label: str):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"{label} is not valid JSON: {error}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate_plugin_macos.py PLUGIN_ROOT")
    plugin_root = Path(sys.argv[1]).resolve()
    manifest_path = plugin_root / ".codex-plugin" / "plugin.json"
    mcp_path = plugin_root / ".mcp.json"
    hooks_path = plugin_root / "hooks" / "hooks.json"
    skill_path = plugin_root / "skills" / "vibebus-coordination" / "SKILL.md"
    binary_path = plugin_root / "bin" / "vibebus"
    for path, label in (
        (manifest_path, "Plugin manifest"),
        (mcp_path, "MCP configuration"),
        (hooks_path, "Hook configuration"),
        (skill_path, "Coordination skill"),
        (binary_path, "Packaged executable"),
    ):
        require_file(path, label)

    manifest = load_json(manifest_path, "Plugin manifest")
    if manifest.get("name") != "vibebus":
        fail("Plugin name must be vibebus.")
    version = manifest.get("version", "")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        fail("Plugin version must be semantic X.Y.Z.")
    for field in ("description", "license", "skills", "mcpServers"):
        if not manifest.get(field):
            fail(f"Plugin manifest field '{field}' is required.")
    if not manifest.get("author", {}).get("name"):
        fail("Plugin author.name is required.")
    for field in ("composerIcon", "logo"):
        reference = manifest.get("interface", {}).get(field, "")
        if not re.fullmatch(r"\./assets/[^/]+\.png", reference):
            fail(f"Plugin interface.{field} must be a relative PNG under ./assets/.")
        asset = plugin_root / reference.removeprefix("./")
        require_file(asset, f"Plugin interface.{field} asset")
        if asset.stat().st_size == 0:
            fail(f"Plugin interface.{field} asset is empty: {asset}")

    mcp = load_json(mcp_path, "MCP configuration")
    server = mcp.get("mcpServers", {}).get("vibebus", {})
    if server.get("command") != "./bin/vibebus" or server.get("args") != ["mcp"]:
        fail("macOS MCP configuration must launch ./bin/vibebus mcp.")

    hooks = load_json(hooks_path, "Hook configuration").get("hooks", {})
    expected = {
        "SessionStart": "hook session-start",
        "PostToolUse": "hook post-tool-use",
        "Stop": "hook stop",
    }
    for event, command_fragment in expected.items():
        entries = hooks.get(event, [])
        if len(entries) != 1:
            fail(f"Exactly one {event} hook is required.")
        commands = entries[0].get("hooks", [])
        if len(commands) != 1 or command_fragment not in commands[0].get("command", ""):
            fail(f"{event} must invoke the native macOS VibeBus hook.")
    if hooks["PostToolUse"][0].get("matcher") != "^Bash$":
        fail("PostToolUse must retain the Bash matcher.")

    skill = skill_path.read_text(encoding="utf-8")
    if not re.match(r"^---\r?\nname:\s*vibebus-coordination\r?\ndescription:\s*.+?\r?\n---\r?\n", skill):
        fail("Coordination skill frontmatter is invalid.")

    completed = subprocess.run(
        [str(binary_path), "--version"],
        check=False,
        capture_output=True,
        text=True,
    )
    version_output = completed.stdout.strip()
    if completed.returncode != 0 or version_output != f"vibebus {version}":
        fail(f"Binary version '{version_output}' does not match plugin version '{version}'.")
    signature = subprocess.run(
        ["/usr/bin/codesign", "--verify", "--verbose=2", str(binary_path)],
        check=False,
        capture_output=True,
        text=True,
    )
    if signature.returncode != 0:
        fail("Packaged executable does not have a valid macOS code signature.")
    digest = hashlib.sha256(binary_path.read_bytes()).hexdigest()
    print(
        json.dumps(
            {
                "ok": True,
                "plugin": str(plugin_root),
                "version": version,
                "binary": str(binary_path),
                "binaryBytes": binary_path.stat().st_size,
                "binarySha256": digest,
                "platform": "macos",
            },
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
