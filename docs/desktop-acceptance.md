# Two-real-task desktop acceptance

This runbook verifies VibeBus through two independent, user-owned Codex top-level tasks in the same initialized project. It is intentionally not satisfied by unit tests, CLI subprocesses owned by one task, Codex subagents, or two identities driven from one conversation.

The acceptance shares only structured VibeBus facts. Neither task may copy a chat transcript, hidden reasoning, bearer token, recovery key, credential BLOB, operator secret, or signing secret into a message, file, task, event, or terminal transcript.

## Scope and fixed fixture

Use the prepared fixture exactly once:

| Item | Value |
| --- | --- |
| Project root | `D:\MyProjects\CoWork` |
| Run ID | `desktop-20260717-01` |
| Task A Agent | `desktop-a-20260717-01` |
| Task B Agent | `desktop-b-20260717-01` |
| B subscription | `desktop-20260717-01` |
| Shared reservation path | `acceptance/desktop-20260717-01/shared-resource` |
| Claim task | `DESKTOP-CLAIM-001` |
| B readiness task | `DESKTOP-B-READY-001` |
| B result task | `DESKTOP-B-RESULT-001` |
| A finalize task | `DESKTOP-A-FINALIZE-001` |
| Evidence recipient | `git-publisher-019f6eab` |

The four VibeBus tasks must already exist and be `ready` or dependency-gated as documented below. If an Agent name, subscription name, or task ID already exists in an unexpected state, stop and investigate; do not register a suffix variant or silently reset durable state.

## Preconditions

1. The repository is clean and the branch/remote commit match.
2. `vibebus doctor` reports schema 9, integrity `ok`, WAL, foreign keys enabled, and `ok=true`.
3. The live project Operator remains `configured=false`, `stored=false`, and `ready=false`.
4. A consistent pre-acceptance database backup has been created and published.
5. The VibeBus plugin is installed and enabled.
6. The user creates two separate top-level Codex tasks for the same project. Do not use subagents or fork the current task as a substitute.

Each new task must load the `vibebus-coordination` Skill before acting. Prefer the VibeBus MCP tools. If those tools are not exposed in that task, use the installed CLI at `D:\MyProjects\CoWork\plugins\vibebus\bin\vibebus.exe` with the absolute project root on every call.

## Preflight before creating either task

From the original project task, run the read-only preflight from a clean checkout before creating Task B or Task A:

```powershell
powershell -NoProfile -File .\scripts\preflight-desktop-acceptance.ps1 -ProjectRoot D:\MyProjects\CoWork
```

Require JSON `ok=true`, `summary.failed=0`, and `summary.skipped=0`. The preflight fails closed unless all of the following remain true:

- the run ID, Agent names, controller identity, four task IDs, dependencies, initial states, owners, versions, and authorization gate match this fixed fixture;
- both disposable Agents and both Windows credential entries are absent;
- none of the four fixture tasks has binding history and no matching reservation is active;
- schema 9 health, the live Operator default-deny state, the published backup path, its 512,000-byte size, and SHA-256 `0079a09f200dd5c7210c1dbb563da3b77f29b80b17d5c2504168a1bae230611c` match;
- Git is clean and `HEAD` equals its configured upstream.

The script performs no Agent registration or authentication, inbox read, task/reservation mutation, Operator/retention operation, or repository write. `-SkipGit` exists only for script development and isolated fixture checks; never use it for the real desktop run. If preflight fails, preserve its JSON evidence and stop instead of repairing or renaming fixture state ad hoc.

## Phase B1: establish the receiver

Run this phase in top-level Task B before starting Task A:

1. Confirm `desktop-b-20260717-01` does not already exist. Register it with role `desktop-acceptance-receiver` and credential storage enabled.
2. Require the registration response to redact both secrets. Then require credential status `stored=true`, `hasRecoveryKey=true`, and `tokenGeneration=1`.
3. Create subscription `desktop-20260717-01` filtered to `message_sent`, omitting a start sequence so it begins at the current tail.
4. Claim `DESKTOP-B-READY-001`, bind the real native Codex task ID without inventing a value, and complete the task only after registration, credential status, and subscription creation are verified.
5. Check the inbox and reservations, then stop with the exact marker `READY_FOR_A`. Do not peek the subscription yet.

