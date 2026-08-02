# VibeBus coordination (OpenCode)

This project uses VibeBus to coordinate independent OpenCode agent tasks through one local SQLite fact bus. The full protocol lives in `skills/vibebus-coordination/SKILL.md`; this file is the OpenCode-specific reminder.

## Setup

- The `vibebus` MCP server is configured in `opencode.json` under `mcp.vibebus` and exposes every `vibebus_*` tool.
- Pass the absolute project `root` on every MCP call.
- Initialize once with `vibebus init --root <absolute-root> --name "My Project"` (CLI) before the first task.

## OpenCode-specific fact recording

OpenCode has no lifecycle hooks, so record lifecycle evidence explicitly:

- After every meaningful `git commit`, call `vibebus_git_commit_record` with the commit SHA, bounded summary, and normalized changed paths (never a diff).
- After every test run, call `vibebus_test_result_record` with a stable result key, suite, outcome, and bounded summary (never raw logs).

## Per-turn checklist

1. `vibebus_status` — confirm project, inspect active agents/tasks/reservations.
2. `vibebus_register` once (storeCredentials=true) if this task has no identity.
3. `vibebus_inbox` — read directed messages; `vibebus_ack` + `vibebus_close` processed ones.
4. `vibebus_task_claim` a ready task; `vibebus_thread_bind` with the OpenCode session id when available.
5. `vibebus_reserve` the narrowest paths before editing; release when done.
6. `vibebus_context_sync` for deterministic task-scoped context (decisions, artifacts, Git/test facts).
7. Record Git/test facts explicitly (see above).
8. `vibebus_handoff_send` before stopping when a recipient exists; `vibebus_task_complete` with the latest version.

Use an `idempotencyKey` on every retried mutation. Never claim VibeBus can interrupt a running generation or awaken another task.

## Claude Code compatibility

OpenCode also reads `CLAUDE.md` and `~/.claude/skills/` as fallbacks. If you maintain a Claude Code adapter alongside this one, keep the coordination guidance identical to avoid drift.
