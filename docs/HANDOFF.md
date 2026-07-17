# VibeBus handoff

## Current baseline

VibeBus 0.4 is a working native Windows MVP. Rust core, CLI, stdio MCP, Codex Skill, SessionStart Hook, repo marketplace, tests, health check, backup, and packaging are present. Agent recovery, reservation renewal, retry idempotency, ordered events, replay-safe named subscriptions, structured handoffs, message closing, durable task/thread bindings, and resume snapshots are implemented.

Local recovery copies are kept under the ignored `backups/` directory:

- `vibebus-source-0.4-final.zip` is the final 0.4 source, documentation, marketplace, project marker, and packaged plugin, excluding build toolchains and targets.
- `vibebus-0.4-final.db` is the accepted schema-v7 project database snapshot produced by SQLite online backup; SHA-256 `be6066499084796baf4aa4ea1e78a701ccaecc5eac01a18855c2200f56c9ec31`.
- The 0.1 through 0.3 source/database backups remain available for rollback/reference.

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
- Replay-safe subscription peek/ack provides at-least-once batch access, not exactly-once consumer side effects. Legacy poll still consumes and commits in one call.
- Retention/compaction policy and remote synchronization are not implemented. Any future event retention must protect pending subscription delivery ranges and retain independently durable message/task state as required by policy.
- The optional best-effort bridge to native Codex thread tools is not implemented.
- Release signing, installer generation, and CI publishing are not implemented.
- Windows CLI callers should use `artifact publish --metadata-file` for complex JSON; MCP accepts metadata as a native object.

## Recommended next slice

1. Perform the two-real-task desktop acceptance recorded in `docs/acceptance.md`; plugin installation is complete, but creating user-owned top-level tasks requires explicit user action/authorization.
2. Add bounded retention/cleanup for events, closed message receipts, completed binding history, and idempotency records while protecting pending deliveries and audit requirements.
3. Integrate an OS credential vault without ever writing bearer or recovery secrets to the repository.
4. Add release signing, installer generation, and CI publishing.
5. Only then evaluate an optional Codex thread notification bridge; keep SQLite authoritative and treat UI delivery as best effort.

## Startup prompt

```text
Read README.md, docs/architecture.md, docs/protocol.md, docs/acceptance.md, and docs/HANDOFF.md. Verify the current checkout and run the test and clippy commands before changing code. Preserve independent Codex top-level tasks and the single SQLite source of truth. Treat bearer tokens and recovery keys as private one-time outputs. Preserve message close and task/thread terminal semantics. Prefer subscription peek/ack for replay-safe delivery, keep consumer side effects idempotent, and use legacy poll only when consume-on-poll loss is acceptable. Start with the first incomplete item under Recommended next slice, and do not claim that VibeBus can interrupt an already-running model generation.
```