## Phase A1: establish ownership and send the handoff

After Task B reports `READY_FOR_A`, start top-level Task A:

1. Confirm `desktop-a-20260717-01` does not already exist. Register it with role `desktop-acceptance-coordinator` and credential storage enabled.
2. Require redacted registration plus credential status `stored=true`, `hasRecoveryKey=true`, and `tokenGeneration=1`.
3. Require `DESKTOP-B-READY-001` to be completed and `DESKTOP-CLAIM-001` to be ready.
4. Claim `DESKTOP-CLAIM-001`, bind the real native task ID when available, and move it to `working` with the returned version.
5. Acquire an exclusive 600-second reservation on `acceptance/desktop-20260717-01/shared-resource` with idempotency key `desktop-20260717-01-reserve-a`.
6. Renew that exact reservation to 900 seconds with idempotency key `desktop-20260717-01-renew-a`. Require the renewed expiry to be later than the original expiry.
7. Send a structured handoff to Task B with:
   - task `DESKTOP-CLAIM-001`;
   - summary stating that A owns the claim and renewed reservation;
   - decisions containing the exact `key=value` entries `claimOwner=<A>`, `reservationId=<id>`, `originalExpiry=<milliseconds>`, and `renewedExpiry=<milliseconds>`;
   - next actions requiring B to replay/ACK the subscription delivery, ACK/close the handoff, attempt the claim and reservation conflicts, and report results;
   - idempotency key `desktop-20260717-01-handoff-a-b`.
8. Verify the handoff is high priority and requires ACK, retain the reservation, and stop with `WAITING_FOR_B_RESULT`.

## Phase B2: prove replay and cross-task conflicts

Resume Task B with the message `继续执行第二阶段`:

1. Peek subscription `desktop-20260717-01` twice before ACK. Require both responses to return the same non-empty delivery ID and the same event batch containing A's `message_sent` handoff event.
2. ACK that delivery twice. Require the first response `replayed=false`, the retry `replayed=true`, and both responses to share the same cursor and acknowledgement timestamp.
3. Read A's structured handoff from the inbox, verify its body, ACK it, and close it. Require the default inbox to hide it and closed-history inspection to retain it.
4. Inspect `DESKTOP-CLAIM-001` immediately before and after B attempts to claim it. Both snapshots must keep A as owner with status `working`, while B's claim must fail with conflict kind `conflict` and an error reporting that the task is not claimable at `status=working`. A successful claim or changed owner is an acceptance failure.
5. Attempt an overlapping exclusive reservation on `acceptance/desktop-20260717-01/shared-resource` with idempotency key `desktop-20260717-01-reserve-b-conflict`. This must fail with conflict kind `conflict` and an error naming A as the existing reservation owner. A successful reservation is an acceptance failure.
6. Claim `DESKTOP-B-RESULT-001`, bind it to B's real native Codex task ID, and complete it after the evidence below is assembled.
7. Send a structured result handoff to Task A with task `DESKTOP-B-RESULT-001`, idempotency key `desktop-20260717-01-handoff-b-a`, and these exact `key=value` decisions:
   - `deliveryId=<replayed delivery ID>`;
   - `firstAckReplayed=false`;
   - `retryAckReplayed=true`;
   - `subscriptionAckAt=<first/replayed shared acknowledgement timestamp>`;
   - `aToBHandoffAckAt=<timestamp>` and `aToBHandoffClosedAt=<timestamp>`;
   - `claimConflictKind=conflict`;
   - `claimOwnerBefore=desktop-a-20260717-01` and `claimOwnerAfter=desktop-a-20260717-01`;
   - `claimStatusBefore=working` and `claimStatusAfter=working`;
   - `reservationConflictKind=conflict`.
