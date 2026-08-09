#!/usr/bin/env python3
"""Negative regression suite for staged and archived VibeBus plugins."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import uuid
import zipfile
from pathlib import Path


def run_validator(
    helper: Path,
    repo_root: Path,
    plugin_root: Path,
    target_platform: str,
    arch: str,
) -> subprocess.CompletedProcess[str]:
    marketplace = plugin_root.parents[1] / ".agents" / "plugins" / "marketplace.json"
    return subprocess.run(
        [
            sys.executable,
            str(helper),
            "validate",
            "--repo-root",
            str(repo_root),
            "--plugin-root",
            str(plugin_root),
            "--marketplace-path",
            str(marketplace),
            "--platform",
            target_platform,
            "--arch",
            arch,
            "--cargo",
            "cargo",
        ],
        check=False,
        capture_output=True,
        text=True,
    )


def require_success(result: subprocess.CompletedProcess[str], label: str) -> None:
    if result.returncode != 0:
        raise SystemExit(f"positive baseline failed ({label}): {result.stdout}\n{result.stderr}")


def require_failure(result: subprocess.CompletedProcess[str], label: str, diagnostic: str) -> None:
    combined = result.stdout + result.stderr
    if result.returncode == 0:
        raise SystemExit(f"negative fixture unexpectedly passed: {label}")
    if diagnostic not in combined:
        raise SystemExit(
            f"negative fixture failed for the wrong reason ({label}); "
            f"expected {diagnostic!r}, got:\n{combined}"
        )
    print(f"negative fixture passed: {label}")


def fixture(source_package: Path, temporary: Path, label: str) -> tuple[Path, Path]:
    package = temporary / label / source_package.name
    shutil.copytree(source_package, package)
    plugin = package / "plugins" / "vibebus"
    marketplace = package / ".agents" / "plugins" / "marketplace.json"
    return plugin, marketplace


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def expect_archive_failure(
    helper: Path,
    repo_root: Path,
    archive: Path,
    kind: str,
    target_platform: str,
    arch: str,
    label: str,
    diagnostic: str,
) -> None:
    result = subprocess.run(
        [
            sys.executable,
            str(helper),
            "validate-archive",
            "--repo-root",
            str(repo_root),
            "--archive",
            str(archive),
            "--kind",
            kind,
            "--platform",
            target_platform,
            "--arch",
            arch,
            "--cargo",
            "cargo",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    require_failure(result, label, diagnostic)


def archive_negative_suite(
    helper: Path,
    repo_root: Path,
    temporary: Path,
    target_platform: str,
    arch: str,
) -> int:
    count = 0

    traversal = temporary / "archive-traversal.zip"
    with zipfile.ZipFile(traversal, "w") as archive:
        archive.writestr("../escape", "escape")
    expect_archive_failure(helper, repo_root, traversal, "plugin", target_platform, arch, "archive path traversal", "path traversal")
    count += 1

    collision = temporary / "archive-case-collision.zip"
    with zipfile.ZipFile(collision, "w") as archive:
        archive.writestr("vibebus/README.md", "one")
        archive.writestr("vibebus/README.MD", "two")
    expect_archive_failure(helper, repo_root, collision, "plugin", target_platform, arch, "archive case collision", "case-colliding")
    count += 1

    symlink = temporary / "archive-symlink.zip"
    with zipfile.ZipFile(symlink, "w") as archive:
        entry = zipfile.ZipInfo("vibebus/bin/vibebus")
        entry.create_system = 3
        entry.external_attr = (stat.S_IFLNK | 0o777) << 16
        archive.writestr(entry, "../../escape")
    expect_archive_failure(helper, repo_root, symlink, "plugin", target_platform, arch, "archive symlink", "symlink")
    count += 1

    metadata = temporary / "archive-metadata.zip"
    with zipfile.ZipFile(metadata, "w") as archive:
        archive.writestr("vibebus/._payload", "metadata")
    expect_archive_failure(helper, repo_root, metadata, "plugin", target_platform, arch, "archive AppleDouble", "AppleDouble")
    count += 1

    hardlink = temporary / "archive-hardlink.tar.gz"
    with tarfile.open(hardlink, "w:gz") as archive:
        member = tarfile.TarInfo("vibebus/link")
        member.type = tarfile.LNKTYPE
        member.linkname = "../../escape"
        archive.addfile(member)
    expect_archive_failure(helper, repo_root, hardlink, "plugin", target_platform, arch, "archive hardlink", "link or unsupported")
    count += 1

    ads = temporary / "archive-ads.zip"
    with zipfile.ZipFile(ads, "w") as archive:
        archive.writestr("vibebus/bin/vibebus.exe:evil", "alternate stream")
    expect_archive_failure(helper, repo_root, ads, "plugin", target_platform, arch, "archive NTFS ADS", "Windows-unsafe")
    count += 1
    return count


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("staged_plugin")
    parser.add_argument("--platform", choices=("macos", "linux", "windows"), required=True)
    parser.add_argument("--arch", choices=("arm64", "x64"), required=True)
    parser.add_argument("--portable-archive")
    parser.add_argument("--plugin-archive")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    helper = Path(__file__).resolve().with_name("plugin_package.py")
    staged_plugin = Path(args.staged_plugin).resolve()
    if not staged_plugin.is_dir():
        raise SystemExit(f"staged plugin does not exist: {staged_plugin}")
    source_package = staged_plugin.parents[1]
    baseline = run_validator(helper, repo_root, staged_plugin, args.platform, args.arch)
    require_success(baseline, "staged plugin")
    negative_count = 0

    with tempfile.TemporaryDirectory(prefix="vibebus-plugin-negative-") as temporary_name:
        temporary = Path(temporary_name)

        plugin, _ = fixture(source_package, temporary, "manifest-drift")
        manifest_path = plugin / ".codex-plugin" / "plugin.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["version"] = "0.0.0"
        write_json(manifest_path, manifest)
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "manifest drift", "Plugin manifest must identify")
        negative_count += 1

        plugin, _ = fixture(source_package, temporary, "wrong-mcp")
        mcp_path = plugin / ".mcp.json"
        mcp = json.loads(mcp_path.read_text(encoding="utf-8"))
        old_command = mcp["mcpServers"]["vibebus"]["command"]
        new_command = "./bin/vibebus" if args.platform == "windows" else "./bin/vibebus.exe"
        if old_command == new_command:
            raise SystemExit("wrong-MCP mutation would be a no-op")
        mcp["mcpServers"]["vibebus"]["command"] = new_command
        write_json(mcp_path, mcp)
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "wrong platform MCP entry", "MCP configuration must launch")
        negative_count += 1

        plugin, _ = fixture(source_package, temporary, "cross-platform-binary")
        native = plugin / "bin" / ("vibebus.exe" if args.platform == "windows" else "vibebus")
        foreign = plugin / "bin" / ("vibebus" if args.platform == "windows" else "vibebus.exe")
        shutil.copy2(native, foreign)
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "cross-platform binary", "allowlist mismatch")
        negative_count += 1

        for label, relative, contents in (
            ("source-tree", "src/leaked.rs", "fn leaked() {}\n"),
            ("target-cache", "target/cache/object/leaked.o", "object\n"),
            ("git-metadata", ".git/config", "[core]\n"),
            ("node-modules", "node_modules/cache/index.js", "cache\n"),
            ("python-cache", "__pycache__/leaked.pyc", "cache\n"),
        ):
            plugin, _ = fixture(source_package, temporary, label)
            residue = plugin / relative
            residue.parent.mkdir(parents=True, exist_ok=True)
            residue.write_text(contents, encoding="utf-8")
            require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), label, "disallowed top-level paths")
            negative_count += 1

        for index, name in enumerate(("Cargo.toml", "cargo.toml", "Cargo.lock", "cargo.lock", "CARGO.tomL", "cargo.TOML"), 1):
            plugin, _ = fixture(source_package, temporary, f"cargo-case-{index}")
            residue = plugin / "nested" / name
            residue.parent.mkdir(parents=True)
            residue.write_text("residue\n", encoding="utf-8")
            require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), f"forbidden filename {name}", "disallowed top-level paths")
            negative_count += 1

        for index, name in enumerate((".mcp.macos.json", ".MCP.WINDOWS.JSON", ".Mcp.Linux.Json"), 1):
            plugin, _ = fixture(source_package, temporary, f"mcp-case-{index}")
            residue = plugin / "nested" / name
            residue.parent.mkdir(parents=True)
            residue.write_text("{}\n", encoding="utf-8")
            require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), f"alternate MCP file {name}", "disallowed top-level paths")
            negative_count += 1

        plugin, marketplace = fixture(source_package, temporary, "agents-cache")
        cache = marketplace.parents[1] / "cache" / "leaked.tmp"
        cache.parent.mkdir(parents=True)
        cache.write_text("cache\n", encoding="utf-8")
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "staging agents cache", "Staging .agents")
        negative_count += 1

        plugin, marketplace = fixture(source_package, temporary, "appledouble")
        (marketplace.parents[2] / "._leaked.tmp").write_text("metadata\n", encoding="utf-8")
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "staging AppleDouble", "Staging root must contain exactly")
        negative_count += 1

        plugin, marketplace = fixture(source_package, temporary, "cross-marketplace")
        market = json.loads(marketplace.read_text(encoding="utf-8"))
        market["plugins"][0]["source"]["path"] = "./dist/staging/VibeBus-other/plugins/vibebus"
        write_json(marketplace, market)
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "cross-platform marketplace", "Staged marketplace")
        negative_count += 1

        plugin, marketplace = fixture(source_package, temporary, "wrong-marketplace-source")
        market = json.loads(marketplace.read_text(encoding="utf-8"))
        market["plugins"][0]["source"]["path"] = "./plugins/not-vibebus"
        write_json(marketplace, market)
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "wrong marketplace source", "Staged marketplace")
        negative_count += 1

        plugin, marketplace = fixture(source_package, temporary, "external-marketplace-bypass")
        external_marketplace = temporary / "valid-external-marketplace.json"
        shutil.copy2(marketplace, external_marketplace)
        market = json.loads(marketplace.read_text(encoding="utf-8"))
        market["plugins"][0]["source"]["path"] = "./plugins/tampered"
        write_json(marketplace, market)
        result = subprocess.run(
            [
                sys.executable,
                str(helper),
                "validate",
                "--repo-root",
                str(repo_root),
                "--plugin-root",
                str(plugin),
                "--marketplace-path",
                str(external_marketplace),
                "--platform",
                args.platform,
                "--arch",
                args.arch,
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        require_failure(result, "external marketplace bypass", "canonical file inside the staging root")
        negative_count += 1

        plugin, _ = fixture(source_package, temporary, "missing-hooks")
        shutil.rmtree(plugin / "hooks")
        require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "missing hooks", "allowlist mismatch")
        negative_count += 1

        if args.platform != "windows":
            plugin, _ = fixture(source_package, temporary, "wrong-permissions")
            (plugin / "bin" / "vibebus").chmod(0o700)
            require_failure(run_validator(helper, repo_root, plugin, args.platform, args.arch), "wrong executable permissions", "mode 0755")
            negative_count += 1

        if args.platform == "macos":
            package_script = repo_root / "scripts" / "package-plugin-macos.sh"
            outside = temporary / "outside"
            result = subprocess.run([str(package_script), str(outside)], check=False, capture_output=True, text=True)
            require_failure(result, "output outside repository", "Output directory")
            if outside.exists():
                raise SystemExit("outside-output rejection created the forbidden target")
            negative_count += 1

            sensitive = repo_root / "plugins" / "vibebus"
            result = subprocess.run([str(package_script), str(sensitive)], check=False, capture_output=True, text=True)
            require_failure(result, "sensitive in-repository output", "dist/ or a descendant")
            if (sensitive / "staging").exists():
                raise SystemExit("sensitive-output rejection contaminated the source plugin")
            negative_count += 1

            source_bin = repo_root / "plugins" / "vibebus" / "bin"
            if source_bin.exists() and any(source_bin.iterdir()):
                raise SystemExit("source-bin negative test requires a clean source plugin bin")
            source_bin.mkdir(exist_ok=True)
            residue = source_bin / "vibebus"
            try:
                residue.write_text("stale generated binary\n", encoding="utf-8")
                result = subprocess.run([str(package_script)], check=False, capture_output=True, text=True)
                require_failure(result, "source plugin binary residue", "Source plugin bin must be empty")
                negative_count += 1
            finally:
                if residue.exists():
                    residue.unlink()
                if source_bin.is_dir() and not any(source_bin.iterdir()):
                    source_bin.rmdir()

            symlink_path = repo_root / f".vibebus-p0-output-{uuid.uuid4().hex}"
            symlink_target = temporary / "symlink-target"
            symlink_target.mkdir()
            if symlink_path.exists() or symlink_path.is_symlink():
                raise SystemExit(f"refusing to reuse pre-existing test path: {symlink_path}")
            try:
                os.symlink(symlink_target, symlink_path)
                result = subprocess.run([str(package_script), str(symlink_path)], check=False, capture_output=True, text=True)
                require_failure(result, "symlink output", "must not traverse a symlink")
                negative_count += 1
            finally:
                if symlink_path.is_symlink():
                    symlink_path.unlink()

        if args.portable_archive:
            archive = Path(args.portable_archive).resolve()
            require_success(
                subprocess.run(
                    [sys.executable, str(helper), "validate-archive", "--repo-root", str(repo_root), "--archive", str(archive), "--kind", "portable", "--platform", args.platform, "--arch", args.arch],
                    check=False,
                    capture_output=True,
                    text=True,
                ),
                "portable archive",
            )
        if args.plugin_archive:
            archive = Path(args.plugin_archive).resolve()
            require_success(
                subprocess.run(
                    [sys.executable, str(helper), "validate-archive", "--repo-root", str(repo_root), "--archive", str(archive), "--kind", "plugin", "--platform", args.platform, "--arch", args.arch],
                    check=False,
                    capture_output=True,
                    text=True,
                ),
                "plugin archive",
            )
        negative_count += archive_negative_suite(helper, repo_root, temporary, args.platform, args.arch)

    if args.platform == "macos" and negative_count != 34:
        raise SystemExit(f"macOS regression suite must execute 34 negative cases, ran {negative_count}")
    print(json.dumps({"ok": True, "platform": args.platform, "negativeCases": negative_count}, separators=(",", ":")))


if __name__ == "__main__":
    main()
