# Multi-tool adapters

VibeBus is tool-agnostic: one `vibebus` binary (CLI + MCP server), one SQLite fact bus, one domain model. The only thing that varies per coding agent is the *adapter package* — the MCP config, lifecycle hooks (when available), plugin manifest, and skill wiring that connects the agent to VibeBus.

## Why adapters

VibeBus was originally a Codex plugin. Codex, Claude Code, Cursor, OpenCode, and Antigravity each expose extensibility differently:

| Capability | Codex | Claude Code | Cursor | OpenCode | Antigravity |
| --- | --- | --- | --- | --- | --- |
| MCP server | `.mcp.json` | `.mcp.json` | `.cursor/mcp.json` | `opencode.json` (`mcp`) | `mcp_config.json` |
| Lifecycle hooks | `hooks/hooks.json` | `.claude/settings.json` | none | none | `hooks.json` |
| Hook event model | SessionStart / PostToolUse / Stop | identical to Codex | — | — | pre/post tool |
| Skills | `skills/*/SKILL.md` | `.claude/skills/` | `.cursor/skills/` | `~/.claude/skills/` (CC compat) | `skills/*.md` |
| Plugin manifest | `.codex-plugin/plugin.json` | (none; `.claude/`) | `.cursor-plugin/plugin.json` | (none; `.opencode/`) | `plugin.json` |
| Rules / instructions | skills | `CLAUDE.md` / `.claude/rules/` | `.cursor/rules/*.mdc` | `AGENTS.md` | `rules/` |

The MCP server (`vibebus mcp`, built on `rmcp`) and the 40+ `vibebus_*` tools are identical across all adapters — only the configuration file that launches them differs.

## Adapter packages

Each adapter lives under `plugins/`:

```
plugins/
├── vibebus/            # Codex adapter (canonical plugin; .codex-plugin/plugin.json)
├── claude-code/        # Claude Code adapter (.mcp.json + .claude/settings.json hooks)
├── cursor/             # Cursor adapter (.cursor/mcp.json + rules + skill)
├── opencode/           # OpenCode adapter (opencode.json + AGENTS.md)
└── antigravity/        # Antigravity adapter (plugin.json + mcp_config.json + hooks.json)
```

Every adapter ships the same `vibebus-coordination` skill (a tool-agnostic coordination discipline). The canonical copy lives at `plugins/vibebus/skills/vibebus-coordination/SKILL.md`; the other adapters hold identical copies. Keep them in sync when editing the protocol.

## Hook availability

Lifecycle hooks let VibeBus capture bounded Git/test facts automatically and inject session-start context. Hook availability determines how much is automatic:

- **Codex, Claude Code, Antigravity** — hooks fire SessionStart (context injection), PostToolUse (Git commit / test result capture), and Stop (handoff proposal). The hook input/output schema (stdin JSON with `session_id`, `cwd`, `hook_event_name`, `tool_name`, `tool_input`, `tool_response`; stdout `hookSpecificOutput.additionalContext`) is a shared protocol, so the same `vibebus hook session-start|post-tool-use|stop` subcommands serve all three.
- **Cursor, OpenCode** — no lifecycle hooks. The agent records Git/test facts explicitly via `vibebus_git_commit_record` and `vibebus_test_result_record` after each meaningful commit or test run, as documented in the skill and the per-adapter rule/AGENTS.md.

## Thread / session binding

A task may bind to the host agent's session identifier via `vibebus_thread_bind`. The identifier may be the raw session id (portable) or a `{tool}:{session_id}` form. The hook resolver accepts any of: `codex:`, `claude:`, `cursor:`, `opencode:`, `antigravity:`, `gemini:`, or the bare session id. This keeps bindings portable across tools while still letting a hook resolve the active task from the current session.

## Adding a new tool

1. Create `plugins/<tool>/` with the tool's MCP config pointing at `vibebus mcp`.
2. If the tool has lifecycle hooks, add a hooks config that calls `vibebus hook session-start|post-tool-use|stop` (set `PLUGIN_DATA` to a project-local writable dir for the Stop proposal).
3. Copy the canonical `SKILL.md` into the adapter's skills location and add a rules/instructions file if the tool uses one.
4. If the tool has a plugin manifest format, add it.
5. Add the tool prefix to `AGENT_TOOL_PREFIXES` in `src/hooks.rs` if it uses a `{tool}:{session_id}` binding convention.
6. Add a row to the table above and a `README.md` in the adapter directory.
