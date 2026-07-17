# CLI and MCP protocol

All times are Unix milliseconds. CLI responses are JSON. MCP tools return formatted JSON as text content so the same domain models remain visible on both interfaces.

## Identity and inspection

| Capability | CLI | MCP |
| --- | --- | --- |
| Initialize project | `init` | Deliberately CLI-only |
| Register agent | `register` | `vibebus_register` |
| Recover agent | `agent recover` | `vibebus_agent_recover` |
| Rotate/provision recovery key | `agent provision-recovery` | `vibebus_recovery_provision` |
| List agents | `agents` | `vibebus_agents` |
| Project snapshot | `status` | `vibebus_status` |
| Integrity check | `doctor` | `vibebus_doctor` |
| Consistent backup | `backup` | `vibebus_backup` |

Registration returns both a bearer token and a recovery key in plaintext once. Store both outside the repository and never send either through VibeBus. Recovery accepts the current recovery key, revokes both old secrets, increments `tokenGeneration`, and returns a fresh pair. Provisioning a recovery key requires the current bearer token and revokes the previous recovery key without changing the bearer token. Mutating or private reads require `agent` and `token`.

## Messages

| Capability | CLI | MCP |
| --- | --- | --- |
| Send | `send` | `vibebus_send` |
| Inbox | `inbox` | `vibebus_inbox` |
| Mark read | `read` | `vibebus_read` |
| ACK | `ack` | `vibebus_ack` |
| Close | `close` | `vibebus_close` |

Priorities are `low`, `normal`, `high`, and `urgent`. `requiresAck` records sender intent; receipt state is still explicitly updated by the recipient.

Normal inbox reads hide closed messages. CLI callers may combine `inbox --all --include-closed`; MCP callers set `unreadOnly=false` and `includeClosed=true` to inspect receipt history. Closing is recipient-only, marks the message read, and is a retry-safe terminal action. A message with `requiresAck=true` must be ACKed before close. After close, read and ACK mutations conflict.

`send --idempotency-key <key>` / `idempotencyKey` makes an external retry return the original message. Reusing the same key with a different request is a conflict.

## Tasks

| Capability | CLI | MCP |
| --- | --- | --- |
| Create | `task create` | `vibebus_task_create` |
| Claim | `task claim` | `vibebus_task_claim` |
| Update | `task update` | `vibebus_task_update` |
| Complete | `task complete` | `vibebus_task_complete` |
| Show | `task show` | `vibebus_task_show` |
| List | `task list` | `vibebus_task_list` |

State transitions:

```text
pending --dependencies complete--> ready --claim--> claimed
claimed --> working | blocked | completed | abandoned
working --> review | blocked | completed | abandoned
review --> working | blocked | completed | abandoned
blocked --> working | abandoned
completed, abandoned --> terminal
```

Every update includes `expectedVersion`. A conflict requires a fresh read and reconciliation.

## Task/thread bindings

| Capability | CLI | MCP |
| --- | --- | --- |
| Bind active task | `thread bind` | `vibebus_thread_bind` |
| Unbind task | `thread unbind` | `vibebus_thread_unbind` |
| List active/history | `thread list` | `vibebus_thread_list` |

Only the current task owner may bind or unbind. The task must be non-terminal when binding, and only one active binding may exist per task. Repeating the same bind or unbind returns the existing record; a different active thread conflicts. `thread list` returns active bindings by default; CLI `--all` or MCP `activeOnly=false` includes history. Completing or abandoning a task automatically timestamps its active binding as unbound.

The thread ID is an opaque caller-provided identifier of 1-128 ASCII letters, digits, `-`, `_`, `.`, `:`, or `/`. VibeBus records this association but does not create, navigate, awaken, or archive a native Codex task.

## Reservations

| Capability | CLI | MCP |
| --- | --- | --- |
| Acquire | `reserve add` | `vibebus_reserve` |
| Renew owned lease | `reserve renew` | `vibebus_reservation_renew` |
| Release | `reserve release` | `vibebus_release` |
| List active | `reserve list` | `vibebus_reservations` |

TTL is 1 to 86,400 seconds. Paths use `/`-normalized project-relative syntax. A reservation expresses intent and conflict detection; it is not an operating-system file lock.

Acquire and renew accept idempotency keys. Renewal requires the authenticated owner and an active, unexpired reservation; the new expiry is calculated from renewal time.

## Artifacts

| Capability | CLI | MCP |
| --- | --- | --- |
| Publish | `artifact publish` | `vibebus_artifact_publish` |
| List | `artifact list` | `vibebus_artifact_list` |

Publishing requires an existing regular file inside the project. VibeBus stores a SHA-256 digest, type, summary, optional task ID, and arbitrary JSON metadata.
For Windows CLI calls, `--metadata-file <path>` is the stable form for complex JSON because native argument quoting can alter embedded quotes. MCP accepts the metadata object directly.

Artifact publication accepts an idempotency key. The request identity includes the current file SHA-256, so changing the file and reusing the key produces a conflict instead of returning a stale artifact.

## Events and subscriptions

