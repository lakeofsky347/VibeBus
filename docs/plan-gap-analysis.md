# VibeBus plan gap analysis

Assessment date: 2026-07-18.

This document compares the implemented VibeBus 0.9 baseline with the goals and phased roadmap in the 2026-07-17 project startup plan. The comparison uses repository code, tests, release automation, VibeBus durable state, and the completed two-real-task desktop acceptance as evidence.

## Executive assessment

The core product definition is achieved: independent Codex top-level tasks can exchange authenticated, structured, durable facts through a local SQLite-backed CLI/MCP plugin without sharing complete conversations. The real desktop run used two user-owned top-level tasks and passed the repository auditor with 178 of 178 checks, zero failures, and zero skips.

All 12 original MVP acceptance criteria are now fully covered. VibeBus 0.9 closes the former Agent-specific `sync` gap with an authenticated deterministic projection of active owned tasks, direct dependencies, unread directed messages, relevant artifacts, and immutable confirmed decisions. Item/byte budgets, bounded previews, semantic deduplication, and continuation cursors keep that projection explicit and bounded.

Phase 0, the usable Phase 1 core, and Phase 3 pluginization are complete. Phase 2 is mixed: context sync, confirmed decisions, reservations, dependency unlock, idempotency, handoffs, and lease expiry are present, while responsibility-domain enforcement and deterministic Git/test Hooks are not. Phase 4 remains deliberately deferred.

## Roadmap comparison

| Planning goal | Status | Current evidence | Remaining gap |
| --- | --- | --- | --- |
| Local structured fact bus with isolated top-level-task contexts | Complete | SQLite WAL, authenticated Agents, tasks, messages, events, artifacts, subscriptions, CLI, and MCP | None for the single-host Windows scope |
| Two independent top-level tasks sharing one service | Complete | Task B `019f73ad-0618-76a1-9c42-e17a8fda1486` and Task A `019f73af-839c-7b03-a62b-09fd7eb07ec0` completed B1/A1/B2/A2 | Do not generalize this into forced model interruption |
| Directed Inbox, read/ACK/close, and replay-safe delivery | Complete | Three structured handoffs, closed recipient receipts, same-delivery double peek, ACK replay `false` then `true` | Exactly-once consumer side effects remain out of scope |
| Tasks, dependencies, atomic claim, versions, and terminal bindings | Complete | Dependency unlock, live competing-claim conflict, optimistic-version tests, and four closed desktop bindings | Task reassignment and richer scheduling remain Phase 4 concerns |
| Artifacts, audit history, backup, and recovery | Complete | Hashed project-scoped artifacts, ordered events, schema migrations, online backups, static handoff, retained-history floor | Export/import UX and optional physical compaction can be improved |
| Agent-specific context synchronization | Complete | `context sync` has CLI/MCP parity, authenticated scope isolation, direct-dependency expansion, confirmed decisions, item/byte budgets, bounded previews, and stable continuation | Cursor pagination is deliberately not an atomic database snapshot; restart for fresh concurrent state |
| File conflict control | Partial | Exclusive overlapping reservations, renewal, expiry, release, and live conflict proof | No declarative `allowed_paths`/responsibility-domain policy or enforcement |
| Deterministic lifecycle automation | Partial | Read-only SessionStart discovery Hook and repository CI exist | No Git-commit association Hook, test-result publication Hook, or automatic terminal handoff generation |
| Codex plugin packaging and Windows delivery | Complete for unsigned acceptance | Plugin manifest, Skill, stdio MCP, Hook, portable ZIP, per-user MSI, validation, and CI | Production certificate, protected release environment, signed tag, and disposable-profile acceptance remain external gates |
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

### P1: responsibility-domain policy

- Add project-configured Agent roles and `allowed_paths` patterns.
- Validate reservations and artifact/code-change declarations against those patterns.
- Require an explicit cross-domain request or override fact rather than silently allowing drift.
- Preserve reservations as the concurrency mechanism; responsibility rules are policy, not filesystem locks.

### P1: deterministic Git and test Hooks

- Associate commit hashes with VibeBus tasks and artifacts through idempotent, bounded Hook payloads.
- Publish test results as summaries plus report paths, never raw unbounded logs.
- Generate terminal handoff proposals at safe lifecycle points while retaining recipient ACK/close control.
- Treat Hook failure as observable degradation, not permission to corrupt task state.

### P2: best-effort native-task notification bridge

- Translate selected VibeBus events into Codex task notifications only when the host exposes an authorized thread tool.
- Keep SQLite/Inboxes authoritative and make delivery retry-safe and optional.
- Never claim this bridge can interrupt a model generation already in progress.

### P2: operational maturity

- Add documented export/import and restore drills around online backups.
- Add optional, explicitly approved `VACUUM` maintenance rather than coupling it to logical retention.
- Define Agent lifecycle/offline visibility and stale-identity operational guidance.
- Decide whether the accepted disposable desktop Agent vault entries should be retained for regression or explicitly deleted.

### P3: Supervisor, visualization, and remote operation

- Add a status panel, dependency graph, reassignment, or automatic merge only after repeated local workflows justify them.
- Treat remote multi-host synchronization as a separate product architecture with its own identity and consistency model.

## Recommended next implementation slice

Implement responsibility-domain policy and deterministic Git/test Hooks before expanding orchestration. Acceptance for that slice should require:

1. Project configuration maps Agent roles to validated project-relative `allowed_paths` patterns.
2. Reservation and declared code/artifact changes outside an Agent's responsibility domain conflict unless a durable explicit override exists.
3. Cross-domain requests are authenticated, task-scoped, idempotent, and auditable.
4. Git commit association publishes bounded task facts without parsing or copying entire diffs.
5. Test-result publication stores summaries plus report artifact references, never unbounded logs.
6. Hook failure is observable and retry-safe without corrupting task, decision, message, or reservation state.
7. Context projection, authentication, migration, retained-history-floor, concurrency, and secret-redaction tests remain green.

The signed release can proceed in parallel only after the maintainer supplies the external certificate and protected environment. The optional notification bridge may now follow the authoritative context projection, but it remains lower priority than responsibility and lifecycle automation.
