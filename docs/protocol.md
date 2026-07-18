# CLI and MCP protocol

All times are Unix milliseconds. CLI responses are JSON. MCP tools return formatted JSON as text content so the same domain models remain visible on both interfaces.

## Identity and inspection

| Capability | CLI | MCP |
| --- | --- | --- |
| Initialize project | `init` | Deliberately CLI-only |
| Register agent | `register` | `vibebus_register` |
| Recover agent | `agent recover` | `vibebus_agent_recover` |
| Rotate/provision recovery key | `agent provision-recovery` | `vibebus_recovery_provision` |
| Inspect credential vault | `credential status` | `vibebus_credential_status` |
| Delete credential entry | `credential delete` | `vibebus_credential_delete` |
| Inspect operator readiness | `operator status` | Project status only |
| Initialize operator credential | `operator init` | Deliberately unavailable |
| Rotate operator credential | `operator rotate` | Deliberately unavailable |
| Restore operator vault entry | `operator restore-credential` | Deliberately unavailable |
| Delete operator vault entry | `operator delete-credential` | Deliberately unavailable |
| List agents | `agents` | `vibebus_agents` |
| Project snapshot | `status` | `vibebus_status` |
| Integrity check | `doctor` | `vibebus_doctor` |
| Consistent backup | `backup` | `vibebus_backup` |

Registration returns both a bearer token and a recovery key in plaintext once by default. CLI `--store-credentials` or MCP `storeCredentials=true` writes the pair to the current Windows user's Generic Credential and, on success, returns metadata with `secretsRedacted=true` instead of either secret. The target is `VibeBus:<project-id>:<agent>` and never enters the repository or SQLite database.

Recovery accepts an explicit recovery key or loads it from the matching vault entry, revokes both old secrets, increments `tokenGeneration`, and produces a fresh pair. Provisioning requires a current bearer token and revokes the previous recovery key without changing the bearer token. If recovery or provisioning used a vault secret, VibeBus automatically replaces the stored pair; callers can also request storage explicitly. A post-rotation vault-write failure returns the new pair with `secretsRedacted=false` and `credentialStorageError`, because hiding it would permanently strand the identity.

Mutating or private reads require an Agent identity. CLI bearer resolution is `--token`, then `VIBEBUS_AGENT_TOKEN`, then the current-user vault. MCP resolution is explicit `token`, then the vault. Token fields are therefore optional only when the correct project/Agent vault entry exists. `credential status` never returns secret material. `credential delete` removes only the OS entry; it does not remove or revoke the Agent, and later no-token calls fail until credentials are stored again. MCP deletion additionally requires `confirm=true`. Same-user processes share the Windows credential trust boundary.

The project operator is a separate procedural capability for destructive maintenance, not an Agent role. Its database row contains only a SHA-256 digest and generation; the secret is stored under `VibeBusOperator:<project-id>`, which cannot collide with `VibeBus:<project-id>:<agent>`. Operator mutation commands first require a real terminal and an exact typed confirmation. They are absent from MCP. Successful vault storage redacts the secret. If a post-initialize or post-rotate vault write fails, the interactive response returns the only usable secret plus `credentialStorageError`; after securing it, the maintainer repairs the entry with `operator restore-credential`, which reads the secret without echo. `operator status` reports DB/vault generation agreement as `ready` without exposing secret material. `operator delete-credential` requires the exact `delete:<project-id>` confirmation and removes only the OS vault entry; the database digest and generation remain configured so deletion cannot masquerade as revocation or project reset. Without another secured copy of the current secret, deletion is intentionally irreversible for that database.

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

## Confirmed decisions and Agent context

| Capability | CLI | MCP |
| --- | --- | --- |
| Confirm immutable decision | `decision confirm` | `vibebus_decision_confirm` |
| Synchronize scoped context | `context sync` | `vibebus_context_sync` |

`decision confirm` requires an authenticated owner, a non-terminal task, a project-wide semantic `key`, a 1–1,024 UTF-8 byte summary, and optional artifact IDs. Task-scoped artifacts must belong to the same task; project-level artifacts may be referenced without being copied. The first confirmation appends one `decision_confirmed` event. Repeating the exact key/payload returns the original decision even without an idempotency key, while changing the task, author, summary, or normalized artifact set conflicts. An idempotency key additionally protects an ambiguous first response.

`context sync` returns one authenticated deterministic projection containing only:

1. the Agent's active owned tasks;
2. those tasks' direct dependencies;
3. unread messages directed to the Agent;
4. confirmed decisions related to the owned/dependency task scope; and
5. artifacts related to the same scope.

