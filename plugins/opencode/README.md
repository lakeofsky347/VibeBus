# VibeBus · OpenCode adapter

Coordinate independent OpenCode agent tasks through one local SQLite fact bus. This adapter wires the `vibebus` MCP server, project instructions, and the `vibebus-coordination` skill into OpenCode.

## Prerequisites

Install the `vibebus` binary and place it on `PATH`:

```sh
cargo install --path .   # from the VibeBus repository root
vibebus init --root /path/to/project --name "My Project"
```

## Install this adapter

Copy these files into your project root:

- `opencode.json` → `opencode.json` (or merge into an existing `opencode.json`; the `mcp.vibebus` entry registers the server, and `instructions` loads `AGENTS.md` and the skill)
- `AGENTS.md` → `AGENTS.md` (OpenCode's primary instruction file)
- `skills/vibebus-coordination/SKILL.md` → `skills/vibebus-coordination/SKILL.md`

## How it maps to OpenCode

| VibeBus concept | OpenCode equivalent |
| --- | --- |
| MCP coordination server | `opencode.json` `mcp.vibebus` (`type: "local"`, `command: ["vibebus", "mcp"]`) |
| Coordination discipline | `AGENTS.md` + `instructions` referencing the SKILL |
| Lifecycle fact capture | **No hooks** — record Git/test facts explicitly via `vibebus_git_commit_record` / `vibebus_test_result_record` |
| Agent profile | optional `.opencode/agents/*.md` (this adapter does not require a custom agent) |

## Notes

- OpenCode MCP tool names follow `{server}_{tool}` (e.g. `vibebus_status`); OpenCode exposes them directly.
- OpenCode also reads `CLAUDE.md` and `~/.claude/skills/` as a Claude Code compatibility fallback, so the Claude Code adapter's files work here too if present.
- Pass the absolute project `root` on every MCP call.
