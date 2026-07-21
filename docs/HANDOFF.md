# Maintainer handoff

## Current operating model

VibeBus is a continuously maintained open-source project. GitHub Releases are the official distribution channel; stable releases are `vX.Y.Z` tags from commits reachable from `main`. Do not treat a pull request, a CI artifact, local output, or a container image as a release.

The release workflow creates a Windows x64 evidence set: source archive, signed MSI, portable ZIP, Codex plugin ZIP, SHA-256 checksums, release manifest, CycloneDX SBOM, and supply-chain evidence. macOS and Linux remain source and CI-supported until maintainers establish public platform-specific distribution contracts.

## Before changing the repository

1. Read `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `docs/release.md`, and the affected platform documentation.
2. Inspect the active branch and working tree. Do not overwrite another contributor's changes.
3. Run the smallest relevant validation first, then the full affected-platform suite before proposing a release-related merge.
4. Never commit Agent credentials, recovery keys, signing material, local databases, backups, generated plugin binaries, or `dist/` assets.

## Release and recovery rules

- Stable tags must be `vX.Y.Z`, match Cargo and plugin versions, and point to commits reachable from `main`.
- Prerelease publishing is disabled; create a separate reviewed policy before enabling it.
- GitHub Release assets must be verified as one set using `SHA256SUMS.txt`, the manifest, SBOM, and supply-chain evidence.
- A first install uses the current MSI. A subsequent upgrade is tested with `scripts/test-installer.ps1 -PreviousMsiPath <prior.msi> -ExerciseLifecycle`.
- Windows MSI downgrades are blocked by design. Preserve data, uninstall the newer version, install the earlier verified MSI, and run smoke tests to roll back.

## Coordination boundaries

VibeBus is the durable source of coordination facts. Claim tasks atomically, bind only the real native Codex task ID, inspect responsibility policy, reserve exact paths, use task-scoped expiring overrides for cross-domain paths, and process required messages with ACK before close. Use replay-safe subscription peek/ACK; at-least-once delivery requires idempotent side effects.

Operator initialization, rotation, credential deletion, retention approval, and compaction require a maintainer-controlled real terminal. They are deliberately unavailable through MCP and must not be automated by a task.
