# VibeBus architecture

## Goal

VibeBus coordinates independent Codex top-level tasks without collapsing their chat or worktree isolation. The shared layer contains durable facts only: recoverable agent identities, directed messages, receipts, tasks, task/thread associations, dependencies, reservations, artifacts, subscriptions, and audit events.

## Runtime shape

```mermaid
flowchart LR
    A["Codex task A"] -->|MCP or CLI| X["vibebus.exe"]
    B["Codex task B"] -->|MCP or CLI| X
    C["Codex task C"] -->|MCP or CLI| X
    X --> D["Project SQLite database in WAL mode"]
    X --> V["Windows Credential Manager<br/>current-user Generic Credentials"]
    R["Repository .vibebus/project.json"] --> X
```

There is no resident HTTP daemon. Each CLI call or MCP process opens the same project database. SQLite `BEGIN IMMEDIATE`, unique constraints, optimistic versions, and a busy timeout provide the coordination boundary.

## Project identity and storage

The repository marker is intentionally small and commit-friendly:

```json
{
  "projectId": "prj_...",
  "name": "Example",
  "createdAt": "...",
  "schemaVersion": 1
}
```

The database is outside the repository by default:

```text
%LOCALAPPDATA%\VibeBus\projects\<project-id>\vibebus.db
```

`VIBEBUS_DATA_HOME` or `--data-home` can override the base directory for testing and controlled deployments.

Bearer and recovery secrets remain outside this database. When storage is explicitly requested, Windows Credential Manager receives one current-user Generic Credential per project/Agent target:

```text
VibeBus:<project-id>:<agent-name>
```

Destructive maintenance uses a distinct project operator target:

```text
VibeBusOperator:<project-id>
```

The operator is intentionally not an Agent role. Its CLI-only interactive capability approves a single exact retention plan for a short window; MCP can plan and apply but cannot initialize, rotate, restore, delete, or approve the operator credential.

The serialized secret pair includes a format version and token generation and stays below the Windows 2,560-byte Generic Credential BLOB limit. `CRED_PERSIST_LOCAL_MACHINE` makes it available to later local logon sessions of the same Windows user, not to other users or machines.

## Data model

| Table | Purpose |
| --- | --- |
| `projects` | Resolved project identity and root |
| `agents` | Names, roles, bearer/recovery hashes, token generation, liveness timestamps |
| `messages` | Directed structured facts |
| `message_receipts` | Per-recipient delivered/read/ACK/closed state |
| `tasks` | Owner, state, version, blocker, timestamps |
| `task_dependencies` | Prerequisite edges |
| `task_thread_bindings` | Historical task-to-Codex-thread associations and unbind time |
| `reservations` | TTL-backed path intent |
| `artifacts` | File path, hash, summary, task relation, metadata |
| `idempotency_records` | Operation-scoped request hashes and cached responses |
| `subscriptions` | Agent-owned filters, committed cursors, pending deliveries, and last-ACK state |
| `events` | Append-only audit facts |
| `retention_state` | Retained event-history floor and latest applied plan |
| `retention_runs` | Retry-safe immutable cleanup reports keyed by confirmation plan |
| `operator_credentials` | Project operator secret digest, generation, and rotation timestamps |
| `retention_approvals` | Short-lived plan approvals, generation binding, and atomic consumption audit |
| `schema_migrations` | Applied database schema versions |

## Invariants

