# VibeBus handoff

## Current baseline

VibeBus 0.2 is a working native Windows MVP. Rust core, CLI, stdio MCP, Codex Skill, SessionStart Hook, repo marketplace, tests, health check, backup, and packaging are present. Agent recovery, reservation renewal, retry idempotency, ordered events, durable named subscriptions, structured handoffs, and resume snapshots are implemented.

Local recovery copies are kept under the ignored `backups/` directory:

- `vibebus-source-0.2.zip` is the 0.2 source, documentation, marketplace, project marker, and packaged plugin, excluding build toolchains and targets.
- `vibebus-0.2-final.db` is the accepted schema-v5 project database snapshot produced by SQLite online backup.
- The 0.1 source and database backups remain available for rollback/reference.

Run these first:

```powershell
git status --short
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
powershell -File .\scripts\package-plugin.ps1
```

The project truth is in `README.md`, `docs/architecture.md`, `docs/protocol.md`, and `docs/acceptance.md`.

## Known boundaries

- The packaged binary is Windows-only; the hook has a Windows implementation and a no-op Unix command.
- Plugin MCP calls must pass an explicit absolute project `root`.
- Agent bearer tokens and recovery keys are returned only at creation/rotation and retained by the calling task; secure OS credential persistence is not implemented.
- Message polling occurs at safe task boundaries. There is no promise of interrupting an active generation.
- Named subscription polling consumes and commits its cursor in one call; a lost response after commit is not replayed automatically. Critical handoffs remain durable inbox messages.
- Message closing, subscription peek/ack delivery, thread bindings, retention/compaction policy, and remote synchronization are not implemented.
- The optional best-effort bridge to native Codex thread tools is not implemented.
- Release signing, installer generation, and CI publishing are not implemented.
- Windows CLI callers should use `artifact publish --metadata-file` for complex JSON; MCP accepts metadata as a native object.

## Recommended next slice

1. Perform the two-real-task desktop acceptance recorded in `docs/acceptance.md`; plugin installation is complete, but creating user-owned top-level tasks requires explicit user action/authorization.
2. If consumers require at-least-once event delivery, split subscription polling into peek plus explicit cursor acknowledgement while keeping current consume-on-poll behavior versioned.
3. Add message closing, task/thread bindings, and bounded retention/cleanup for events and idempotency records.
4. Integrate an OS credential vault without ever writing bearer or recovery secrets to the repository.
5. Only then evaluate an optional Codex thread notification bridge; keep SQLite authoritative and treat UI delivery as best effort.

## Startup prompt

```text
Read README.md, docs/architecture.md, docs/protocol.md, docs/acceptance.md, and docs/HANDOFF.md. Verify the current checkout and run the test and clippy commands before changing code. Preserve independent Codex top-level tasks and the single SQLite source of truth. Treat bearer tokens and recovery keys as private one-time outputs. Start with the first incomplete item under Recommended next slice, and do not claim that VibeBus can interrupt an already-running model generation or guarantee replay after a committed consume-on-poll response is lost.
```
