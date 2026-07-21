# Changelog

## Unreleased

- Established the public open-source release contract: GitHub Releases are the official distribution channel, stable SemVer tags must resolve to commits reachable from `main`, and prerelease publishing remains disabled until it has its own verified workflow.
- Added source archives, release-evidence manifest fields, public contribution and conduct guidance, Issue/PR templates, Dependabot updates, download verification instructions, and explicit first-install, upgrade, and rollback rules.
- Added native macOS Security.framework Keychain storage for Agent and Operator credentials, including redacted registration/rotation, vault fallback, status, explicit deletion, and an isolated real-Keychain acceptance fixture.
- Added native-binary macOS SessionStart/PostToolUse/Stop Hooks with the existing path-only Git, no-log test, reliable-exit, and review-only handoff boundaries.
- Added Apple Silicon local plugin packaging, ad-hoc signing, manifest/archive/checksum validation, installed-cache acceptance, and a macOS CI job. Developer ID signing and notarization remain production gates.
- Added a CLI-only `maintenance compact --backup <new-path>` operation with exact real-terminal confirmation, vault-backed Operator authentication, fail-fast exclusive SQLite locking, and zero-active-state enforcement.
- Required a new verified backup plus conservative free-space validation before `VACUUM`, restored and checkpointed WAL afterward, and returned bounded before/after hashes, page counts, reclaimed bytes, and integrity evidence.
- Added `compaction_started` and `compaction_completed` audit events plus disposable-project coverage for success, active-state refusal, busy-database refusal, and redirected-input no-mutation behavior. No compaction tool was added to MCP.

## 0.10.0 - 2026-07-18

- Added a strict, bounded `.vibebus/responsibility.json` policy that maps Agent roles to validated project-relative paths, while preserving allow-all compatibility only when no policy is configured.
- Enforced responsibility domains for task-scoped reservations, artifact declarations, and immutable Git changed-path facts.
- Added authenticated, expiring, task-owner-issued cross-domain overrides with semantic idempotency and audit events; overrides do not replace reservation conflict control.
- Added immutable, task-scoped Git commit and test-result facts with bounded payloads, stable replay, payload-drift conflicts, optional report-artifact references, CLI/MCP parity, and context projection.
- Added a read-only bounded handoff proposal command and MCP tool.
- Added deterministic Codex `PostToolUse` capture for successful Git commits and observed test outcomes, plus a `Stop` Hook that writes a reviewable proposal without sending a handoff or reading transcripts.
- Added schema-v11 migration, policy/core/CLI/MCP coverage, seven deterministic PowerShell Hook checks, and CI Hook validation.

## 0.9.0 - 2026-07-18

- Added immutable, task-scoped confirmed decisions with stable semantic keys, task-owner authorization, artifact references, exact replay, payload-drift conflicts, idempotency, and audit events.
- Added deterministic Agent context sync across active owned tasks, direct dependencies, unread directed messages, relevant artifacts, and confirmed decisions.
- Added item and serialized-byte budgets, bounded text previews, opaque monotonic continuation cursors, and explicit scope metadata without reading artifact file contents.
- Excluded unrelated task facts plus acknowledged or closed messages from the default projection.
- Added schema-v10 migration, CLI/MCP parity, vault-backed MCP coverage, context isolation, semantic deduplication, cursor, budget, and migration tests.

## 0.8.0 - 2026-07-17

- Added a project-scoped operator credential whose secret digest is isolated from Agent bearer credentials and whose secret is stored under a distinct Windows Credential Manager target.
- Added CLI-only interactive operator initialization, rotation, vault restoration, explicit vault deletion, status, and retention-plan approval; redirected input, MCP, and normal automation cannot perform operator mutations.
- Required every new retention apply to consume one unexpired approval bound to the exact plan ID and current operator generation.
- Preserved ambiguous-result retry recovery: a completed retention run replays its stored report without requiring or consuming a second approval.
- Added schema-v9 operator credential and retention approval audit tables plus approval linkage in retention reports and runs.
- Added migration, expiration, rotation invalidation, concurrent single-consumption, vault redaction/restoration/deletion, noninteractive rejection, CLI, and MCP boundary coverage.

## 0.7.0 - 2026-07-17

