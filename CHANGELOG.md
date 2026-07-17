# Changelog

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