| Capability | CLI | MCP |
| --- | --- | --- |
| Query events | `event list` | `vibebus_events` |
| Create subscription | `subscription create` | `vibebus_subscription_create` |
| List owned subscriptions | `subscription list` | `vibebus_subscription_list` |
| Peek replay-safe delivery | `subscription peek` | `vibebus_subscription_peek` |
| ACK replay-safe delivery | `subscription ack` | `vibebus_subscription_ack` |
| Poll and advance (legacy) | `subscription poll` | `vibebus_subscription_poll` |

Events use a project-wide monotonically increasing `sequence`. Query with `afterSequence`/`--after` and retain the last returned sequence. An empty event-type filter means all types; a non-empty filter accepts up to 32 exact event names. Message events contain routing metadata, not message subjects or bodies. After retention has advanced the history floor, an older cursor conflicts instead of silently returning a partial history; inspect `retention status` and resume from `eventsPrunedThroughSequence`.

Subscriptions belong to one authenticated agent and are unique by agent and name. Omitting `fromSequence` starts at the current project tail; `0` replays matching history only while the retained-history floor is still zero. An explicit cursor older than that floor conflicts. Subscription views expose the committed cursor, an optional pending delivery, and the most recently acknowledged delivery ID.

`peek` creates at most one pending delivery containing up to 500 matching events without advancing the committed cursor. Repeating peek returns that same delivery and full batch, even if the new request specifies a smaller limit or newer events have arrived. A delivery may contain zero matching events when it represents a scanned range of non-matching project events; it must still be acknowledged to advance over that range.

`ack` accepts the pending `deliveryId`, advances the committed cursor through the delivery range, and clears the pending state. Retrying the most recent successful ACK returns the original cursor and timestamp with `replayed=true`. A wrong or stale ID conflicts. This provides at-least-once access to the batch, not exactly-once processing; consumers must make side effects idempotent and ACK only after the complete batch succeeds.

Legacy `poll` remains available for compatibility. It returns up to 500 events and commits immediately, so a response lost after commit is not replayed. It refuses to run while a replay-safe delivery is pending and therefore cannot silently cross an unacknowledged batch.

## Bounded retention

| Capability | CLI | MCP |
| --- | --- | --- |
| Preview candidates | `retention plan` | `vibebus_retention_plan` |
| Apply confirmed plan | `retention apply` | `vibebus_retention_apply` |
| Inspect history floor | `retention status` | `vibebus_retention_status` |

Both preview and apply require an authenticated Agent. Defaults are 90 days for events, 1,000 recent events always retained, 30 days for idempotency records and closed messages, and 90 days for terminal task/thread binding history. Each age accepts 1–3,650 days; the recent tail accepts 1–1,000,000 events. `closedMessageMaxAgeDays` must be at least `idempotencyMaxAgeDays` so a cached send retry cannot reference a deleted message.

Preview is read-only and returns policy, subscription protection details, exact candidate counts, and a `planId`. Apply must repeat the same custom policy values and provide that ID. Any intervening domain event, cursor change, prior cleanup, or candidate change produces a new plan ID and makes the old confirmation conflict before deletion. Concurrent or repeated application of the same successful plan returns its stored report with `replayed=true`.

Event candidates form one contiguous prefix that is old enough, at or below the slowest subscription cursor, and outside the recent tail. A pending replay-safe delivery keeps its committed cursor unchanged, so its complete range remains protected. Apply also removes expired idempotency records, old closed receipts, resulting receipt-less messages, and old unbound history belonging to terminal tasks. It preserves active state and appends `retention_applied` as a new audit event. The operation does not run `VACUUM`.

## Structured handoff

| Capability | CLI | MCP |
| --- | --- | --- |
| Send handoff | `handoff send` | `vibebus_handoff_send` |
| Resume snapshot | `handoff snapshot` | `vibebus_handoff_snapshot` |

A handoff is a directed message with a JSON body containing `summary`, optional `taskId`, `decisions`, `artifacts`, `blockers`, and `nextActions`. VibeBus forces `high` priority and `requiresAck=true`, verifies referenced tasks and artifacts, and supports retry deduplication with an idempotency key. The recipient should read the body, act on it, and call `ack`.

The authenticated snapshot combines unread messages, non-terminal owned tasks, their active task/thread bindings, active owned reservations, the agent's recent artifacts, recent available events after a supplied sequence, the latest event sequence, and retention state. It clamps an obsolete supplied sequence to the retained-history floor so recovery remains available. It is a compact resume view, not a replacement for direct task/message reads when more than the bounded recent window is needed.

## Idempotency rules

Idempotency keys are scoped by project, authenticated agent, and operation. Valid keys are 1-128 ASCII letters, digits, `-`, `_`, `.`, or `:`. They are available on message/handoff send, reservation acquire/renew, and artifact publish. Same key plus same effective request returns the stored response; same key plus different request returns a conflict while the record remains inside the configured retention window. Task creation already has a stable caller-selected task ID, while task claim and update rely on atomic state/version checks.

## MCP root rule

When using the bundled plugin, pass the absolute repository root as `root` on every call. A direct `vibebus mcp --root <path>` launch may omit it because the process already has an explicit default root.
