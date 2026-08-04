# VibeBus · Antigravity adapter

Coordinate independent Antigravity agent tasks through one local SQLite fact bus. This adapter packages the `vibebus` MCP server, lifecycle hooks, and the `vibebus-coordination` skill as an Antigravity plugin.

## Prerequisites

Install the `vibebus` binary and place it on `PATH`:

```sh
cargo install --path .   # from the VibeBus repository root
vibebus init --root /path/to/project --name "My Project"
```

## Install this plugin

Stage the plugin into the Antigravity plugins path:

```sh
agy plugin install /path/to/VibeBus/plugins/antigravity
```

Or copy `plugin.json`, `mcp_config.json`, `hooks.json`, and `skills/` into `~/.gemini/antigravity-cli/plugins/vibebus/`.

For workspace-local MCP without a full plugin, copy `mcp_config.json` to `.agents/mcp_config.json` at your project root.

## How it maps to Antigravity

| VibeBus concept | Antigravity equivalent |
| --- | --- |
| MCP coordination server | `mcp_config.json` `mcpServers.vibebus` (local stdio) |
| Plugin manifest | `plugin.json` (`name`, `description`) |
| Lifecycle fact capture | `hooks.json` (SessionStart / PostToolUse / Stop → `vibebus hook *`) |
| Coordination discipline | `skills/vibebus-coordination/SKILL.md` (compiles to a `/vibebus-coordination` slash command) |

Antigravity's plugin layout (`plugin.json` + `mcp_config.json` + `hooks.json` + `skills/`) is close to the Codex plugin layout, so the same `vibebus hook` subcommands and the common stdin-JSON / `hookSpecificOutput` protocol apply.

## Notes

- The Stop hook writes handoff proposals under `${AGENTS_PROJECT_DIR:-$PWD}/.vibebus/hook-data`. Add `.vibebus/hook-data/` to `.gitignore`.
- **Hook event names**: Antigravity's `hooks.json` schema is still maturing. The event names above (`SessionStart` / `PostToolUse` / `Stop`) and the matcher syntax mirror the Codex/Claude Code convention that VibeBus already speaks; confirm them against the current Antigravity plugin docs and adjust if Antigravity uses different event identifiers (e.g. `pre_tool` / `post_tool`). The `vibebus hook session-start|post-tool-use|stop` subcommands themselves are stable.
- The Antigravity SDK additionally exposes richer lifecycle hook categories (Inspect / Decide / Transform across nine points) for programmatic use; this adapter uses the simpler command-hook form.
- Pass the absolute project `root` on every `vibebus_*` MCP call.
