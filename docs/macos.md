# macOS development and local delivery

VibeBus 0.10 has a native macOS path for CLI, stdio MCP, macOS Keychain credentials, SessionStart/PostToolUse/Stop Hooks, and Codex plugin packaging. The accepted local host is Apple Silicon (`arm64`); the packaging script also names native Intel builds as `x64`, but Intel has not yet received the same live acceptance.

## Prerequisites

Install the Xcode Command Line Tools and the repository-pinned Rust toolchain:

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.97.1 --profile minimal --component rustfmt,clippy
```

The repository `rust-toolchain.toml` selects Rust 1.97.1 automatically. A login shell must load `$HOME/.cargo/env`.

## Build and acceptance

From the repository root:

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/test-lifecycle-hooks.sh
./scripts/test-macos-keychain.sh
./scripts/package-plugin-macos.sh
./scripts/validate-plugin-macos.sh \
  ./dist/staging/VibeBus-0.10.0-macos-arm64/plugins/vibebus
(cd dist && shasum -a 256 -c SHA256SUMS-macos-arm64.txt)
```

The Keychain fixture uses an isolated project and data directory. It writes a disposable Agent credential, proves vault-backed inbox access and recovery rotation, deletes the entry, and proves later no-token rejection. Through a real pseudo-terminal it also initializes a redacted Operator, verifies database/Keychain generation agreement, rotates it, and performs the exact-confirmation deletion path. Both Keychain entries and all temporary state are removed.

`package-plugin-macos.sh` builds a native release binary, applies an ad-hoc local signature by default, stages a marketplace with a macOS-specific `.mcp.json`, validates it, and creates:

- `VibeBus-X.Y.Z-macos-arm64.tar.gz`;
- `VibeBus-Codex-plugin-X.Y.Z-macos-arm64.zip`;
- `SHA256SUMS-macos-arm64.txt`;
- `release-manifest-macos-arm64.json`.

The ad-hoc signature is for local integrity checks only. macOS treats each changed ad-hoc binary as a different program for file-based Keychain authorization. VibeBus therefore disables interactive Keychain UI in CLI/Hook operations and returns an actionable error instead of suspending a headless process when a rebuilt binary is not authorized. Finish code changes and packaging before registering the durable local Agent, or re-register under a new Agent name after a local rebuild.

For a maintainer-owned Developer ID Application certificate, set `VIBEBUS_CODESIGN_IDENTITY` to the certificate identity. The script then signs with the hardened runtime and a secure timestamp; it records `signature: "identity"` rather than claiming notarization:

```sh
VIBEBUS_CODESIGN_IDENTITY='Developer ID Application: Example Corp (TEAMID)' \
  ./scripts/package-plugin-macos.sh
```

Public distribution still requires verification that the selected identity is the intended Developer ID, notarization, stapling where applicable, and clean downloaded-artifact Gatekeeper acceptance. The repository never stores the identity's private key or notary credentials.

## Install the local plugin

Build first, then register the staged marketplace rather than the Windows-oriented source payload:

```sh
codex plugin marketplace add \
  "$PWD/dist/staging/VibeBus-0.10.0-macos-arm64"
codex plugin add vibebus@vibebus-local
```

Start a new Codex task after installation. Review and trust the Hook definitions. The installed `.mcp.json` launches `./bin/vibebus mcp`; macOS Hook commands launch the same binary with the hidden `hook` subcommands.

## Initialize local runtime state

Do not copy the Windows SQLite runtime or export Windows credentials. Keep the tracked `.vibebus/project.json`, then create fresh Mac-local state:

```sh
./plugins/vibebus/bin/vibebus --root /path/to/project \
  register --name mac-worker --role implementation --store-credentials
./plugins/vibebus/bin/vibebus --root /path/to/project \
  credential status --agent mac-worker
./plugins/vibebus/bin/vibebus --root /path/to/project \
  inbox --agent mac-worker
```

Successful storage reports backend `macos-keychain`, `stored=true`, and `secretsRedacted=true`. Agent entries use service `VibeBus:<project-id>:<agent>` with account `VibeBus`; Operator entries use the distinct service `VibeBusOperator:<project-id>`. SQLite stores only credential digests.

The default database is outside the repository under:

```text
~/Library/Application Support/dev.VibeBus.VibeBus/projects/<project-id>/vibebus.db
```

Use `VIBEBUS_DATA_HOME` or `--data-home` for isolated testing. `credential delete` removes only the Keychain entry; it does not revoke or delete the Agent database record. Operator initialization and destructive maintenance remain explicit real-terminal actions and are never performed by installation scripts or MCP.

## Platform boundaries

- macOS and Windows have native current-user vault backends; Linux/container builds still require explicit or environment-injected Agent tokens.
- macOS Hooks use the native Rust binary and do not require PowerShell, Node, Python, jq, transcript access, diffs, or raw test logs at runtime. Python is used only by repository validation fixtures.
- The current package is native to the host architecture, not a Universal 2 binary.
- Runtime state is local to one host. VibeBus does not synchronize databases or Keychain items across machines.
