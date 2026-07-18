# VibeBus plan gap analysis

Assessment date: 2026-07-18.

This document compares the implemented VibeBus 0.10 baseline with the goals and phased roadmap in the 2026-07-17 project startup plan. The comparison uses repository code, tests, release automation, VibeBus durable state, and the completed two-real-task desktop acceptance as evidence.

## Executive assessment

The core product definition is achieved: independent Codex top-level tasks can exchange authenticated, structured, durable facts through a local SQLite-backed CLI/MCP plugin without sharing complete conversations. The real desktop run used two user-owned top-level tasks and passed the repository auditor with 178 of 178 checks, zero failures, and zero skips.

All 12 original MVP acceptance criteria are fully covered. VibeBus 0.10 adds strict responsibility domains, authenticated expiring task-scoped overrides, immutable Git/test facts, and deterministic lifecycle Hooks to the 0.9 context projection. Item/byte budgets, bounded previews, semantic deduplication, and continuation cursors keep the shared surface explicit and bounded.

Phase 0, the usable Phase 1 core, and Phase 3 pluginization are complete. The branded plugin package is integrated into cumulative PR #12, and the repository is publicly visible with secret scanning, push protection, private vulnerability reporting, and a root security policy. The planned local Phase 2 coordination controls are now substantially complete: context sync, confirmed decisions, responsibility enforcement, reservations, dependency unlock, idempotency, handoffs, lease expiry, and bounded Git/test lifecycle automation are present. Phase 4 remains deliberately deferred.

## Roadmap comparison

| Planning goal | Status | Current evidence | Remaining gap |
| --- | --- | --- | --- |
| Local structured fact bus with isolated top-level-task contexts | Complete | SQLite WAL, authenticated Agents, tasks, messages, events, artifacts, subscriptions, CLI, and MCP | None for the single-host Windows scope |
| Two independent top-level tasks sharing one service | Complete | Task B `019f73ad-0618-76a1-9c42-e17a8fda1486` and Task A `019f73af-839c-7b03-a62b-09fd7eb07ec0` completed B1/A1/B2/A2 | Do not generalize this into forced model interruption |
| Directed Inbox, read/ACK/close, and replay-safe delivery | Complete | Three structured handoffs, closed recipient receipts, same-delivery double peek, ACK replay `false` then `true` | Exactly-once consumer side effects remain out of scope |
| Tasks, dependencies, atomic claim, versions, and terminal bindings | Complete | Dependency unlock, live competing-claim conflict, optimistic-version tests, and four closed desktop bindings | Task reassignment and richer scheduling remain Phase 4 concerns |
| Artifacts, audit history, backup, and recovery | Complete | Hashed artifacts, ordered events, migrations, online backups, retained-history floor, CI-backed isolated restore/import, and backup-first CLI-only offline compaction | Live compaction remains a maintainer-controlled maintenance-window decision, not a release requirement |
| Agent-specific context synchronization | Complete | `context sync` has CLI/MCP parity, authenticated scope isolation, direct-dependency expansion, confirmed decisions, item/byte budgets, bounded previews, and stable continuation | Cursor pagination is deliberately not an atomic database snapshot; restart for fresh concurrent state |
| File conflict control | Complete for declared operations | Strict role `allowedPaths`, task-scoped expiring overrides, reservation/artifact/Git-path enforcement, plus existing overlap/TTL control | This is application policy, not an OS filesystem sandbox; raw external writes remain outside the bus |
| Deterministic lifecycle automation | Complete for bounded local facts | PostToolUse records commit identity/path lists and test outcomes; Stop writes a review-only proposal; Hook fixtures run in CI | Specialized host tools may opt out of Hooks; automatic handoff sending is deliberately excluded |
| Codex plugin packaging and Windows delivery | Complete for unsigned acceptance | Public repository, branded plugin manifest/assets, marketplace metadata, Skill, stdio MCP, Hook, portable ZIP, per-user MSI, validation, and green cumulative PR #12 CI | Production certificate, protected release environment, signed tag, and disposable-profile acceptance remain external gates |
| Notifications, Supervisor, and visualization | Deferred | Codex task tools can create, read, wait, and continue user-authorized tasks; SQLite remained authoritative during acceptance | No plugin-owned best-effort notification bridge, status UI, dependency graph, or Worker supervisor |
| Remote/multi-host operation | Deferred by design | Project state is local and project-ID scoped | No remote synchronization, cross-device vault recovery, or distributed consistency model |

## Original MVP acceptance criteria

