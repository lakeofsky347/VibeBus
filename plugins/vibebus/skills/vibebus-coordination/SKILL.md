---
name: vibebus-coordination
description: Coordinate independent Codex top-level tasks in the same project with VibeBus recoverable identities, directed messages, ACKs, atomic task claims, renewable file reservations, event subscriptions, and structured handoffs. Use when several Codex tasks or worktrees must exchange durable facts without sharing full chat context.
---

# VibeBus coordination

Use the bundled `vibebus` MCP server as the single coordination interface. VibeBus stores structured project facts in one local SQLite database; it does not merge chat transcripts or interrupt a model that is already generating.

## Start or resume a task

1. Resolve the absolute project root. Use the nearest ancestor containing `.vibebus/project.json`.
2. Pass that absolute path as `root` on every MCP call. The bundled MCP process runs from the installed plugin directory, not from the user's repository.
3. Call `vibebus_status` to confirm the project and inspect active agents, tasks, and reservations.
4. If this top-level Codex task has no VibeBus identity yet, call `vibebus_register` once with a short unique name and a role. Retain both returned `token` and `recoveryKey` only in private task/credential context. Never commit them, include them in a message, or print them into project files.
5. On a resumed task with a valid token, call `vibebus_handoff_snapshot`. It combines unread messages, owned work, active reservations, recent artifacts, and recent events. Use direct list/show calls when the bounded snapshot is insufficient.
6. If the bearer token is lost but the private recovery key remains, call `vibebus_agent_recover` once and replace both stored secrets with the returned pair. Never register a duplicate identity to bypass authentication. A legacy identity with a working token can call `vibebus_recovery_provision`.
7. Call `vibebus_inbox` at startup, after compaction or resume, before important decisions, and before the final response.

## Work protocol

- Claim a ready task with `vibebus_task_claim` before performing it. Treat a claim conflict as proof that another task won.
- Use the returned task `version` for later updates. On a version conflict, re-read the task with `vibebus_task_show`; never overwrite blindly.
- Before editing, reserve the narrowest practical project-relative file or directory path with `vibebus_reserve`. A reservation is advisory but conflict-checked and expires automatically. Renew it with `vibebus_reservation_renew` before expiry when work continues.
- Release reservations promptly after the edit scope is complete.
- Send facts, decisions, blockers, contract changes, and artifact paths with `vibebus_send`. Keep the body concise and self-contained; do not send entire chat transcripts.
- Use `vibebus_handoff_send` for a resumable transfer. Include a concise summary, task ID when applicable, decisions, verified artifact IDs, blockers, and next actions. It is always high priority and requires recipient ACK.
- Use `requiresAck` for information that another task must explicitly consume. Recipients should call `vibebus_ack` after acting on it.
- Give every externally retried send, handoff, reservation acquire/renew, or artifact publish a stable `idempotencyKey`. Reuse it only for the identical logical request; a changed payload must use a new key.
- For lightweight change detection, create one named `vibebus_subscription_create` subscription and poll it at safe boundaries. Omit `fromSequence` to start at the current tail; use `0` only when deliberate history replay is needed. Poll commits the cursor, so critical instructions must also be messages/tasks rather than event-only payloads.
- Complete work with `vibebus_task_complete` using the latest version. Dependency tasks unlock automatically after prerequisites complete.

## Conflict handling

- Authentication failure: verify the task identity and token; never register a duplicate identity merely to bypass the error.
- Claim conflict: stop work on that task and choose another ready task.
- Reservation conflict: inspect `vibebus_reservations`, narrow the requested path, or coordinate with the owner by message.
- Reservation expired: acquire a new reservation; renewal cannot revive a released or expired lease.
- Version conflict: re-read current state, reconcile the new facts, then retry with the new version.
- Idempotency conflict: do not alter the request under the old key. Inspect the already-completed operation, then either accept it or choose a new key for a genuinely new mutation.
- Subscription poll conflict: another consumer advanced the same named cursor. Re-list the subscription and continue from its stored cursor; do not assume the returned batch was yours.
- Missing project marker: do not initialize a project implicitly. Tell the user that `vibebus init` must be run deliberately at the intended root.

## End of turn

Before handing off or stopping, check the inbox once, publish verified artifacts, send a structured handoff or blocker, release reservations no longer needed, and ensure owned task state reflects reality. Never claim that VibeBus can awaken or inject text into an already-running Codex generation, or that a subscription response lost after cursor commit will replay automatically.
