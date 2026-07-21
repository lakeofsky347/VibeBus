# VibeBus

![VibeBus logo](plugins/vibebus/assets/vibebus-logo-light.png)

VibeBus is a local, structured fact bus for independent Codex tasks. Tasks keep their own conversations and worktrees while sharing only explicit messages, acknowledgements, task state, decisions, path reservations, artifacts, and bounded test or Git facts through a project-scoped SQLite database.

VibeBus provides a Rust CLI, stdio MCP server, Codex plugin, and lifecycle Hooks. It runs natively on Windows and macOS; Linux is supported through the `amd64` container workflow.

## Install and run from source

Windows:

```powershell
./scripts/package-plugin.ps1
codex plugin marketplace add .
codex plugin add vibebus@vibebus-local
```

macOS:

```sh
./scripts/package-plugin-macos.sh
codex plugin marketplace add "$PWD/dist/staging/VibeBus-0.10.0-macos-arm64"
codex plugin add vibebus@vibebus-local
```

After installing or changing Hooks, start a new Codex task and review the Hook changes. Initialize a project deliberately, then register an Agent with an explicit responsibility role:

```sh
vibebus init --root /path/to/project --name "My Project"
vibebus register --root /path/to/project --name implementation --role implementation --store-credentials
vibebus credential status --root /path/to/project --agent implementation
```

Before editing, claim a ready task, inspect the responsibility policy, reserve only the exact project-relative paths, and obtain a task-scoped expiring override for paths outside the role. Process required messages as read → ACK → close and use replay-safe subscription peek/ACK for event delivery.

## Releases

GitHub Releases are the only official distribution channel. A stable release is a `vX.Y.Z` tag whose commit is reachable from `main`; ordinary branches, pull-request artifacts, and local builds are not releases. SemVer is used for release versions:

- patch releases fix compatible behavior;
- minor releases add backwards-compatible capabilities;
- major releases may introduce incompatible changes and include migration guidance.

The stable workflow publishes a source archive plus Windows x64 signed MSI, portable ZIP, Codex plugin ZIP, `SHA256SUMS.txt`, machine-readable manifest, CycloneDX SBOM, and supply-chain evidence. macOS and Linux remain source and CI-supported paths until their own public distribution contracts are implemented. Release candidates are not currently published: tags such as `vX.Y.Z-rc.N` fail closed rather than being treated as stable releases.

To verify a downloaded stable release, download the complete evidence set from the same GitHub Release, verify `SHA256SUMS.txt`, verify Authenticode on the Windows executable and MSI, then test a first install. For an upgrade, use a prior MSI with `scripts/test-installer.ps1 -PreviousMsiPath <prior.msi> -ExerciseLifecycle`. Windows MSI downgrades are intentionally blocked: uninstall the newer version before installing an earlier MSI, and preserve any project data separately before that rollback.

Detailed maintainer and verifier instructions are in [docs/release.md](docs/release.md).

## Development

The repository pins Rust `1.97.1`. Run the checks that match the files you change:

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python3 scripts/normalize-cyclonedx.py --self-test
git diff --check
```

Windows packaging and MSI validation require Windows tooling:

```powershell
./scripts/build-release.ps1
$msi = Get-ChildItem ./dist/VibeBus-*-windows-x64.msi | Select-Object -First 1
./scripts/test-installer.ps1 -MsiPath $msi.FullName
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution expectations, [SECURITY.md](SECURITY.md) for private vulnerability reporting, and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community standards.

## Security and operating boundaries

- Agent credentials belong in Windows Credential Manager or macOS Keychain; never commit tokens, recovery keys, signing material, databases, or generated release assets.
- The Operator lifecycle and destructive maintenance require a real interactive terminal; those actions are intentionally unavailable through MCP.
- VibeBus records durable facts but does not wake, interrupt, merge, or otherwise control native Codex tasks.
- Replay-safe subscriptions are at-least-once deliveries. Consumers must make side effects idempotent.

## Documentation

- [Architecture](docs/architecture.md)
- [CLI and MCP protocol](docs/protocol.md)
- [Release engineering](docs/release.md)
- [macOS development](docs/macos.md)
- [Container development](docs/container.md)
- [Maintainer handoff](docs/HANDOFF.md)

VibeBus is released under the [MIT License](LICENSE).