Unrelated task facts and acknowledged, read, or closed messages are excluded. Artifacts expose identity, task, kind, path, SHA-256, and a bounded summary; VibeBus never reads artifact contents into context. Long task/message fields are bounded previews with explicit truncation flags.

CLI defaults are `--item-limit 100 --byte-budget 65536`; MCP uses `itemLimit` and `byteBudget`. Item limits accept 1–500 and byte budgets accept 4,096–1,048,576. `bytesUsed` is the exact sum of serialized context items, not the response envelope. A truncated page returns `hasMore=true` and an opaque `nextCursor`; pass it as `--cursor`/`cursor` to continue. Cursors encode the last deterministic fact key rather than a SQL offset, so an unchanged projection paginates without duplicates or omissions. A cursor is not an atomic database snapshot: restart from the beginning when concurrent task/message state must be reflected consistently.

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
| Approve exact plan | `operator approve-retention` | Deliberately unavailable |
| Apply confirmed plan | `retention apply` | `vibebus_retention_apply` |
| Inspect history floor | `retention status` | `vibebus_retention_status` |

Both preview and apply require an authenticated Agent. Apply additionally requires a separately authenticated operator approval. Defaults are 90 days for events, 1,000 recent events always retained, 30 days for idempotency records and closed messages, and 90 days for terminal task/thread binding history. Each age accepts 1–3,650 days; the recent tail accepts 1–1,000,000 events. `closedMessageMaxAgeDays` must be at least `idempotencyMaxAgeDays` so a cached send retry cannot reference a deleted message.

Preview is read-only and returns policy, subscription protection details, exact candidate counts, and a `planId`. A local maintainer then runs `operator approve-retention`, repeats any custom policy flags, reviews the full current plan printed to the terminal, and types the full plan ID. The approval defaults to 600 seconds, accepts 60–3,600 seconds, and is bound to the exact plan and current operator generation. It is recorded in `retention_approvals` without appending a domain event, so approval does not invalidate the plan it authorizes.

Apply must repeat the same custom policy values and provide that ID. In one `BEGIN IMMEDIATE` transaction it recomputes the plan, selects one unexpired/unconsumed approval from the current operator generation, performs the bounded deletions, consumes that approval, appends the audit event, and stores the report. Any intervening domain event, cursor change, prior cleanup, candidate change, approval expiry, or operator rotation prevents a new apply. Concurrent attempts can consume the approval only once; the winner completes normally and the loser returns the stored report with `replayed=true`. A later retry of an already successful run does not require a new approval and cannot delete twice.

Event candidates form one contiguous prefix that is old enough, at or below the slowest subscription cursor, and outside the recent tail. A pending replay-safe delivery keeps its committed cursor unchanged, so its complete range remains protected. Apply also removes expired idempotency records, old closed receipts, resulting receipt-less messages, and old unbound history belonging to terminal tasks. It preserves active state and appends `retention_applied` as a new audit event. The operation does not run `VACUUM`.

## Structured handoff

| Capability | CLI | MCP |
| --- | --- | --- |
| Send handoff | `handoff send` | `vibebus_handoff_send` |
| Resume snapshot | `handoff snapshot` | `vibebus_handoff_snapshot` |

A handoff is a directed message with a JSON body containing `summary`, optional `taskId`, `decisions`, `artifacts`, `blockers`, and `nextActions`. VibeBus forces `high` priority and `requiresAck=true`, verifies referenced tasks and artifacts, and supports retry deduplication with an idempotency key. The recipient should read the body, act on it, and call `ack`.

The authenticated snapshot combines unread messages, non-terminal owned tasks, their active task/thread bindings, active owned reservations, the agent's recent artifacts, recent available events after a supplied sequence, the latest event sequence, and retention state. It clamps an obsolete supplied sequence to the retained-history floor so recovery remains available. It is an operational resume view. Use `context sync` for the deterministic, task-scoped, budgeted projection of dependencies, decisions, and relevant artifacts.

## Idempotency rules

Idempotency keys are scoped by project, authenticated agent, and operation. Valid keys are 1-128 ASCII letters, digits, `-`, `_`, `.`, or `:`. They are available on message/handoff send, reservation acquire/renew, artifact publish, and decision confirmation. Same key plus same effective request returns the stored response; same key plus different request returns a conflict while the record remains inside the configured retention window. Confirmed decisions also retain semantic deduplication through their immutable project-wide decision key. Task creation already has a stable caller-selected task ID, while task claim and update rely on atomic state/version checks.

## MCP root rule

When using the bundled plugin, pass the absolute repository root as `root` on every call. A direct `vibebus mcp --root <path>` launch may omit it because the process already has an explicit default root.