1. A message appears only in explicitly named recipient inboxes.
2. Exactly one concurrent claimant can own a claimable task.
3. A task update requires both the owner identity and current version.
4. Dependency completion changes eligible pending tasks to ready.
5. Terminal tasks cannot return to active states.
6. Exclusive overlapping reservations owned by different agents conflict.
7. Reservation paths cannot escape the project syntactically.
8. Artifact paths must resolve to existing files inside the canonical project root.
9. Backup creation never overwrites an existing destination.
10. Successful agent recovery rotates both the bearer token and single-use recovery key.
11. An idempotency key either returns its original response or conflicts on payload drift.
12. A subscription has at most one pending replay-safe delivery; its committed cursor moves only after matching ACK.
13. Structured handoffs are directed, high-priority, and marked as requiring acknowledgement.
14. Legacy consume-on-poll cannot advance through an unacknowledged replay-safe delivery.
15. A message requiring acknowledgement cannot be closed before the recipient ACKs it.
16. Closed messages are immutable receipt history and are hidden from the normal inbox.
17. A task has at most one active Codex-thread binding, managed only by its task owner.
18. Completed or abandoned tasks have no active thread binding; terminal transition ends it atomically.
19. Event retention deletes only a contiguous prefix older than policy, below the slowest subscription cursor, and outside the configured recent tail.
20. A pending subscription delivery is protected because its committed cursor cannot advance until matching ACK.
21. Retention apply requires an unchanged preview plan; the same confirmed plan can be retried without deleting twice.
22. Closed messages cannot expire before message idempotency responses that may still reference them.
23. A credential target is scoped by both project ID and validated Agent name, so identities with the same name in different projects do not collide.
24. A successful stored-secret delivery omits bearer and recovery values; a failed post-rotation write returns the only usable pair with an explicit storage error instead of stranding the identity.
25. Explicit token input wins over environment input, which wins over current-user vault fallback; recovery-key fallback comes only from the matching vault entry.
26. Operator mutation is CLI-only, requires a real terminal plus exact typed confirmation, and has no MCP tool.
27. A new retention run requires one unexpired, unconsumed approval for the exact plan and current operator generation.
28. Approval consumption, retention deletion, audit append, and immutable report storage commit atomically; a completed retry needs no second approval.

## Recovery and retry boundaries

Bearer tokens and recovery keys are generated from independent random UUID material and stored in SQLite only as SHA-256 digests. Recovery is a credential rotation, not an identity recreation: task ownership, messages, subscriptions, reservations, and artifacts remain attached to the same agent row. A legacy agent can provision its first recovery key using its still-valid bearer token.

The credential-vault layer is outside the domain transaction. Register, recover, and provision first commit the authoritative digest state, then write the returned pair to the OS vault. On success, the response is redacted. If that second write fails, suppressing the secrets would make the newly committed identity unrecoverable, so the response deliberately falls back to the plaintext pair plus `credentialStorageError` and `secretsRedacted=false`. When recovery or provisioning itself used a vault secret, the rotated pair is automatically written back so the single-use recovery key cannot become stale.

The Windows backend calls `CredWriteW`, `CredReadW`, `CredDeleteW`, and `CredFree` directly. Tests inject an in-memory implementation and never touch a developer's real credentials. The vault is an at-rest and accidental-disclosure boundary, not a sandbox between processes already running as the same Windows user.

Operator initialization and rotation use the same digest-first/vault-second recovery rule as Agent credentials. Successful storage redacts the operator secret. If the vault write fails after the authoritative generation changes, the interactive response exposes the only usable secret once with an explicit error. `operator restore-credential` accepts that secret through a no-echo TTY prompt, verifies it against the database digest, and repairs the separate vault entry. An operator rotation invalidates every outstanding approval from the previous generation without deleting its audit record. Explicit `operator delete-credential` removes only the current-user vault entry after a real-terminal `delete:<project-id>` confirmation. It deliberately preserves the database digest and generation, causing `ready=false`; this supports verifiable disposable-project cleanup without adding a remote or MCP deletion capability.

Externally retried writes record a canonical JSON request hash and serialized response in the same `BEGIN IMMEDIATE` transaction as the domain mutation. This prevents the common ambiguous-result retry from creating duplicate messages, leases, renewals, or artifacts. Records are deliberately scoped by operation so unrelated APIs may reuse a caller's key. After an explicitly configured idempotency retention window expires, that historical retry guarantee also expires; the cleanup plan reports how many records are affected.

## Event consumption

Every domain mutation appends a project event in the same transaction. The integer `sequence` is the canonical ordering cursor. Direct event queries are stateless and replayable from a caller-retained cursor.

Named subscriptions keep an authenticated agent-owned committed cursor in SQLite. Replay-safe peek writes one pending delivery range without moving that cursor; repeated peeks reconstruct the same immutable event range. ACK atomically moves the cursor, records the most recent acknowledged delivery for retry recovery, and clears pending state. Cursor bookkeeping deliberately does not append another event, avoiding an infinite self-notification loop for wildcard subscriptions.

Legacy polling still consumes and commits in one call, but it conflicts whenever pending delivery state exists. Future retention must never delete events inside any pending delivery range. Critical payloads such as handoffs remain independently durable in the directed inbox, so event delivery is notification/indexing state rather than the sole copy of work instructions.

## Message and task/thread lifecycles

Message closing is a recipient-owned terminal receipt action. Closing also records read state. Messages marked `requiresAck` must have an ACK first; a successful close is retry-safe and appends exactly one `message_closed` event. The default inbox excludes closed messages, while explicit history reads can include them.

