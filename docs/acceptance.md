# Acceptance record

Acceptance date: 2026-07-17.

## Automated checks

The repository is accepted with:

```powershell
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
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
- replay-safe pending delivery, repeated peek identity, concurrent peek convergence, concurrent/idempotent ACK, empty filtered ranges, wrong-ID conflict, and legacy-poll exclusion;
- retry-safe structured handoff, ACK lifecycle, and authenticated resume snapshot;
- CLI end-to-end subscription/handoff flow;
- MCP initialize negotiation, expanded tool listing, and real status tool execution.

The suite contains 18 tests: 2 CLI workflows, 15 core workflows, and 1 MCP protocol workflow. All pass on the accepted 0.3 checkout together with formatting and clippy-as-error checks.

## Plugin checks

- Manifest and component paths pass the plugin validator.
- `.mcp.json` launches `./bin/vibebus.exe mcp` from the plugin root.
- SessionStart is read-only and requires normal Codex hook trust review.
- The Skill states the root, private bearer/recovery handling, snapshot, replay-safe peek/ACK, legacy polling, claim, renewal, idempotency, handoff, conflict, and non-interruption boundaries.
- `vibebus@vibebus-local` is installed and enabled as version 0.3.0 in the local Codex plugin cache.
- The installed binary reports `vibebus 0.3.0` and matches the packaged SHA-256 `1be2050ddefc8c4527e69964bd2c30a2609e4935ef00dd69289d42ec6b1c609f`.

## Live project migration

The existing project database was opened by the 0.3 release binary and migrated in place to schema version 6. `doctor` reports integrity `ok`, WAL journal mode, foreign keys enabled, and overall `ok=true`.

A live project subscription then received a real `task_updated` event. Two peeks returned the same delivery ID without advancing the committed cursor; the first ACK advanced it, the repeated ACK returned `replayed=true`, and the listed subscription had no remaining pending delivery.

## Remaining manual acceptance

Start two new independent Codex top-level tasks in the same initialized project and verify registration/recovery-key retention, a structured handoff plus ACK, a competing task claim, a reservation conflict plus owner renewal, and subscription peek/ACK replay through the actual UI. Local plugin installation and CLI/MCP protocol acceptance are complete; creation of user-owned Codex tasks is intentionally not inferred from a general implementation request and remains the only manual UI acceptance item.