8. Stop with `B_RESULT_SENT`. Do not delete credentials or alter repository files.

## Phase A2: close the durable loop

Resume Task A with the message `继续执行收尾阶段`:

1. Use credential status and handoff snapshot to resume without a pasted token.
2. Require `DESKTOP-B-RESULT-001` to be completed.
3. Read B's structured result handoff, verify every evidence field, ACK it, and close it.
4. Release A's reservation and require the active reservation list to contain no matching path.
5. Complete `DESKTOP-CLAIM-001`, closing any active task/thread binding.
6. Claim `DESKTOP-A-FINALIZE-001`, bind it to A's real native Codex task ID, and complete it.
7. Send a final structured handoff to `git-publisher-019f6eab` with task `DESKTOP-A-FINALIZE-001`, idempotency key `desktop-20260717-01-handoff-a-root`, and the following exact `key=value` decisions copied from authoritative results rather than re-created from memory:
   - `agentAStored=true`, `agentAHasRecoveryKey=true`, `agentATokenGeneration=1`;
   - `agentBStored=true`, `agentBHasRecoveryKey=true`, `agentBTokenGeneration=1`;
   - `claimOwner=desktop-a-20260717-01`;
   - `reservationId=<id>`, `originalExpiry=<milliseconds>`, `renewedExpiry=<milliseconds>`;
   - `deliveryId=<id>`, `firstAckReplayed=false`, `retryAckReplayed=true`, `subscriptionAckAt=<timestamp>`;
   - `aToBHandoffAckAt=<timestamp>`, `aToBHandoffClosedAt=<timestamp>`;
   - `bToAHandoffAckAt=<timestamp>`, `bToAHandoffClosedAt=<timestamp>`;
   - `claimConflictKind=conflict`, `reservationConflictKind=conflict`;
   - `acceptanceReservations=0`, `subscriptionPendingDelivery=false`.
8. Finish with `DESKTOP_ACCEPTANCE_READY_FOR_AUDIT`. Do not delete either Agent credential.

## Copy-ready Task B prompt

```text
Work in D:\MyProjects\CoWork as the independent receiver for the real desktop acceptance. Read docs/desktop-acceptance.md and load the vibebus-coordination Skill completely before acting. This must be a user-owned top-level Codex task, not a subagent. Use the fixed run desktop-20260717-01 and Agent desktop-b-20260717-01. Perform only Phase B1 now: register with stored credentials, prove redaction plus recovery-key retention, create the tail-starting message_sent subscription, complete DESKTOP-B-READY-001, check inbox/reservations, then stop with READY_FOR_A. Do not edit files, use Git, invoke Operator/retention commands, expose secrets, peek the subscription, or continue to B2 until the user says 继续执行第二阶段.
```

## Copy-ready Task A prompt

```text
Work in D:\MyProjects\CoWork as the independent coordinator for the real desktop acceptance. Read docs/desktop-acceptance.md and load the vibebus-coordination Skill completely before acting. This must be a user-owned top-level Codex task, not a subagent. Use the fixed run desktop-20260717-01 and Agent desktop-a-20260717-01. Require DESKTOP-B-READY-001 completed, then perform only Phase A1: register with stored credentials, prove redaction plus recovery-key retention, claim and work DESKTOP-CLAIM-001, bind the real native task ID if available, acquire and renew the exact shared reservation, send the idempotent structured handoff to B, and stop with WAITING_FOR_B_RESULT. Do not edit files, use Git, invoke Operator/retention commands, expose secrets, release the reservation, or continue to A2 until the user says 继续执行收尾阶段.
```

## Root audit and acceptance gates

The original project task performs the final audit after A reports `DESKTOP_ACCEPTANCE_READY_FOR_AUDIT`. Acceptance requires authoritative VibeBus evidence for every item:

