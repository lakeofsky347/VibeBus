# Changelog

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
