# VibeBus · Cursor adapter

Coordinate independent Cursor agent tasks through one local SQLite fact bus. This adapter wires the `vibebus` MCP server, a Cursor rule, and the `vibebus-coordination` skill into Cursor.

## Prerequisites

Install the `vibebus` binary and place it on `PATH`:

```sh
cargo install --path .   # from the VibeBus repository root
vibebus init --root /path/to/project --name "My Project"
```

## Install this adapter

Copy these files into your project root:

- `.cursor/mcp.json` → `.cursor/mcp.json` (Cursor reads project-scoped MCP servers from here)
- `.cursor/rules/vibebus-coordination.mdc` → `.cursor/rules/vibebus-coordination.mdc` (Agent Requested rule)
- `.cursor/skills/vibebus-coordination/SKILL.md` → `.cursor/skills/vibebus-coordination/SKILL.md`

## How it maps to Cursor

| VibeBus concept | Cursor equivalent |
| --- | --- |
| MCP coordination server | `.cursor/mcp.json` `mcpServers.vibebus` |
| Coordination discipline | `vibebus-coordination` skill + `.cursor/rules/vibebus-coordination.mdc` |
| Lifecycle fact capture | **No hooks** — record Git/test facts explicitly via `vibebus_git_commit_record` / `vibebus_test_result_record` |
| Plugin packaging | `.cursor-plugin/plugin.json` (optional marketplace manifest) |

## Notes

- Cursor does not provide lifecycle hooks. The rule and skill instruct the agent to call the explicit fact-recording MCP tools after each commit/test run.
- Cursor's MCP tool names are namespaced; use the `vibebus_*` tools directly.
- Pass the absolute project `root` on every MCP call.