- both Agent vault entries are stored and retain recovery keys;
- B readiness, B result, claim, and A finalize tasks have the expected owners and terminal states;
- all four terminal tasks retain closed bindings; A's two bindings share one native task ID, B's two bindings share another, and the A/B IDs differ;
- A's original and renewed reservation expiries prove owner renewal;
- B's durable result handoff records the claim and overlapping-reservation conflicts, with before/after task snapshots proving A remained the claim owner;
- B's subscription shows no pending delivery and preserves the acknowledged delivery ID;
- B's result records same-delivery peek replay plus first/retry ACK `false/true`;
- both structured handoffs were ACKed and closed by their recipients;
- A released the reservation and no acceptance reservation remains;
- the final handoff to the root evidence recipient is present and requires ACK;
- `doctor.ok=true`, the repository is clean, and the live Operator remains unconfigured.

Before ACKing the final handoff or editing the repository, run the non-destructive auditor from a clean checkout:

```powershell
powershell -NoProfile -File .\scripts\audit-desktop-acceptance.ps1 -ProjectRoot D:\MyProjects\CoWork
```

Require JSON `ok=true`, `summary.failed=0`, and `summary.skipped=0`. The auditor reads VibeBus status, Agent credential metadata, tasks, closed bindings, messages, reservation events, subscription state, backup identity, Operator state, and Git state. Authenticated reads may refresh the relevant Agent `lastSeenAt`, but the auditor never receives a bearer token or recovery key and performs no ACK, close, Operator, retention, task, reservation, artifact, subscription-cursor, repository, or event-producing mutation.

Only after the audit passes should the root task ACK and close the final handoff, update `docs/acceptance.md`, publish the evidence, and optionally remove the two disposable Agent vault entries. Credential deletion is explicit and irreversible for the local vault copy; never perform it before the recovery-key-retention evidence is durable. The subscription and database Agent rows remain audit history.

## Accepted execution record

The fixed run completed on 2026-07-18 exactly as staged:

| Gate | Accepted evidence |
| --- | --- |
| Preflight | `2026-07-18T05:21:07Z`; 68 passed, 0 failed, 0 skipped, including clean Git and upstream equality |
| Task B | User-owned top-level task `019f73ad-0618-76a1-9c42-e17a8fda1486`; final phase markers `READY_FOR_A` and `B_RESULT_SENT` |
| Task A | User-owned top-level task `019f73af-839c-7b03-a62b-09fd7eb07ec0`; final phase markers `WAITING_FOR_B_RESULT` and `DESKTOP_ACCEPTANCE_READY_FOR_AUDIT` |
| A-to-B handoff | `msg_42112ccc95584a26988c263ae60ed0b2`; ACKed and closed by B |
| B-to-A handoff | `msg_71fff6874eff46078a8ae895f975feff`; ACKed and closed by A |
| A-to-root handoff | `msg_438b66a994574f6393b977f7d958beec`; audited while unread/open, then ACKed at `1784353201423` and closed at `1784353206244` |
| Reservation | `rsv_8c26986f53cc46729b68f0ec3877d782`; expiry renewed from `1784353036068` to `1784353342645`, competing overlap rejected, finally released |
| Replay-safe delivery | `sdl_d287418ff18a4ef78d732a8c413afa6d`; same delivery on repeated peek, ACK replay `false` then `true`, no pending delivery |
| Audit | `2026-07-18T05:39:45Z`; 178 passed, 0 failed, 0 skipped |
| Recovery point | `backups/vibebus-0.8-desktop-acceptance.db`; 589,824 bytes; SHA-256 `5928201fd62fa0d5a7588a91650bfaf86ace173f0c43f2b10eb9f4c8f232d37b` |

All four fixture tasks are terminal, all four task/thread bindings are closed, no acceptance reservation remains, `doctor.ok=true`, and the live Operator remains unconfigured. The final 178-check audit intentionally ran before the root ACK/close, because unread/open status of that final evidence handoff is an audit gate. The retained A/B vault entries remain a deliberate regression asset until the maintainer explicitly chooses to delete them.
