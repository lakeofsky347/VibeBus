#!/usr/bin/env python3
"""Build and validate platform-specific VibeBus Codex plugin staging trees.

The source plugin is a template only. Platform binaries and the authoritative
`.mcp.json` are written exclusively below the requested staging directory.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform as host_platform
import re
import shutil
import stat
import struct
import subprocess
import sys
import tarfile
import tempfile
import unicodedata
import uuid
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


PLUGIN_TOP_LEVEL = {
    ".codex-plugin",
    ".mcp.json",
    "LICENSE",
    "README.md",
    "agents",
    "assets",
    "bin",
    "commands",
    "hooks",
    "scripts",
    "skills",
}
SOURCE_DIRECTORIES = (
    ".codex-plugin",
    "assets",
    "hooks",
    "skills",
    "scripts",
    "agents",
    "commands",
)
SOURCE_FILES = ("README.md", "LICENSE")
DENIED_SEGMENTS = {
    ".git",
    ".cache",
    "__pycache__",
    "build",
    "cache",
    "debug",
    "node_modules",
    "obj",
    "object",
    "objects",
    "release",
    "src",
    "target",
}
DENIED_NAMES = {
    "cargo.lock",
    "cargo.toml",
    ".mcp.macos.json",
    ".mcp.linux.json",
    ".mcp.windows.json",
}
DENIED_SUFFIXES = (
    ".a",
    ".d",
    ".dll",
    ".dylib",
    ".log",
    ".o",
    ".obj",
    ".pdb",
    ".py",
    ".pyc",
    ".rlib",
    ".rmeta",
    ".rs",
    ".so",
    ".swp",
    ".temp",
    ".tmp",
    ".dsym",
)
PLATFORM_COMMAND = {
    "macos": "./bin/vibebus",
    "linux": "./bin/vibebus",
    "windows": "./bin/vibebus.exe",
}
PLATFORM_BINARY = {
    "macos": "vibebus",
    "linux": "vibebus",
    "windows": "vibebus.exe",
}
ARCH_ALIASES = {
    "x64": "x64",
    "amd64": "x64",
    "x86_64": "x64",
    "arm64": "arm64",
    "aarch64": "arm64",
}
MAX_ARCHIVE_MEMBERS = 10_000
MAX_ARCHIVE_FILE_BYTES = 128 * 1024 * 1024
MAX_ARCHIVE_TOTAL_BYTES = 512 * 1024 * 1024


def fail(message: str) -> None:
    raise SystemExit(message)


def load_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"{label} is not valid JSON: {error}")


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def require_file(path: Path, label: str) -> None:
    if not path.is_file() or is_link_or_reparse(path):
        fail(f"{label} must be a regular file: {path}")


def normalize_arch(value: str | None = None) -> str:
    machine = (value or host_platform.machine()).lower()
    arch = ARCH_ALIASES.get(machine)
    if arch is None:
        fail(f"Unsupported package architecture: {machine}")
    return arch


def is_link_or_reparse(path: Path) -> bool:
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return False
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    file_attributes = getattr(metadata, "st_file_attributes", 0)
    return path.is_symlink() or bool(file_attributes & reparse_flag)


def reject_reparse_ancestors(path: Path, label: str) -> None:
    current = path.absolute()
    while True:
        if is_link_or_reparse(current):
            fail(f"{label} must not traverse a symlink or reparse point: {current}")
        if current == current.parent:
            break
        current = current.parent


def cargo_package(repo_root: Path, cargo: str) -> dict[str, Any]:
    completed = subprocess.run(
        [cargo, "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=repo_root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        fail(f"cargo metadata failed: {completed.stderr.strip()}")
    try:
        metadata = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        fail(f"cargo metadata returned invalid JSON: {error}")
    manifest = (repo_root / "Cargo.toml").resolve()
    matches = [
        package
        for package in metadata.get("packages", [])
        if package.get("name") == "vibebus"
        and Path(package.get("manifest_path", "")).resolve() == manifest
    ]
    if len(matches) != 1:
        fail(f"Expected exactly one root vibebus Cargo package, found {len(matches)}.")
    version = matches[0].get("version", "")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        fail(f"Cargo package version is not stable semantic X.Y.Z: {version}")
    return matches[0]


def cargo_target_directory(repo_root: Path, cargo: str) -> Path:
    completed = subprocess.run(
        [cargo, "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=repo_root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        fail(f"cargo metadata failed: {completed.stderr.strip()}")
    try:
        target_directory = json.loads(completed.stdout)["target_directory"]
    except (json.JSONDecodeError, KeyError, TypeError) as error:
        fail(f"cargo metadata has no valid target_directory: {error}")
    return Path(target_directory).resolve()


def assert_source_versions(repo_root: Path, version: str) -> None:
    manifests = (
        repo_root / "plugins" / "vibebus" / ".codex-plugin" / "plugin.json",
        repo_root / "plugins" / "cursor" / ".cursor-plugin" / "plugin.json",
    )
    for path in manifests:
        if path.is_file():
            actual = load_json(path, f"Source manifest {path}").get("version")
            if actual != version:
                fail(f"Source manifest version drift: {path} has {actual}, Cargo has {version}.")


def ensure_output_root(repo_root: Path, value: str) -> Path:
    candidate = Path(value)
    if not candidate.is_absolute():
        candidate = repo_root / candidate
    unresolved = candidate.absolute()
    reject_reparse_ancestors(unresolved, "Output directory")
    resolved = candidate.resolve()
    try:
        resolved.relative_to(repo_root.resolve())
    except ValueError:
        fail(f"Output directory must be inside the repository: {resolved}")
    if resolved == repo_root.resolve():
        fail("Output directory must not be the repository root.")
    dist_root = (repo_root / "dist").resolve()
    try:
        resolved.relative_to(dist_root)
    except ValueError:
        fail(f"Output directory must be dist/ or a descendant of dist/: {resolved}")
    return resolved


def ensure_no_symlinks(root: Path, label: str) -> None:
    if is_link_or_reparse(root):
        fail(f"{label} must not be a symlink or reparse point: {root}")
    for path in root.rglob("*"):
        if is_link_or_reparse(path):
            fail(f"{label} must not contain symlinks or reparse points: {path.relative_to(root)}")


def stage_plugin(args: argparse.Namespace) -> None:
    repo_root = Path(args.repo_root).resolve()
    package = cargo_package(repo_root, args.cargo)
    version = package["version"]
    assert_source_versions(repo_root, version)
    target_platform = args.platform
    arch = normalize_arch(args.arch)
    output_root = ensure_output_root(repo_root, args.output_root)
    source_plugin = repo_root / "plugins" / "vibebus"
    marketplace_source = repo_root / ".agents" / "plugins" / "marketplace.json"
    mcp_template = source_plugin / f".mcp.{target_platform}.json"
    binary_source = Path(args.binary).resolve()
    require_file(binary_source, "Built VibeBus executable")
    require_file(marketplace_source, "Source marketplace manifest")
    require_file(mcp_template, f"{target_platform} MCP template")
    ensure_no_symlinks(source_plugin, "Source plugin")
    source_bin = source_plugin / "bin"
    if source_bin.exists() and any(source_bin.iterdir()):
        fail("Source plugin bin must be empty; packaging writes binaries only to staging.")

    staging_root = output_root / "staging" / f"VibeBus-{version}-{target_platform}-{arch}"
    reject_reparse_ancestors(staging_root, "Staging directory")
    expected_parent = (output_root / "staging").resolve()
    staging_resolved = staging_root.resolve()
    if staging_resolved.parent != expected_parent:
        fail(f"Refusing unsafe staging path: {staging_resolved}")
    if staging_root.exists():
        shutil.rmtree(staging_root)
    staged_plugin = staging_root / "plugins" / "vibebus"
    staged_plugin.mkdir(parents=True)
    (staging_root / ".agents" / "plugins").mkdir(parents=True)

    for directory in SOURCE_DIRECTORIES:
        source = source_plugin / directory
        if source.is_dir():
            shutil.copytree(source, staged_plugin / directory)
    for filename in SOURCE_FILES:
        source = source_plugin / filename
        if source.is_file():
            shutil.copy2(source, staged_plugin / filename)
    shutil.copy2(repo_root / "LICENSE", staging_root / "LICENSE")
    shutil.copy2(repo_root / "README.md", staging_root / "README.md")

    manifest_path = staged_plugin / ".codex-plugin" / "plugin.json"
    manifest = load_json(manifest_path, "Staged plugin manifest")
    manifest["version"] = version
    write_json(manifest_path, manifest)
    shutil.copy2(mcp_template, staged_plugin / ".mcp.json")

    staged_binary = staged_plugin / "bin" / PLATFORM_BINARY[target_platform]
    staged_binary.parent.mkdir()
    shutil.copy2(binary_source, staged_binary)
    if target_platform != "windows":
        staged_binary.chmod(0o755)

    marketplace_path = staging_root / ".agents" / "plugins" / "marketplace.json"
    marketplace = load_json(marketplace_source, "Source marketplace manifest")
    matches = [item for item in marketplace.get("plugins", []) if item.get("name") == "vibebus"]
    if len(matches) != 1:
        fail("Source marketplace must contain exactly one VibeBus plugin.")
    matches[0].setdefault("source", {})["path"] = "./plugins/vibebus"
    write_json(marketplace_path, marketplace)

    print(
        json.dumps(
            {
                "ok": True,
                "version": version,
                "platform": f"{target_platform}-{arch}",
                "staging": str(staging_root),
                "plugin": str(staged_plugin),
                "binary": str(staged_binary),
                "marketplace": str(marketplace_path),
            },
            separators=(",", ":"),
        )
    )


def validate_payload(plugin_root: Path, target_platform: str) -> None:
    if not plugin_root.is_dir() or plugin_root.is_symlink():
        fail(f"Plugin root must be a real directory: {plugin_root}")
    unexpected = sorted({path.name for path in plugin_root.iterdir()} - PLUGIN_TOP_LEVEL)
    if unexpected:
        fail(f"Final {target_platform} plugin contains disallowed top-level paths: {unexpected}")
    expected_binary = PLATFORM_BINARY[target_platform]
    expected_files = {
        ".codex-plugin/plugin.json",
        ".mcp.json",
        "README.md",
        "assets/vibebus-icon-dark.png",
        "assets/vibebus-icon-light.png",
        "assets/vibebus-logo-dark.png",
        "assets/vibebus-logo-light.png",
        f"bin/{expected_binary}",
        "hooks/hooks.json",
        "scripts/hook-common.ps1",
        "scripts/post-tool-facts.ps1",
        "scripts/session-start.ps1",
        "scripts/stop-handoff.ps1",
        "skills/vibebus-coordination/SKILL.md",
    }
    actual_files = {
        path.relative_to(plugin_root).as_posix()
        for path in plugin_root.rglob("*")
        if path.is_file()
    }
    if actual_files != expected_files:
        fail(
            "Final plugin file allowlist mismatch: "
            f"missing={sorted(expected_files - actual_files)}, "
            f"unexpected={sorted(actual_files - expected_files)}"
        )
    for path in plugin_root.rglob("*"):
        relative = path.relative_to(plugin_root)
        if is_link_or_reparse(path):
            fail(f"Final plugin must not contain symlinks or reparse points: {relative}")
        for part in relative.parts:
            lowered = part.lower()
            if lowered in DENIED_SEGMENTS or lowered in DENIED_NAMES:
                fail(f"Final plugin contains build/source residue: {relative}")
            if lowered.endswith(DENIED_SUFFIXES):
                fail(f"Final plugin contains build/source residue: {relative}")
            if lowered.startswith("._") or lowered == ".ds_store":
                fail(f"Final plugin contains platform metadata: {relative}")
            if lowered == ".mcp.json" and relative.as_posix() != ".mcp.json":
                fail(f"Final plugin contains a nested MCP configuration: {relative}")
        if path.is_file() and path.parent == plugin_root / "bin" and path.name != expected_binary:
            fail(f"Final plugin contains a cross-platform executable: {relative}")
    bin_root = plugin_root / "bin"
    entries = sorted(path.name for path in bin_root.iterdir()) if bin_root.is_dir() else []
    if entries != [expected_binary]:
        fail(f"Final {target_platform} bin must contain only {expected_binary}, found: {entries}")


def validate_staging_root(plugin_root: Path) -> Path:
    staging_root = plugin_root.parents[1]
    if not staging_root.is_dir() or is_link_or_reparse(staging_root):
        fail(f"Staging root must be a real directory: {staging_root}")
    ensure_no_symlinks(staging_root, "Staging root")
    allowed_root = {".agents", "LICENSE", "README.md", "plugins"}
    entries = {path.name for path in staging_root.iterdir()}
    if entries != allowed_root:
        fail(f"Staging root must contain exactly {sorted(allowed_root)}, found {sorted(entries)}")
    marketplace = staging_root / ".agents" / "plugins" / "marketplace.json"
    require_file(staging_root / "LICENSE", "Staging LICENSE")
    require_file(staging_root / "README.md", "Staging README")
    require_file(marketplace, "Staged marketplace manifest")
    if {path.name for path in (staging_root / ".agents").iterdir()} != {"plugins"}:
        fail("Staging .agents must contain only plugins/.")
    if {path.name for path in marketplace.parent.iterdir()} != {"marketplace.json"}:
        fail("Staging .agents/plugins must contain only marketplace.json.")
    if {path.name for path in (staging_root / "plugins").iterdir()} != {"vibebus"}:
        fail("Staging plugins must contain only vibebus/.")
    for path in staging_root.rglob("*"):
        if path.name.startswith("._") or path.name == ".DS_Store":
            fail(f"Staging root contains AppleDouble or Finder metadata: {path.relative_to(staging_root)}")
    return marketplace


def validate_manifest(plugin_root: Path, version: str) -> dict[str, Any]:
    path = plugin_root / ".codex-plugin" / "plugin.json"
    require_file(path, "Plugin manifest")
    manifest = load_json(path, "Plugin manifest")
    if manifest.get("name") != "vibebus" or manifest.get("version") != version:
        fail(f"Plugin manifest must identify vibebus {version}.")
    if manifest.get("mcpServers") != "./.mcp.json":
        fail("Plugin manifest must reference ./.mcp.json.")
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
    return manifest


def validate_mcp_config(plugin_root: Path, target_platform: str) -> None:
    path = plugin_root / ".mcp.json"
    require_file(path, "MCP configuration")
    servers = load_json(path, "MCP configuration").get("mcpServers", {})
    if set(servers) != {"vibebus"}:
        fail("MCP configuration must contain exactly the vibebus server.")
    server = servers.get("vibebus", {})
    if server != {"cwd": ".", "command": PLATFORM_COMMAND[target_platform], "args": ["mcp"]}:
        fail(
            f"{target_platform} MCP configuration must launch "
            f"{PLATFORM_COMMAND[target_platform]} mcp from cwd ."
        )
    alternates = sorted(path.name for path in plugin_root.iterdir() if path.name.lower().startswith(".mcp.") and path.name != ".mcp.json")
    if alternates:
        fail(f"Final plugin contains alternate MCP configurations: {alternates}")


def validate_hooks_and_skill(plugin_root: Path) -> None:
    hooks_path = plugin_root / "hooks" / "hooks.json"
    skill_path = plugin_root / "skills" / "vibebus-coordination" / "SKILL.md"
    require_file(hooks_path, "Hook configuration")
    require_file(skill_path, "Coordination skill")
    hooks = load_json(hooks_path, "Hook configuration").get("hooks", {})
    expected = {
        "SessionStart": ("hook session-start", "session-start.ps1"),
        "PostToolUse": ("hook post-tool-use", "post-tool-facts.ps1"),
        "Stop": ("hook stop", "stop-handoff.ps1"),
    }
    if set(hooks) != set(expected):
        fail("Hooks must contain exactly SessionStart, PostToolUse, and Stop.")
    for event, (unix_fragment, windows_fragment) in expected.items():
        entries = hooks[event]
        commands = entries[0].get("hooks", []) if len(entries) == 1 else []
        command = commands[0] if len(commands) == 1 else {}
        if unix_fragment not in command.get("command", ""):
            fail(f"{event} must invoke the native VibeBus hook on Unix.")
        if windows_fragment not in command.get("commandWindows", ""):
            fail(f"{event} must invoke {windows_fragment} on Windows.")
    if hooks["PostToolUse"][0].get("matcher") != "^Bash$":
        fail("PostToolUse must retain the Bash matcher.")
    for script_name in ("session-start.ps1", "post-tool-facts.ps1", "stop-handoff.ps1", "hook-common.ps1"):
        require_file(plugin_root / "scripts" / script_name, f"Hook script {script_name}")
    skill = skill_path.read_text(encoding="utf-8")
    if not re.match(r"^---\r?\nname:\s*vibebus-coordination\r?\ndescription:\s*.+?\r?\n---\r?\n", skill):
        fail("Coordination skill frontmatter is invalid.")


def validate_binary_format(binary: Path, target_platform: str, arch: str) -> None:
    require_file(binary, "Packaged executable")
    header = binary.read_bytes()[:512]
    if target_platform == "macos":
        if header[:4] not in {
            b"\xfe\xed\xfa\xce",
            b"\xce\xfa\xed\xfe",
            b"\xfe\xed\xfa\xcf",
            b"\xcf\xfa\xed\xfe",
            b"\xca\xfe\xba\xbe",
            b"\xbe\xba\xfe\xca",
        }:
            fail("Packaged executable is not Mach-O.")
        info = subprocess.run(["/usr/bin/file", "-b", str(binary)], check=False, capture_output=True, text=True)
        expected = "arm64" if arch == "arm64" else "x86_64"
        if info.returncode != 0 or "Mach-O" not in info.stdout or expected not in info.stdout:
            fail(f"Packaged executable is not a {expected} Mach-O binary: {info.stdout.strip()}")
    elif target_platform == "linux":
        if len(header) < 20 or header[:4] != b"\x7fELF" or header[4] != 2 or header[5] != 1:
            fail("Packaged Linux executable must be little-endian ELF64.")
        machine = struct.unpack_from("<H", header, 18)[0]
        expected = 183 if arch == "arm64" else 62
        if machine != expected:
            fail(f"Packaged Linux executable has e_machine={machine}, expected {expected}.")
    else:
        if len(header) < 64 or header[:2] != b"MZ":
            fail("Packaged Windows executable has no DOS/PE header.")
        pe_offset = struct.unpack_from("<I", header, 0x3C)[0]
        with binary.open("rb") as stream:
            stream.seek(pe_offset)
            pe_header = stream.read(6)
        if pe_header[:4] != b"PE\x00\x00":
            fail("Packaged Windows executable has an invalid PE signature.")
        machine = struct.unpack_from("<H", pe_header, 4)[0]
        expected = 0xAA64 if arch == "arm64" else 0x8664
        if machine != expected:
            fail(f"Packaged Windows executable has PE machine=0x{machine:04x}, expected 0x{expected:04x}.")


def mcp_smoke(binary: Path, plugin_root: Path, version: str) -> int:
    with tempfile.TemporaryDirectory(prefix="vibebus-package-mcp-") as temporary:
        acceptance_root = Path(temporary)
        project_root = acceptance_root / "project"
        data_home = acceptance_root / "data"
        project_root.mkdir()
        data_home.mkdir()
        initialized_project = subprocess.run(
            [
                str(binary),
                "init",
                "--root",
                str(project_root),
                "--data-home",
                str(data_home),
                "--name",
                "VibeBus package smoke",
            ],
            cwd=plugin_root,
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
        if initialized_project.returncode != 0:
            fail(f"Packaged binary could not initialize an isolated project: {initialized_project.stderr.strip()}")
        messages = (
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "vibebus-plugin-validator", "version": "1"},
                },
            },
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "vibebus_status",
                    "arguments": {"root": str(project_root)},
                },
            },
        )
        payload = "\n".join(json.dumps(message, separators=(",", ":")) for message in messages) + "\n"
        try:
            completed = subprocess.run(
                [str(binary), "mcp", "--root", str(project_root), "--data-home", str(data_home)],
                cwd=plugin_root,
                input=payload,
                check=False,
                capture_output=True,
                text=True,
                timeout=15,
            )
        except subprocess.TimeoutExpired:
            fail("MCP smoke test timed out.")
        if completed.returncode != 0:
            fail(f"MCP smoke test failed: {completed.stderr.strip()}")
    responses: dict[int, dict[str, Any]] = {}
    for line in completed.stdout.splitlines():
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            fail(f"MCP stdout contained non-JSON data: {error}: {line[:200]!r}")
        if message.get("jsonrpc") != "2.0":
            fail(f"MCP response has no JSON-RPC 2.0 marker: {message}")
        if isinstance(message.get("id"), int):
            responses[message["id"]] = message
    for response_id in (1, 2, 3):
        if response_id not in responses or "error" in responses[response_id]:
            fail(f"MCP did not return a successful response for id {response_id}: {responses.get(response_id)}")
    server_info = responses.get(1, {}).get("result", {}).get("serverInfo", {})
    if server_info.get("name") != "vibebus" or server_info.get("version") != version:
        fail(f"MCP initialize serverInfo is invalid: {server_info}")
    tools = responses.get(2, {}).get("result", {}).get("tools", [])
    names = {tool.get("name") for tool in tools if isinstance(tool, dict)}
    required = {"vibebus_status", "vibebus_doctor"}
    if len(tools) < 40 or not required.issubset(names):
        fail(f"MCP tools/list is incomplete: count={len(tools)}, missing={sorted(required - names)}")
    status_result = responses[3].get("result", {})
    status_content = status_result.get("content", [])
    status_text = "\n".join(
        item.get("text", "") for item in status_content if isinstance(item, dict)
    )
    if status_result.get("isError") is not False or "VibeBus package smoke" not in status_text:
        fail("MCP vibebus_status did not read the isolated package-smoke project.")
    return len(tools)


def validate_marketplace(path: Path) -> None:
    require_file(path, "Staged marketplace manifest")
    marketplace = load_json(path, "Staged marketplace manifest")
    matches = [item for item in marketplace.get("plugins", []) if item.get("name") == "vibebus"]
    if len(matches) != 1 or matches[0].get("source", {}).get("path") != "./plugins/vibebus":
        fail("Staged marketplace must point to ./plugins/vibebus.")


def validate_plugin(
    repo_root: Path,
    plugin_root: Path,
    target_platform: str,
    arch: str,
    cargo: str,
    marketplace_path: Path | None,
    scope: str,
) -> dict[str, Any]:
    package = cargo_package(repo_root, cargo)
    version = package["version"]
    if target_platform == "windows":
        reject_reparse_ancestors(plugin_root, "Windows plugin root")
    if is_link_or_reparse(plugin_root):
        fail(f"Plugin root must not be a symlink or reparse point: {plugin_root}")
    plugin_root = plugin_root.resolve()
    validate_payload(plugin_root, target_platform)
    if scope == "stage":
        discovered_marketplace = validate_staging_root(plugin_root)
        if marketplace_path is not None and marketplace_path.resolve() != discovered_marketplace.resolve():
            fail("Marketplace path must be the canonical file inside the staging root.")
        marketplace_path = discovered_marketplace
        package_name = plugin_root.parents[1].name
        expected_name = f"VibeBus-{version}-{target_platform}-{arch}"
        if package_name != expected_name:
            fail(f"Staging root must be named {expected_name}, found {package_name}.")
    validate_manifest(plugin_root, version)
    validate_mcp_config(plugin_root, target_platform)
    validate_hooks_and_skill(plugin_root)
    binary = plugin_root / "bin" / PLATFORM_BINARY[target_platform]
    if target_platform != "windows" and stat.S_IMODE(binary.stat().st_mode) != 0o755:
        fail(f"{target_platform} executable must have mode 0755.")
    validate_binary_format(binary, target_platform, arch)
    version_result = subprocess.run([str(binary), "--version"], cwd=plugin_root, check=False, capture_output=True, text=True)
    output = version_result.stdout.strip()
    if version_result.returncode != 0 or output != f"vibebus {version}":
        fail(f"Binary version '{output}' does not match plugin/Cargo version '{version}'.")
    if target_platform == "macos":
        signature = subprocess.run(
            ["/usr/bin/codesign", "--verify", "--verbose=2", str(binary)],
            check=False,
            capture_output=True,
            text=True,
        )
        if signature.returncode != 0:
            fail("Packaged executable does not have a valid macOS code signature.")
    tool_count = mcp_smoke(binary, plugin_root, version)
    if marketplace_path is not None:
        validate_marketplace(marketplace_path.resolve())
    return {
        "ok": True,
        "plugin": str(plugin_root),
        "version": version,
        "binary": str(binary),
        "binaryBytes": binary.stat().st_size,
        "binarySha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "platform": f"{target_platform}-{arch}",
        "tools": tool_count,
        "scope": scope,
    }


def validate_command(args: argparse.Namespace) -> None:
    result = validate_plugin(
        Path(args.repo_root).resolve(),
        Path(args.plugin_root),
        args.platform,
        normalize_arch(args.arch),
        args.cargo,
        Path(args.marketplace_path) if args.marketplace_path else None,
        args.scope,
    )
    print(json.dumps(result, separators=(",", ":")))


def iter_tree(root: Path) -> Iterable[Path]:
    yield root
    yield from sorted(root.rglob("*"), key=lambda path: path.as_posix())


def create_zip(path: Path, roots: Iterable[tuple[Path, str]]) -> None:
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for root, prefix in roots:
            for item in iter_tree(root):
                relative = item.relative_to(root)
                archive_name = PurePosixPath(prefix, *relative.parts).as_posix()
                archive.write(item, archive_name)


def archive_command(args: argparse.Namespace) -> None:
    repo_root = Path(args.repo_root).resolve()
    output_root = ensure_output_root(repo_root, args.output_root)
    staging_requested = Path(args.staging_root).absolute()
    reject_reparse_ancestors(staging_requested, "Staging directory")
    staging_root = staging_requested.resolve()
    staging_parent = (output_root / "staging").resolve()
    if staging_root.parent != staging_parent:
        fail(f"Staging root must be an immediate child of {staging_parent}.")
    ensure_no_symlinks(staging_root, "Staging root")
    portable_requested = Path(args.portable).absolute()
    plugin_requested = Path(args.plugin).absolute()
    reject_reparse_ancestors(portable_requested, "Portable archive output")
    reject_reparse_ancestors(plugin_requested, "Plugin archive output")
    portable = portable_requested.resolve()
    plugin_archive = plugin_requested.resolve()
    if portable == plugin_archive:
        fail("Portable and plugin archive paths must be different.")
    for path in (portable, plugin_archive):
        if path.parent != output_root:
            fail(f"Archive output must be an immediate child of {output_root}: {path}")
        if is_link_or_reparse(path):
            fail(f"Archive output must not be a symlink or reparse point: {path}")
    temporary_portable = output_root / f".{portable.name}.{uuid.uuid4().hex}.tmp"
    temporary_plugin = output_root / f".{plugin_archive.name}.{uuid.uuid4().hex}.tmp"
    try:
        if args.portable_format == "tar.gz":
            with tarfile.open(temporary_portable, "w:gz", format=tarfile.PAX_FORMAT, dereference=False) as archive:
                for name in (".agents", "plugins", "LICENSE", "README.md"):
                    archive.add(staging_root / name, arcname=name, recursive=True)
        else:
            create_zip(temporary_portable, ((staging_root / name, name) for name in (".agents", "plugins", "LICENSE", "README.md")))
        create_zip(temporary_plugin, ((staging_root / "plugins" / "vibebus", "vibebus"),))
        os.replace(temporary_portable, portable)
        os.replace(temporary_plugin, plugin_archive)
    finally:
        for temporary_path in (temporary_portable, temporary_plugin):
            if temporary_path.exists():
                temporary_path.unlink()
    print(json.dumps({"ok": True, "portable": str(portable), "plugin": str(plugin_archive)}, separators=(",", ":")))


def safe_member_name(name: str, allowed_roots: set[str]) -> PurePosixPath:
    if "\\" in name or "\x00" in name or any(ord(character) < 32 for character in name):
        fail(f"Archive contains a non-portable member name: {name!r}")
    candidate = name[:-1] if name.endswith("/") else name
    raw_parts = candidate.split("/")
    if candidate.startswith("/") or not candidate or any(part in {"", ".", ".."} for part in raw_parts):
        fail(f"Archive contains path traversal: {name!r}")
    pure = PurePosixPath(candidate)
    if pure.is_absolute() or not pure.parts:
        fail(f"Archive contains path traversal: {name!r}")
    if pure.parts[0] not in allowed_roots:
        fail(f"Archive contains a disallowed root path: {name!r}")
    if any(part.startswith("._") or part == ".DS_Store" for part in pure.parts):
        fail(f"Archive contains AppleDouble or Finder metadata: {name!r}")
    reserved = {"con", "prn", "aux", "nul", *(f"com{index}" for index in range(1, 10)), *(f"lpt{index}" for index in range(1, 10))}
    for part in pure.parts:
        if any(character in part for character in '<>:"|?*'):
            fail(f"Archive contains a Windows-unsafe member name: {name!r}")
        if part.endswith((" ", ".")):
            fail(f"Archive contains a Windows-normalized member name: {name!r}")
        if part.split(".", 1)[0].casefold() in reserved:
            fail(f"Archive contains a reserved Windows device name: {name!r}")
    return pure


def collision_key(path: PurePosixPath) -> str:
    return unicodedata.normalize("NFC", path.as_posix()).casefold()


def extract_checked_archive(archive_path: Path, destination: Path, kind: str) -> None:
    allowed = {".agents", "plugins", "LICENSE", "README.md"} if kind == "portable" else {"vibebus"}
    seen: set[str] = set()
    total_bytes = 0
    if tarfile.is_tarfile(archive_path):
        with tarfile.open(archive_path, "r:*") as archive:
            members = archive.getmembers()
            if len(members) > MAX_ARCHIVE_MEMBERS:
                fail(f"Archive contains too many members: {len(members)}")
            for member in members:
                pure = safe_member_name(member.name, allowed)
                key = collision_key(pure)
                if key in seen:
                    fail(f"Archive contains a duplicate or case-colliding path: {member.name}")
                seen.add(key)
                if member.issym() or member.islnk() or not (member.isdir() or member.isfile()):
                    fail(f"Archive contains a link or unsupported member: {member.name}")
                target = destination.joinpath(*pure.parts)
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                    continue
                if member.size > MAX_ARCHIVE_FILE_BYTES:
                    fail(f"Archive member exceeds the size limit: {member.name}")
                total_bytes += member.size
                if total_bytes > MAX_ARCHIVE_TOTAL_BYTES:
                    fail("Archive expands beyond the total size limit.")
                target.parent.mkdir(parents=True, exist_ok=True)
                source = archive.extractfile(member)
                if source is None:
                    fail(f"Archive file has no payload: {member.name}")
                with source, target.open("wb") as output:
                    shutil.copyfileobj(source, output)
                target.chmod(stat.S_IMODE(member.mode))
    elif zipfile.is_zipfile(archive_path):
        with zipfile.ZipFile(archive_path) as archive:
            members = archive.infolist()
            if len(members) > MAX_ARCHIVE_MEMBERS:
                fail(f"Archive contains too many members: {len(members)}")
            for member in members:
                pure = safe_member_name(member.filename, allowed)
                key = collision_key(pure)
                if key in seen:
                    fail(f"Archive contains a duplicate or case-colliding path: {member.filename}")
                seen.add(key)
                mode = member.external_attr >> 16
                if stat.S_ISLNK(mode):
                    fail(f"Archive contains a symlink: {member.filename}")
                target = destination.joinpath(*pure.parts)
                if member.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                    continue
                if member.flag_bits & 0x1:
                    fail(f"Archive contains an encrypted member: {member.filename}")
                if member.file_size > MAX_ARCHIVE_FILE_BYTES:
                    fail(f"Archive member exceeds the size limit: {member.filename}")
                total_bytes += member.file_size
                if total_bytes > MAX_ARCHIVE_TOTAL_BYTES:
                    fail("Archive expands beyond the total size limit.")
                if member.file_size > 1024 * 1024 and member.compress_size == 0:
                    fail(f"Archive member has an invalid compression ratio: {member.filename}")
                if member.compress_size and member.file_size / member.compress_size > 1000:
                    fail(f"Archive member exceeds the compression ratio limit: {member.filename}")
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.open(member) as source, target.open("wb") as output:
                    shutil.copyfileobj(source, output)
                if mode:
                    target.chmod(stat.S_IMODE(mode))
    else:
        fail(f"Unsupported archive format: {archive_path}")


def validate_archive_command(args: argparse.Namespace) -> None:
    repo_root = Path(args.repo_root).resolve()
    archive_path = Path(args.archive).resolve()
    require_file(archive_path, "Plugin archive")
    arch = normalize_arch(args.arch)
    with tempfile.TemporaryDirectory(prefix="vibebus-archive-validate-") as temporary:
        extracted = Path(temporary)
        extract_checked_archive(archive_path, extracted, args.kind)
        if args.kind == "portable":
            plugin_root = extracted / "plugins" / "vibebus"
            marketplace = extracted / ".agents" / "plugins" / "marketplace.json"
            scope = "plugin"
            discovered_marketplace = validate_staging_root(plugin_root)
            if discovered_marketplace != marketplace:
                fail("Portable archive marketplace path is not canonical.")
        else:
            plugin_root = extracted / "vibebus"
            marketplace = None
            scope = "plugin"
        result = validate_plugin(
            repo_root,
            plugin_root,
            args.platform,
            arch,
            args.cargo,
            marketplace,
            scope,
        )
    result.update({"archive": str(archive_path), "kind": args.kind})
    print(json.dumps(result, separators=(",", ":")))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    version = subparsers.add_parser("version")
    version.add_argument("--repo-root", required=True)
    version.add_argument("--cargo", default="cargo")
    version.set_defaults(handler=lambda args: print(cargo_package(Path(args.repo_root).resolve(), args.cargo)["version"]))

    target_directory = subparsers.add_parser("target-directory")
    target_directory.add_argument("--repo-root", required=True)
    target_directory.add_argument("--cargo", default="cargo")
    target_directory.set_defaults(
        handler=lambda args: print(
            cargo_target_directory(Path(args.repo_root).resolve(), args.cargo)
        )
    )

    output_root = subparsers.add_parser("output-root")
    output_root.add_argument("--repo-root", required=True)
    output_root.add_argument("--output-root", required=True)
    output_root.set_defaults(
        handler=lambda args: print(
            ensure_output_root(Path(args.repo_root).resolve(), args.output_root)
        )
    )

    stage = subparsers.add_parser("stage")
    stage.add_argument("--repo-root", required=True)
    stage.add_argument("--output-root", required=True)
    stage.add_argument("--platform", choices=tuple(PLATFORM_COMMAND), required=True)
    stage.add_argument("--arch")
    stage.add_argument("--binary", required=True)
    stage.add_argument("--cargo", default="cargo")
    stage.set_defaults(handler=stage_plugin)

    validate = subparsers.add_parser("validate")
    validate.add_argument("--repo-root", required=True)
    validate.add_argument("--plugin-root", required=True)
    validate.add_argument("--platform", choices=tuple(PLATFORM_COMMAND), required=True)
    validate.add_argument("--arch")
    validate.add_argument("--marketplace-path")
    validate.add_argument("--scope", choices=("stage", "plugin"), default="stage")
    validate.add_argument("--cargo", default="cargo")
    validate.set_defaults(handler=validate_command)

    archive = subparsers.add_parser("archive")
    archive.add_argument("--repo-root", required=True)
    archive.add_argument("--output-root", required=True)
    archive.add_argument("--staging-root", required=True)
    archive.add_argument("--portable", required=True)
    archive.add_argument("--plugin", required=True)
    archive.add_argument("--portable-format", choices=("tar.gz", "zip"), required=True)
    archive.set_defaults(handler=archive_command)

    validate_archive = subparsers.add_parser("validate-archive")
    validate_archive.add_argument("--repo-root", required=True)
    validate_archive.add_argument("--archive", required=True)
    validate_archive.add_argument("--kind", choices=("portable", "plugin"), required=True)
    validate_archive.add_argument("--platform", choices=tuple(PLATFORM_COMMAND), required=True)
    validate_archive.add_argument("--arch")
    validate_archive.add_argument("--cargo", default="cargo")
    validate_archive.set_defaults(handler=validate_archive_command)
    return parser


def main() -> None:
    args = build_parser().parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