- Added Windows GitHub Actions CI for formatting, locked tests, Clippy-as-error, plugin packaging, MSI validation, administrative extraction, and short-lived workflow artifacts.
- Added a pinned Rust 1.97.1 toolchain and WiX 4.0.6 local tool manifest.
- Added a per-user x64 MSI that installs the complete local marketplace under `%LOCALAPPDATA%\Programs\VibeBus` and updates the current-user PATH without custom actions.
- Added portable marketplace and standalone Codex plugin ZIPs, SHA-256 checksums, and a machine-readable release manifest.
- Added SignTool-based SHA-256 Authenticode signing and verification for the executable and MSI using ephemeral PFX material and RFC 3161 timestamps.
- Added fail-closed tag publishing: production GitHub Releases require both signing secrets, an existing matching semantic-version tag, passing release gates, and verified assets.
- Added repository-owned plugin and MSI acceptance scripts compatible with Windows PowerShell 5.1 and pwsh.

## 0.6.0 - 2026-07-17

- Added project-and-agent-scoped secret storage in Windows Credential Manager using current-user Generic Credentials.
- Added explicit credential storage and redacted delivery for registration, recovery, and recovery-key provisioning.
- Added CLI and MCP bearer-token fallback from the vault, with explicit argument and CLI environment precedence preserved.
- Added credential status and explicit delete operations without writing secrets to the repository or SQLite database.
- Added safe fallback delivery when a post-rotation vault write fails, preventing irreversible identity loss.
- Added an injectable in-memory vault plus core and MCP coverage for isolation, precedence, redaction, rotation, deletion, and failure recovery.

## 0.5.0 - 2026-07-17

- Added authenticated retention planning with explicit, state-bound confirmation IDs.
- Added retry-safe retention apply for old event prefixes, idempotency records, closed message history, orphaned messages, and terminal task/thread bindings.
- Protected unread and pending subscription ranges with the slowest committed cursor and always kept a configurable recent event tail.
- Added a persistent retained-history floor, stale event-cursor rejection, and snapshot-safe cursor clamping.
- Added cross-policy validation so closed messages outlive their cached message idempotency responses.
- Added schema-v8 retention state/run audit tables and a `retention_applied` audit event.
- Expanded CLI, MCP, migration, concurrent apply, stale-plan, cursor-gap, and pending-delivery coverage.

## 0.4.0 - 2026-07-17

- Added recipient-owned message closing with an ACK-before-close rule for acknowledgement-required messages.
- Hid closed messages from normal inbox reads while retaining explicit history access.
- Added owner-scoped, retry-safe task-to-Codex-thread bindings with active-binding uniqueness.
- Automatically unbound active task/thread associations when tasks become completed or abandoned.
- Added schema-v7 binding history and message receipt lifecycle fields.
- Expanded CLI, MCP, status, handoff snapshot, migration, concurrency, and terminal-state coverage.

## 0.3.0 - 2026-07-17

- Added replay-safe subscription peek with one persistent pending delivery per subscription.
- Added explicit, retry-safe subscription acknowledgement that advances the committed cursor.
- Preserved legacy consume-on-poll behavior while preventing it from crossing a pending delivery.
- Added schema-v6 migration fields for pending and most-recent acknowledged delivery state.
- Expanded CLI, MCP, core migration, replay, empty-filter, conflict, concurrency, and idempotent-ACK tests.

## 0.2.0 - 2026-07-17

- Added single-use agent recovery keys with bearer-token rotation and generation tracking.
- Added owner-authenticated reservation renewal with bounded TTL.
- Added scoped idempotency keys for message, reservation, renewal, artifact, and handoff retries.
- Added ordered event queries and authenticated named subscriptions with durable sequence cursors.
- Added structured high-priority handoffs, acknowledgement requirements, and resume snapshots.
- Expanded CLI, MCP, plugin Skill, migration coverage, and end-to-end acceptance tests.

## 0.1.0 - 2026-07-17

- Added project-scoped SQLite WAL coordination core.
- Added authenticated agents, directed inboxes, read and ACK receipts.
- Added dependency-aware tasks, atomic claim, state transitions, and optimistic versions.
- Added TTL path reservations and overlap conflict checks.
- Added hashed artifact publication, health diagnostics, and consistent backups.
- Added JSON CLI, official Rust MCP SDK integration, Codex plugin, Skill, Hook, and local marketplace.
- Added concurrency, security-boundary, backup, and MCP protocol tests.
