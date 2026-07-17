# Acceptance record

Acceptance date: 2026-07-17.

## Automated checks

The repository is accepted with:

```powershell
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
python C:\Users\17430\.codex\skills\.system\plugin-creator\scripts\validate_plugin.py D:\MyProjects\CoWork\plugins\vibebus
```

Covered behaviors:

- directed inbox isolation and token rejection;
- read/ACK receipt state;
- dependency locking and automatic unlock;
- optimistic task versions and owner-only updates;
- exactly one winner under concurrent claim;
- overlapping reservation conflict and release;
- absolute-path rejection and task transition rules;
- artifact project scope, SHA-256, metadata, and task filtering;
- SQLite integrity, WAL, foreign keys, schema version, and online backup;
- ancestor project discovery;
- single-use recovery-key rotation, legacy-agent migration, and invalidation of old secrets;
- owner-only reservation renewal and retry-safe reservation operations;
- concurrent idempotent message retries, payload-drift conflicts, and artifact content identity;
- ordered event filtering, durable subscription cursors, and repeated empty polls;
- retry-safe structured handoff, ACK lifecycle, and authenticated resume snapshot;
- CLI end-to-end subscription/handoff flow;
- MCP initialize negotiation, expanded tool listing, and real status tool execution.

The suite contains 16 tests: 2 CLI workflows, 13 core workflows, and 1 MCP protocol workflow. All pass on the accepted 0.2 checkout together with formatting and clippy-as-error checks.

## Plugin checks

- Manifest and component paths pass the plugin validator.
- `.mcp.json` launches `./bin/vibebus.exe mcp` from the plugin root.
- SessionStart is read-only and requires normal Codex hook trust review.
- The Skill states the root, private bearer/recovery handling, snapshot, polling, claim, renewal, idempotency, handoff, conflict, and non-interruption boundaries.
- `vibebus@vibebus-local` is installed and enabled as version 0.2.0 in the local Codex plugin cache.
- The installed binary reports `vibebus 0.2.0` and matches the packaged SHA-256 `165d2c89601c80b121b71eb977b2b8ac6c0427bf6cf3c7193533b580501c1a09`.

## Live project migration

The existing project database was opened by the 0.2 release binary and migrated in place to schema version 5. `doctor` reports integrity `ok`, WAL journal mode, foreign keys enabled, and overall `ok=true`; ordered event queries return the existing audit history.

## Remaining manual acceptance

Start two new independent Codex top-level tasks in the same initialized project and verify registration/recovery-key retention, a structured handoff plus ACK, a competing task claim, a reservation conflict plus owner renewal, and subscription polling through the actual UI. Local plugin installation and CLI/MCP protocol acceptance are complete; creation of user-owned Codex tasks is intentionally not inferred from a general implementation request and remains the only manual UI acceptance item.
