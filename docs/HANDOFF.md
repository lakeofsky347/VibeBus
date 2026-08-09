# Maintainer handoff

## Current operating model

VibeBus is a continuously maintained open-source project. GitHub Releases are the official distribution channel; stable releases are `vX.Y.Z` tags from commits reachable from `main`. Do not treat a pull request, a CI artifact, local output, or a container image as a release.

The first `v0.10.0` release is source-only. GitHub automatically supplies the tag's source ZIP and tarball; the workflow uploads exactly `SHA256SUMS.txt`, a normalized CycloneDX SBOM, `supply-chain-evidence.json`, and `source-release-manifest.json`. It does not build or upload an MSI, portable ZIP, Codex plugin ZIP, signed executable, installer, or duplicate source ZIP. macOS, Windows, and Linux remain source and CI-supported until maintainers establish a separate binary-distribution contract.

## Before changing the repository

1. Read `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `docs/release.md`, and the affected platform documentation.
2. Inspect the active branch and working tree. Do not overwrite another contributor's changes.
3. Run the smallest relevant validation first, then the full affected-platform suite before proposing a release-related merge.
4. Never commit Agent credentials, recovery keys, signing material, local databases, backups, generated plugin binaries, or `dist/` assets.

## Release and recovery rules

- Stable tags must be `vX.Y.Z`, match Cargo and plugin versions, and point to commits reachable from `main`.
- Prerelease publishing is disabled; create a separate reviewed policy before enabling it.
- GitHub supplies the tagged source archives automatically. Verify the `source-release-manifest.json` tag and source revision, then use `SHA256SUMS.txt` to verify the SBOM, supply-chain evidence, and manifest.
- Do not rewrite a published tag or its assets. Withdraw a bad release and publish a higher patch version after the corrective change is reviewed and merged.
- This source-only release has no binary installation, upgrade, signature, or installer rollback claim. A later binary-release contract must define and validate those lifecycle rules before it can publish platform artifacts.

## P0.1 trusted plugin bootstrap

The `0.11.1` patch line treats `plugins/vibebus` as source templates, never as a package output directory. `cargo metadata --locked --no-deps --format-version 1` is the version and target-directory authority. Platform packers create an allowlisted tree only below `dist/staging/VibeBus-X.Y.Z-<platform>-<arch>`, render exactly one platform `.mcp.json`, and copy the native binary only into that staging tree.

Before accepting a rolling artifact, the platform-native validator checks the Cargo, Codex, Cursor, binary, platform and architecture contract; exact payload inventory; executable format and permissions; lifecycle Hooks; canonical staged marketplace; `initialize`, `tools/list`, and a real `vibebus_status` call against an isolated project. Generated archives are inspected for traversal, links, metadata, case collisions, Windows-unsafe names and size limits, then extracted and validated again. Windows MSI construction consumes only that validated staging root. Stable GitHub Release remains source-only and does not promote these rolling artifacts.

Use dynamic paths instead of copying a version literal:

```sh
version="$(python3 scripts/plugin_package.py version --repo-root . --cargo cargo)"
./scripts/package-plugin-macos.sh
```

```powershell
$version = (python scripts/plugin_package.py version --repo-root . --cargo cargo).Trim()
./scripts/package-plugin.ps1 -ExpectedVersion $version
```

The source marketplace cannot be installed directly because source templates intentionally contain no executable. Register the marketplace from the generated platform staging root. A macOS checkpoint proves only macOS; Windows PE/MSI and Linux ELF evidence must come from their native GitHub Actions runners before the P0 repair is accepted.

## Coordination boundaries

VibeBus is the durable source of coordination facts. Claim tasks atomically, bind only the real native Codex task ID, inspect responsibility policy, reserve exact paths, use task-scoped expiring overrides for cross-domain paths, and process required messages with ACK before close. Use replay-safe subscription peek/ACK; at-least-once delivery requires idempotent side effects.

Operator initialization, rotation, credential deletion, retention approval, and compaction require a maintainer-controlled real terminal. They are deliberately unavailable through MCP and must not be automated by a task.
