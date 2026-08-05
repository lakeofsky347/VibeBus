# 多工具适配器

VibeBus 与工具无关（tool-agnostic）：一个 `vibebus` 二进制（CLI + MCP 服务器）、一个 SQLite 事实总线、一个领域模型。每个编码智能体唯一不同的部分是*适配器包（adapter package）*——即将该智能体连接到 VibeBus 的 MCP 配置、生命周期钩子（如可用）、插件清单和技能接线。

## 为什么需要适配器

VibeBus 最初是 Codex 插件。Codex、Claude Code、Cursor、OpenCode 和 Antigravity 各自以不同的方式暴露扩展能力：

| 能力 | Codex | Claude Code | Cursor | OpenCode | Antigravity |
| --- | --- | --- | --- | --- | --- |
| MCP 服务器 | `.mcp.json` | `.mcp.json` | `.cursor/mcp.json` | `opencode.json` (`mcp`) | `mcp_config.json` |
| 生命周期钩子 | `hooks/hooks.json` | `.claude/settings.json` | 无 | 无 | `hooks.json` |
| Hook 事件模型 | SessionStart / PostToolUse / Stop | 与 Codex 相同 | — | — | tool 前后（pre/post） |
| 技能 | `skills/*/SKILL.md` | `.claude/skills/` | `.cursor/skills/` | `~/.claude/skills/`（CC 兼容） | `skills/*.md` |
| 插件清单 | `.codex-plugin/plugin.json` | （无；`.claude/`） | `.cursor-plugin/plugin.json` | （无；`.opencode/`） | `plugin.json` |
| 规则 / 指令 | skills | `CLAUDE.md` / `.claude/rules/` | `.cursor/rules/*.mdc` | `AGENTS.md` | `rules/` |

MCP 服务器（`vibebus mcp`，基于 `rmcp` 构建）和 58 个 `vibebus_*` 工具在所有适配器中完全相同——只有启动它们的配置文件不同。

## 适配器包

每个适配器都位于 `plugins/` 目录下：

```
plugins/
├── vibebus/            # Codex adapter (canonical plugin; .codex-plugin/plugin.json)
├── claude-code/        # Claude Code adapter (.mcp.json + .claude/settings.json hooks)
├── cursor/             # Cursor adapter (.cursor/mcp.json + rules + skill)
├── opencode/           # OpenCode adapter (opencode.json + AGENTS.md)
└── antigravity/        # Antigravity adapter (plugin.json + mcp_config.json + hooks.json)
```

每个适配器都附带相同的 `vibebus-coordination` 技能（一种与工具无关的协作纪律）。权威副本位于 `plugins/vibebus/skills/vibebus-coordination/SKILL.md`；其他适配器持有完全相同的副本。编辑协议时请保持它们同步。

## 钩子的可用性

生命周期钩子（lifecycle hook）让 VibeBus 自动捕获有边界的 Git/测试事实，并注入会话启动上下文。钩子的可用性决定了有多少环节可以自动化：

- **Codex、Claude Code、Antigravity**——Hook 会触发 SessionStart（上下文注入）、PostToolUse（Git 提交 / 测试结果捕获）和 Stop（交接提议）。Hook 的输入/输出模式（stdin JSON，包含 `session_id`、`cwd`、`hook_event_name`、`tool_name`、`tool_input`、`tool_response`；stdout 输出 `hookSpecificOutput.additionalContext`）是共享协议，因此同一组 `vibebus hook session-start|post-tool-use|stop` 子命令即可服务这三种工具。
- **Cursor、OpenCode**——没有生命周期 Hook。智能体会在每次有意义的提交或测试运行后，通过 `vibebus_git_commit_record` 和 `vibebus_test_result_record` 显式记录 Git/测试事实，具体见技能文档及各适配器的 rules/AGENTS.md。

## 线程 / 会话绑定

任务可以通过 `vibebus_thread_bind` 绑定到宿主智能体的会话标识符。该标识符可以是原始会话 id（可移植）或 `{tool}:{session_id}` 形式。Hook 解析器接受以下任一前缀：`codex:`、`claude:`、`cursor:`、`opencode:`、`antigravity:`、`gemini:`，或裸会话 id。这样既保持了绑定跨工具的可移植性，又仍能让 Hook 从当前会话解析出活动任务。

## 添加新工具

1. 创建 `plugins/<tool>/`，其 MCP 配置指向 `vibebus mcp`。
2. 如果该工具有生命周期钩子，添加一个调用 `vibebus hook session-start|post-tool-use|stop` 的 Hook 配置（将 `PLUGIN_DATA` 设置为项目内可写目录，用于 Stop 提议）。
3. 将权威 `SKILL.md` 复制到适配器的技能目录中；如果该工具使用 rules/instructions 文件，则一并添加。
4. 如果该工具有插件清单格式，添加之。
5. 如果该工具采用 `{tool}:{session_id}` 绑定约定，将其前缀添加到 `src/hooks.rs` 中的 `AGENT_TOOL_PREFIXES`。
6. 在上面的表格中添加一行，并在适配器目录中添加 `README.md`。
