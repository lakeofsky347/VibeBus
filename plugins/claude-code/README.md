# VibeBus · Claude Code adapter

Coordinate independent Claude Code tasks through one local SQLite fact bus. This adapter wires the `vibebus` MCP server, lifecycle hooks, and the `vibebus-coordination` skill into Claude Code.

## Prerequisites

Install the `vibebus` binary and place it on `PATH`:

```sh
cargo install --path .   # from the VibeBus repository root
# or copy the release binary to /usr/local/bin
```

Initialize a VibeBus project once at the intended root:

```sh
vibebus init --root /path/to/project --name "My Project"
```

## Install this adapter

Copy these files into your project (or user config) so Claude Code loads them:

- `.mcp.json` → project root (Claude Code reads project-scoped MCP servers from here)
- `.claude/settings.json` → merge into `.claude/settings.json` at the project root (or `~/.claude/settings.json` for all projects)
- `skills/vibebus-coordination/SKILL.md` → `.claude/skills/vibebus-coordination/SKILL.md`

## How it maps to Claude Code

| VibeBus concept | Claude Code equivalent |
| --- | --- |
| MCP coordination server | `.mcp.json` `mcpServers.vibebus` |
| SessionStart context injection | `SessionStart` hook → `vibebus hook session-start` (`additionalContext`) |
| PostToolUse Git/test fact capture | `PostToolUse` hook on `Bash` → `vibebus hook post-tool-use` |
| Stop handoff proposal | `Stop` hook → `vibebus hook stop` (writes under `CLAUDE_PROJECT_DIR/.vibebus/hook-data`) |
| Coordination discipline | `vibebus-coordination` skill (SKILL.md standard) |

Claude Code's hook input/output schema (stdin JSON with `session_id`, `cwd`, `hook_event_name`, `tool_name`, `tool_input`, `tool_response`; stdout `hookSpecificOutput.additionalContext`) is identical to the protocol VibeBus already speaks, so the same `vibebus hook` subcommands work unchanged.

## Notes

- The Stop hook sets `PLUGIN_DATA` to `${CLAUDE_PROJECT_DIR}/.vibebus/hook-data` so handoff proposals stay inside the project. Add `.vibebus/hook-data/` to `.gitignore`.
- On Windows, replace the inline `PLUGIN_DATA=...` shell assignment with a PowerShell wrapper or set the env var in the session profile.
- Pass the absolute project `root` on every `vibebus_*` MCP call; the MCP process runs from the working directory, not the plugin directory.
