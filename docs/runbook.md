# VibeBus MVP — 运行手册

VibeBus 跨 Agent 协作 MVP 的项目结构与端到端运行方式。本 MVP 覆盖：本地事实总线、Agent 生成层（spawn/reply/kill）、fact 层自动衔接，以及多工具 adapter。统一 TUI 为后续阶段。

## 项目结构

```
VibeBus/
├── src/
│   ├── lib.rs              # 模块导出
│   ├── main.rs             # CLI 入口（init/register/agent spawn/...）
│   ├── mcp.rs              # stdio MCP server（40+ vibebus_* 工具 + spawn/reply/kill/status）
│   ├── spawn.rs            # ★ Agent 生成层：画像/标记协议/spawn_once/SessionRegistry/link_spawn_outcome
│   ├── hooks.rs            # 生命周期 hooks（SessionStart/PostToolUse/Stop，多工具泛化）
│   ├── store.rs            # SQLite WAL 事实总线（agents/tasks/messages/decisions/...）
│   ├── models.rs / policy.rs / project.rs / vault.rs / error.rs
├── plugins/
│   ├── vibebus/            # Codex adapter（.codex-plugin + .mcp.json + hooks + skill）
│   ├── claude-code/        # Claude Code adapter
│   ├── cursor/             # Cursor adapter
│   ├── opencode/           # OpenCode adapter
│   └── antigravity/        # Antigravity adapter
├── docs/
│   ├── cross-agent-collaboration.md   # 跨 Agent 协作架构设计
│   ├── multi-tool-adapters.md         # 多工具适配说明
│   ├── runbook.md                     # 本文件
│   └── architecture.md / protocol.md / ...
├── Cargo.toml              # tokio(process/io-util/time/sync) + rmcp + rusqlite + ...
└── rust-toolchain.toml     # Rust 1.97.1
```

## 依赖

- Rust 1.97.1（`rust-toolchain.toml` 已钉版，`rustup` 自动安装）
- 无外部 CLI 依赖即可运行 MVP（内置 `demo` 画像用 `sh` 模拟 agent）
- 真实派生 codex/claude/gemini 需对应 CLI 已安装并在 PATH

## 构建

```sh
cargo build              # debug
cargo build --release    # release（产物在 target/release/vibebus）
```

`cargo fmt --check` / `cargo clippy -D warnings` / `cargo test --all-targets` 均应通过。

## 端到端运行（MVP 演示）

以下用内置 `demo` 画像，无需任何外部 agent CLI。所有命令输出 JSON。

```sh
# 1. 初始化一个 VibeBus 项目
vibebus init --root /path/to/project --name "My Project"
# → 写入 .vibebus/project.json + 创建 SQLite 数据库

# 2. 注册一个 Agent（不存凭证，明文返回 token 便于演示）
vibebus register --root /path/to/project --name impl --role implementation
# → { "result": { "agent": {...}, "token": "<bearer>", "recoveryKey": "..." } }
#   记下 token

# 3. 派生一个 agent（demo 画像）执行任务
vibebus agent spawn \
  --root /path/to/project \
  --profile demo \
  --agent impl \
  --token <token> \
  --task "review the auth module for security boundaries"
# → { "result": { "state": "result", "message": "demo agent completed the task", ... },
#     "link":   { "task_id": "spawn-...", "thread_id": "spawn:spawn-...", "completed": true } }
#   自动：create_task → claim → bind(spawn:{sessionId}) → 回写结果消息 → complete

# 4. 查看项目状态（应看到 completed 的 spawn 任务）
vibebus status --root /path/to/project
# → tasks: [{ status: "completed", title: "spawn: sh default" }]

# 5. 查看 inbox（应有 spawn 结果消息）
vibebus inbox --root /path/to/project --agent impl --token <token>
# → [{ subject: "spawn result: sh (Result)", body: "demo agent completed the task" }]
```

## 派生真实 agent

`.vibebus/agents.json`（项目根，可选）声明画像，叠加在内置默认（codex/claude/gemini/demo）上：

```json
{
  "profiles": {
    "aider": {
      "command": "aider",
      "args": ["--message"],
      "roles": ["executor"],
      "prompt_via_stdin": false
    }
  }
}
```

派生后，子 agent 按 VibeBus spawn 协议在 stdout 输出标记：
- `[RESULT] <答案>` — 完成
- `[QUESTION] <问题>` — 需澄清（仅 MCP 路径可 `vibebus_agent_reply` 续答）
- `[ERROR] <原因>` — 失败

## 双向协作（MCP 路径）

CLI 是一次性 spawn；交互式 `[QUESTION]` → reply 需 MCP server（会话持久化）：

```sh
vibebus mcp --root /path/to/project   # 启动 stdio MCP server
```

MCP 工具：`vibebus_agent_spawn` / `vibebus_agent_reply` / `vibebus_agent_kill` / `vibebus_agent_status`，外加全部事实总线工具（`vibebus_task_*` / `vibebus_send` / `vibebus_inbox` / `vibebus_context_sync` / ...）。

## 多工具 adapter

每个 coding agent 一个 adapter 包，详见 `docs/multi-tool-adapters.md`。把对应 adapter 的配置文件复制到项目即可接入。

## MVP 边界

- ✅ 已实现：事实总线、spawn/reply/kill、fact 自动衔接、5 工具 adapter、demo 可运行
- ⏳ 后续：统一 TUI（ratatui）、任务栏多面板、流水线编排、性能优化、边缘情况

## 验证状态

`cargo test --all-targets`：11 lib 单测 + 30 集成测试全过，含 spawn→[QUESTION]→reply→[RESULT] 双向往返测试。