| # | Criterion | Result |
| ---: | --- | --- |
| 1 | Two independent top-level tasks register distinct identities | Complete in the real desktop run |
| 2 | A directed message is invisible to an unrelated Agent | Complete in authenticated inbox-isolation tests |
| 3 | Agent sync returns only its task, dependencies, unread messages, and decisions | Complete through schema-v10 confirmed decisions and budgeted authenticated context projection |
| 4 | Required messages can be ACKed and stop appearing as unread | Complete in the real desktop run |
| 5 | Only one Agent can own a competing task claim | Complete in concurrency tests and the real B conflict |
| 6 | Stale versions cannot overwrite newer task state | Complete in core tests |
| 7 | Completing a dependency unlocks its direct dependants | Complete in tests and the desktop fixture |
| 8 | Large outputs travel by summary and artifact path | Complete through hashed artifact publication and handoff contracts |
| 9 | Concurrent database access preserves integrity and events | Complete through WAL/concurrency tests and `doctor.ok=true` |
| 10 | Static handoff remains available when live coordination is unavailable | Complete through `docs/HANDOFF.md` and durable handoff artifacts |
| 11 | Tasks collaborate without copying complete conversations | Complete by protocol and real-run prompts |
| 12 | Writes are project-scoped, authenticated, transactional, and audited | Complete across CLI/MCP validation and event-backed mutations |

## Prioritized gaps and optimization items

### P0: production release acceptance

- Configure a protected GitHub `release` environment and real Windows code-signing certificate.
- Create a matching signed tag only after all gates pass.
- Verify downloaded checksums, Authenticode timestamps, and install/uninstall from a disposable Windows user profile.

This is the only remaining release blocker, but it requires maintainer-owned external credentials and policy.

### Completed in 0.9: context sync and confirmed decisions

- CLI and MCP call the same first-class Agent context projection.
- The scope contains only active owned tasks, direct dependencies, unread directed messages, relevant artifacts, and immutable confirmed decisions.
- Configurable serialized-byte/item budgets, deterministic ordering, and opaque monotonic continuation are enforced.
- Confirmed decision keys provide durable semantic deduplication; acknowledged/read/closed messages leave the default projection.
- Long text is a bounded preview and long evidence remains behind artifact references.

This closes the former sole partial MVP criterion and makes responsibility-domain enforcement the next highest-value product slice.

### Completed in 0.10: responsibility-domain policy

- `.vibebus/responsibility.json` maps Agent roles to strict bounded project-relative patterns; absent means legacy-compatible allow-all, present-invalid fails closed.
- Task-scoped reservations, artifact declarations, and Git changed paths are validated against the effective role policy.
- Cross-domain overrides are owner-authenticated, task-scoped, expiring, idempotent, and audited.
- Reservations remain the concurrency mechanism; responsibility rules authorize intent but are not filesystem locks.

### Completed in 0.10: deterministic Git and test Hooks

- Git facts bind task, commit SHA, summary, and at most 200 authorized changed paths; no diff content is parsed or stored.
- Test facts store bounded suite/outcome/command/summary plus an optional report artifact; raw logs are excluded.
- PostToolUse records only when a real active binding and reliable exit status exist. Stop writes a bounded proposal and never sends automatically.
- Hook failure surfaces a `systemMessage`, exits safely, and cannot alter completed tool side effects.

### P2: best-effort native-task notification bridge

- Translate selected VibeBus events into Codex task notifications only when the host exposes an authorized thread tool.
- Keep SQLite/Inboxes authoritative and make delivery retry-safe and optional.
- Never claim this bridge can interrupt a model generation already in progress.

### Completed in 0.10: backup restore drill

- The repository-owned PowerShell drill creates a source recovery point, introduces post-backup drift, imports the marker and database into an isolated data home, and proves point-in-time exclusion.
- Each run verifies the returned SHA-256, schema, WAL, foreign keys, restored Agent authentication, expected task set, secret redaction, and complete temporary-data cleanup.
- Windows CI runs the drill against the release binary, while the production runbook keeps real cutover offline and maintainer-controlled.

### Completed: explicit offline compaction

- `maintenance compact --backup <new-path>` is CLI-only and absent from MCP.
- A real terminal exact confirmation and the current vault-backed Operator secret are required before the maintenance connection opens.
- Current schema, zero active tasks/bindings/reservations, a fail-fast exclusive boundary, a new verified backup, and conservative free-space validation are mandatory before `VACUUM`.
- WAL restoration/checkpoint, integrity/foreign-key/schema verification, bounded before/after evidence, and start/completion audit events are covered by disposable-project tests. The live coordination database was not compacted.

### P2: remaining operational maturity

- Define Agent lifecycle/offline visibility and stale-identity operational guidance.
- Decide whether the accepted disposable desktop Agent vault entries should be retained for regression or explicitly deleted.

### P3: Supervisor, visualization, and remote operation

- Add a status panel, dependency graph, reassignment, or automatic merge only after repeated local workflows justify them.
- Treat remote multi-host synchronization as a separate product architecture with its own identity and consistency model.

## Recommended next implementation slice

The next repository-owned slice should continue operational maturity without expanding into remote orchestration:

1. Add stale-Agent/offline visibility and a safe identity lifecycle runbook without automatic credential deletion.
2. Consider an optional notification bridge only as best-effort UI delivery over the authoritative SQLite Inbox.
3. Preserve context scope, responsibility authorization, Hook no-log/no-auto-send boundaries, migration, retained-history floor, concurrency, and secret redaction.

The signed release remains the highest-priority external gate and can proceed only after the maintainer supplies the certificate and protected environment. Remote synchronization, automatic merging, Supervisor scheduling, and forced model interruption remain separate product decisions rather than 0.10 gaps.