A task owner may bind a non-terminal task to one caller-supplied Codex task/thread identifier. Repeating the same bind or unbind returns the original binding state; attempting a second active identifier conflicts. VibeBus stores only the association—it does not invoke Codex thread APIs. Moving a task to `completed` or `abandoned` atomically closes its active binding, so resume snapshots cannot advertise stale execution ownership.

## Bounded retention

Retention is an explicit three-gate operation. An authenticated Agent preview calculates cutoffs and exact candidate counts, then hashes the policy, current event tail, subscription protection boundary, retained-history floor, and candidates into a `planId`. A local operator reviews and interactively authorizes that exact ID for 60–3,600 seconds. Apply opens `BEGIN IMMEDIATE`, recomputes the plan, requires an active approval at the current operator generation, and refuses stale or unapproved work before deletion. Approval bookkeeping deliberately does not append a domain event because that would invalidate its own plan.

The accepted approval is consumed in the same transaction as deletion, the `retention_applied` event, and the immutable run report. Concurrent attempts serialize; after one commits, another sees and replays the stored report. This ambiguous-result recovery path does not require a second approval and cannot execute the deletion twice.

Events are pruned only as a contiguous prefix. The planned boundary is the minimum of the age boundary, the slowest committed subscription cursor, and the sequence immediately before the configured recent-event tail. A subscription with cursor `0` therefore blocks event deletion; an unacknowledged pending delivery keeps the same protection until ACK. Each successful cleanup appends a new `retention_applied` audit event after deletion and advances a persistent history floor. Event queries or deliberate subscription replays older than that floor conflict instead of silently returning incomplete history; compact handoff snapshots clamp to the available floor.

Other cleanup domains are independent and age-bounded: idempotency records, closed per-recipient receipts, messages left with no receipts, and unbound history for terminal tasks. Active messages, active task bindings, non-terminal task history, subscriptions, tasks, artifacts, agents, reservations, retention reports, and schema records are not removed. Physical `VACUUM` is deliberately not automatic because it requires a more disruptive exclusive database operation; online backups remain the recovery boundary.

## Codex plugin lifecycle

The plugin contains:

- `.mcp.json`, which launches the packaged native executable with `mcp`;
- `skills/vibebus-coordination/SKILL.md`, which defines the coordination discipline;
- `hooks/hooks.json`, which runs a read-only Windows SessionStart discovery script.

The MCP process starts from the installed plugin directory. For that reason every MCP tool accepts a `root` argument; the Skill requires an absolute project root on every call.

## Release supply chain

Rust 1.97.1 and Cargo.lock define the native build inputs. A repository-owned PowerShell path packages the release binary into the plugin, validates its manifest/MCP/Hook/Skill contract, stages one marketplace-shaped payload, and uses pinned WiX 4.0.6 to produce a per-user x64 MSI. The same staged payload feeds the portable and plugin archives, so the three distribution formats share one binary and plugin tree.

Pull-request CI has read-only repository permission and produces explicitly unsigned, short-lived acceptance artifacts. It runs stock MSI validation except the irrelevant per-machine-only ICE91 rule, creates an administrative image, verifies the expected payload and binary version, and cleans the image.

Production release automation is tag-gated and fail-closed. An existing `vX.Y.Z` tag must match Cargo and plugin versions. SignTool signs and verifies the binary before staging and the MSI after construction using SHA-256 and an RFC 3161 timestamp. Checksums are calculated only after signing. A job-scoped `GITHUB_TOKEN` with `contents: write` publishes assets; signing material remains only in repository or protected-environment Secrets.

## Deliberate non-goals for 0.8

- sharing entire chat transcripts;
- injecting messages into an already-running model generation;
- replacing Codex tasks, worktrees, or native subagents;
- remote multi-host synchronization;
- automatic Git merging or conflict resolution;
- holding secrets in repository files;
- exactly-once consumer side effects; replay-safe delivery is at-least-once until ACK;
- non-Windows secure-vault backends or cross-device credential recovery;
- protection against a malicious process already running as the same Windows user;
- hardware-backed or remote multi-party operator approval; the current gate is a local human-presence and accidental-automation boundary;
- automatic creation of release tags or unsigned production releases;
- installer custom actions that silently mutate Codex plugin configuration;
- bit-for-bit reproducible MSI/ZIP output or private-repository attestations on an ineligible GitHub plan.
