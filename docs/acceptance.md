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
- recipient-only message closing, ACK-before-close, hidden-by-default history, and retry-stable close events;
- dependency locking and automatic unlock;
- optimistic task versions and owner-only updates;
- exactly one winner under concurrent claim;
- owner-only task/thread binding, idempotent bind/unbind, terminal auto-unbind, and exactly one concurrent binding winner;
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
- CLI end-to-end subscription/handoff and message/thread lifecycle flows;
- MCP initialize negotiation, expanded tool listing, and real status tool execution.

The suite contains 22 tests: 3 CLI workflows, 18 core workflows, and 1 MCP protocol workflow. All pass on the accepted 0.4 checkout together with formatting and clippy-as-error checks.

## Plugin checks

- Manifest and component paths pass the plugin validator.
- `.mcp.json` launches `./bin/vibebus.exe mcp` from the plugin root.
- SessionStart is read-only and requires normal Codex hook trust review.
- The Skill states the root, private bearer/recovery handling, snapshot, message close lifecycle, task/thread association, replay-safe peek/ACK, legacy polling, claim, renewal, idempotency, handoff, conflict, and non-interruption boundaries.
- `vibebus@vibebus-local` is installed and enabled as version 0.4.0 in the local Codex plugin cache.
- The installed binary reports `vibebus 0.4.0` and matches the packaged SHA-256 `e0eebe3d11dd90e8458a42bccba19c171cb4884ab6d4c4f920b45d753e870940`.

## Live project migration

The existing project database was opened by the 0.4 release binary and migrated in place from schema 6 to schema version 7. `doctor` reports integrity `ok`, WAL journal mode, foreign keys enabled, and overall `ok=true`. The migration explicitly adds `message_receipts.closed_at`, creates task/thread binding history, and preserves the existing project identity and records.

A live project subscription then received a real `task_updated` event. Two peeks returned the same delivery ID without advancing the committed cursor; the first ACK advanced it, the repeated ACK returned `replayed=true`, and the listed subscription had no remaining pending delivery.

The live `MESSAGE-LIFECYCLE-001` task was moved to `working` and bound to the current native Codex task ID. The authenticated handoff snapshot returned that binding. A self-directed `requiresAck` message rejected an early close, accepted ACK then close, returned the same `closedAt` on retry, disappeared from the default inbox, and remained visible only when closed history was explicitly requested.

## Remaining manual acceptance

Start two new independent Codex top-level tasks in the same initialized project and verify registration/recovery-key retention, a structured handoff plus ACK, a competing task claim, a reservation conflict plus owner renewal, and subscription peek/ACK replay through the actual UI. Local plugin installation and CLI/MCP protocol acceptance are complete; creation of user-owned Codex tasks is intentionally not inferred from a general implementation request and remains the only manual UI acceptance item.
