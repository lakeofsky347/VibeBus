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
- project-scoped credential-vault isolation, explicit/environment/vault token precedence, successful secret redaction, rotation write-back, deletion, and safe write-failure fallback;
- owner-only reservation renewal and retry-safe reservation operations;
- concurrent idempotent message retries, payload-drift conflicts, and artifact content identity;
- ordered event filtering, durable subscription cursors, and repeated empty polls;
- replay-safe pending delivery, repeated peek identity, concurrent peek convergence, concurrent/idempotent ACK, empty filtered ranges, wrong-ID conflict, and legacy-poll exclusion;
- retention preview/apply confirmation, pending-delivery protection, stale-plan rejection, concurrent replay-safe apply, retained-history cursor rejection, and snapshot cursor clamping;
- age-bounded cleanup for idempotency records, closed message receipts, orphaned messages, and terminal task/thread history;
- retry-safe structured handoff, ACK lifecycle, and authenticated resume snapshot;
- CLI end-to-end subscription/handoff, message/thread lifecycle, and retention plan/apply flows;
- MCP initialize negotiation, expanded tool listing, stored registration, no-token inbox access, vault-backed recovery, credential deletion, and rejection after deletion.

The suite contains 27 tests: 4 CLI workflows, 19 core workflows, 3 credential-vault workflows, and 1 MCP protocol workflow. All pass on the accepted 0.6 checkout together with formatting and clippy-as-error checks.

## Plugin checks

- Manifest and component paths pass the plugin validator.
- `.mcp.json` launches `./bin/vibebus.exe mcp` from the plugin root.
- SessionStart is read-only and requires normal Codex hook trust review.
- The Skill states the root, `storeCredentials=true`, vault-status and failure-fallback handling, snapshot, message close lifecycle, task/thread association, retention preview/backup/confirmation discipline, replay-safe peek/ACK, legacy polling, claim, renewal, idempotency, handoff, conflict, and non-interruption boundaries.
- `vibebus@vibebus-local` is installed and enabled as version 0.6.0 in the local Codex plugin cache.
- The installed binary reports `vibebus 0.6.0` and matches the packaged SHA-256 `05285bc945e1b597d14e81ad1535d189a47f6587a3a8e1582f8417bfb2786b3b`.

## Live project migration

The existing project database was opened by the 0.4 release binary and migrated in place from schema 6 to schema version 7. `doctor` reports integrity `ok`, WAL journal mode, foreign keys enabled, and overall `ok=true`. The migration explicitly adds `message_receipts.closed_at`, creates task/thread binding history, and preserves the existing project identity and records.

A live project subscription then received a real `task_updated` event. Two peeks returned the same delivery ID without advancing the committed cursor; the first ACK advanced it, the repeated ACK returned `replayed=true`, and the listed subscription had no remaining pending delivery.

The live `MESSAGE-LIFECYCLE-001` task was moved to `working` and bound to the current native Codex task ID. The authenticated handoff snapshot returned that binding. A self-directed `requiresAck` message rejected an early close, accepted ACK then close, returned the same `closedAt` on retry, disappeared from the default inbox, and remained visible only when closed history was explicitly requested.

The 0.5 release binary then migrated the same live project from schema 7 to schema 8. Before any retention apply, SQLite online backup created `vibebus-0.5-pre-retention.db` with SHA-256 `f3035b043f44a8e11893f2f44e963ce875e0450d1ce1484296ddaaae5b1020ed`.

The default live retention preview reported latest event sequence 91, one subscription with slowest safe cursor 37, no pending delivery, retained floor 0, and zero candidates in all five cleanup domains. Applying that exact plan deleted nothing, appended one `retention_applied` audit event, and left `doctor.ok=true`; retrying the same plan returned the original timestamp with `replayed=true`. The accepted post-run backup `vibebus-0.5-final.db` has SHA-256 `4e100e59647f54428716744336fddb5728e5b165b264551fa34ed8c1a631a4c3`.

The 0.6 release binary then completed a real Windows Credential Manager acceptance against a disposable project. Stored registration returned `secretsRedacted=true` with neither secret field; an inbox read succeeded without a token; recovery omitted the recovery key, advanced the generation to 2, and remained redacted; recovery-key provisioning also used and refreshed the vault; `doctor.ok=true`; explicit deletion removed the entry; and the next no-token inbox call failed with exit code 1. The disposable OS credential and its verified workspace-local test directory were removed afterward.

The existing live coordination identity was also migrated with stored recovery-key provisioning. `vibebus_credential_status` reports backend `windows-credential-manager`, target `VibeBus:prj_51ac137e4aa342a7a80bda77d94cfbc5:git-publisher-019f6eab`, `stored=true`, and no secret was exposed during migration.

The accepted post-migration online backup `vibebus-0.6-final.db` is 290,816 bytes with SHA-256 `86a77a9e07bf4d0246223c912fe7bc54d3d97c7568ad307086749f9f7233fe2f`.

## Remaining manual acceptance

Start two new independent Codex top-level tasks in the same initialized project and verify registration/recovery-key retention, a structured handoff plus ACK, a competing task claim, a reservation conflict plus owner renewal, and subscription peek/ACK replay through the actual UI. Local plugin installation and CLI/MCP protocol acceptance are complete; creation of user-owned Codex tasks is intentionally not inferred from a general implementation request and remains the only manual UI acceptance item.
