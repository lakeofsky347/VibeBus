# VibeBus 跨 Agent 协作架构设计

> 状态：设计提案（部分已落地）· 基于 codex-plugin-cc、pal-mcp-server、agent-link-mcp 三个开源项目的架构模式提炼
> 目标：在 VibeBus 现有「本地事实总线」之上，增加单一 Agent 多任务栏、跨 Agent 协作、统一 TUI 三项能力
> 落地进度：生成层（Agent Spawn Bridge，第 4.3 节）已在 0.11.0 实现；TUI（ratatui）与任务栏编排仍为未来阶段

## 1. 设计目标与参考模式提炼

### 1.1 三大升级目标

| # | 目标 | 含义 |
| --- | --- | --- |
| 1 | 单一 Agent 多任务栏协作 | 同一 Agent 内创建多个并行任务栏，各栏间支持上下文共享与任务分发 |
| 2 | 跨 Agent 协作 | 不同 Agent 实例间建立通信通道，实现任务委派、结果回传、状态同步 |
| 3 | 统一 TUI 呈现 | 所有任务状态/进度/输出在同一 TUI 中可视化，支持任务切换、实时日志、交互控制 |

### 1.2 参考项目模式提炼

| 项目 | 核心模式 | 借鉴点 |
| --- | --- | --- |
| `openai/codex-plugin-cc` | **Broker + State Manager**：长驻 Broker 经 JSON-RPC 复用 app-server；State Manager 持久化 jobId/日志/进度/结果，支持 `status`/`result` 异步查询；Subagent 委派；SessionStart/End/Stop hooks 生命周期 | 后台作业跟踪模型、长驻会话 Broker、hook 生命周期绑定 |
| `BeehiveInnovations/pal-mcp-server` | **clink（CLI-to-CLI 桥接）+ 角色子代理**：从当前 CLI 派生隔离子代理（planner/codereviewer），上下文隔离、只回传最终结果；会话线程化、多模型共识 | 角色特化、上下文隔离、子代理只回传摘要而非全文 |
| `agent-link-mcp` | **双向 spawn/reply**：`spawn_agent`/`spawn_agents`/`reply`/`kill_agent`/`get_status`；host 可向子代理提问，子代理也可反向提问；并行子任务；流水线编排（research→impl→review） | 双向问答协议、并行 spawn、会话状态机、两击求助规则 |

### 1.3 设计原则

VibeBus 已是一个成熟的**本地结构化事实总线**（SQLite WAL + 40+ MCP 工具 + 原子任务认领 + 消息/决策/制品/订阅/handoff）。本设计**不重造总线**，而是把上述三项目的的协作模式**映射到 VibeBus 已有事实原语之上**，仅新增最小必要的「生成层」与「TUI 层」。

- **事实为源**：所有协作状态（委派、结果、进度）都是 VibeBus 持久事实，崩溃/重连可恢复。
- **隔离优先**：子代理在独立进程/独立上下文运行，只通过事实总线交换摘要，不污染主上下文。
- **协议复用**：跨 Agent 通信复用已有的 message/handoff/task/subscription，不发明新传输层。

## 2. 总体分层架构

```
┌─────────────────────────────────────────────────────────────────────┐
│  ① 统一 TUI 渲染层   `vibebus tui`  (ratatui)                        │
│     任务栏切换 · 实时日志 · 消息流 · Agent 面板 · 交互控制              │
├─────────────────────────────────────────────────────────────────────┤
│  ② 协作编排层   Coordination Orchestrator                             │
│     多任务栏(Task Bars) · 任务分发(Dispatch) · Handoff · 订阅通知       │
├─────────────────────────────────────────────────────────────────────┤
│  ③ Agent 生成层   Agent Spawn Bridge                                  │
│     spawn_agent · spawn_agents · reply · kill_agent · get_status      │
│     (CLI 子进程 + 双向 stdin/stdout 通道 + 会话状态机)                  │
├─────────────────────────────────────────────────────────────────────┤
│  ④ 事实总线层   VibeBus Core (已有)                                    │
│     SQLite WAL · agents · tasks · messages · decisions · artifacts    │
│     reservations · subscriptions · events · thread_bindings · handoffs│
└─────────────────────────────────────────────────────────────────────┘
```

四层职责：

- **④ 事实总线层（已有，零改动）**：VibeBus 现有核心。所有协作事实落库，是唯一权威源。
- **③ Agent 生成层（新增，薄）**：参考 agent-link-mcp，把「派生一个外部 Agent CLI 子进程」封装成可追踪的会话。负责子进程生命周期、双向通信通道、会话状态机。它**不持有业务状态**——状态全部写回 ④ 层。
- **② 协作编排层（新增，中）**：参考 codex-plugin-cc 的 State Manager + pal 的角色子代理。管理「任务栏」概念、任务分发策略、handoff 编排、订阅→通知。它读写的全是 ④ 层事实。
- **① TUI 渲染层（新增，中）**：参考 codex-plugin-cc 的 `status`/`result` 查询，做成全功能终端 UI。只读 ④ 层 + 订阅事件流，不写业务状态（交互控制通过调用 ③/② 的 CLI/MCP 完成）。

## 3. 单一 Agent 多任务栏协作

### 3.1 概念

「任务栏」是**单一 Agent 会话内的多个并行任务上下文**。VibeBus 已支持一个 Agent 拥有多个 task（`task_create` + `task_claim`），任务栏是把这种多任务能力在**交互层**呈现为可切换的面板。

```
Agent: implementation
├─ 任务栏 A  [task: auth-refactor]   working   ── 上下文: decisions·artifacts·依赖
├─ 任务栏 B  [task: api-rate-limit]  claimed   ── 上下文: reservation·test-fact
└─ 任务栏 C  [task: docs-update]     ready     ── blocked by A
```

### 3.2 机制（复用已有原语）

| 能力 | 复用的 VibeBus 原语 | 说明 |
| --- | --- | --- |
| 创建任务栏 | `vibebus_task_create` + `vibebus_task_claim` | 每栏一个 task，owner 为当前 Agent |
| 栏间上下文共享 | `vibebus_decision_confirm` + `vibebus_artifact_publish` | 决策/制品是任务级但可被 `context_sync` 投影到依赖任务 |
| 栏间任务分发 | `vibebus_send` (directed message) + `vibebus_task_dependencies` | A 完成后通过依赖解锁 C |
| 栏间路径隔离 | `vibebus_reserve` + `vibebus_responsibility_override` | 各栏预留不重叠路径，避免编辑冲突 |
| 栏状态同步 | `vibebus_subscription_create` + peek/ack | 一栏的 `task_completed` 事件可被另一栏订阅 |
| 栏上下文投影 | `vibebus_context_sync` | 按 task scope 拉取确定性上下文，互不污染 |

**关键点**：多任务栏**不需要新数据模型**——它只是「一个 Agent 的多个 task + 各自的 context_sync 视图」在 TUI 层的多面板呈现。隔离由已有的 task scope + reservation 保证。

### 3.3 任务栏调度

- 同一 Agent 同时持有的 `working` 任务栏建议 ≤ 3（避免上下文碎片）。
- 切换任务栏时，TUI 调用 `vibebus_context_sync(taskId=<该栏>)` 重新拉取该栏确定性上下文，cursor 续接。
- 长时间未触碰的栏可置 `blocked`(waiting-on-owner)，释放主上下文压力。

## 4. 跨 Agent 协作：通信机制

### 4.1 通信模型选型

参考三个项目后，VibeBus 采用**「事实总线为背板 + 生成层双向通道」**的混合模型：

- **异步事实通道（默认）**：跨 Agent 的任务委派/结果/状态走 VibeBus 的 message + handoff + task + subscription。这是**持久、可恢复、可审计**的通道，适合绝大多数协作。
- **同步双向通道（按需）**：当需要「子代理实时反问、host 即时回答」时（agent-link-mcp 的 reply 模式），生成层建立一条 stdin/stdout 双向管道，会话结束后**把摘要回写为事实**。

### 4.2 跨 Agent 委派协议（异步，主路径）

```
Host Agent (A)                          Subordinate Agent (B)
     │                                          │
     │ 1. vibebus_task_create(task=delegate-xxx, owner=A)
     │ 2. vibebus_send(to=B, subject=委派, body=任务说明,
     │           requiresAck=true, idempotencyKey=...)
     │ 3. vibebus_thread_bind(task, thread=B的会话id) ──────────▶ B 启动后 bind
     │ ─────────────────────────────────────────────────────────▶
     │                                          │ 4. B: vibebus_inbox → ack → claim task
     │                                          │ 5. B: 执行，vibebus_reserve 路径
     │                                          │ 6. B: vibebus_artifact_publish 产出
     │ ◀────────────────────────── 7. vibebus_handoff_send(to=A,
     │                                       summary=结果, artifacts=[...],
     │                                       decisions=[...], nextActions=[...])
     │ 8. A: vibebus_inbox → ack → vibebus_close
     │ 9. A: vibebus_task_complete(delegate-xxx)
```

**映射到已有原语**：委派 = send(requiresAck) + task(owner 委派给 B)；回传 = handoff_send；状态 = subscription 订阅 `task_*`/`message_*` 事件。**无新传输层**。

### 4.3 双向 spawn/reply 协议（同步，agent-link-mcp 模式）

新增 `vibebus agent spawn` 子命令族（生成层 ③），用于「派生一个外部 Agent CLI 子进程并双向对话」：

| 命令/MCP 工具 | 作用 | 对应 agent-link-mcp |
| --- | --- | --- |
| `vibebus_agent_spawn` | 派生 agent CLI 子进程，发送任务，返回 sessionId | `spawn_agent` |
| `vibebus_agent_spawn_batch` | 并行派生多个子进程处理独立子任务 | `spawn_agents` |
| `vibebus_agent_reply` | 向等待澄清的子代理回复，继续对话 | `reply` |
| `vibebus_agent_kill` | 终止子代理会话 | `kill_agent` |
| `vibebus_agent_status` | 列出所有活跃 spawn 会话及状态 | `get_status` |

**会话状态机**：`spawning → running → (question|result|error|timeout) → closed`。

**与事实总线的衔接（关键）**：
- `spawn` 时在 ④ 层创建一个 `task`(kind=delegate) + `thread_binding`(thread=`spawn:{sessionId}`)，并 `send` 一条 requiresAck 消息。
- 子代理的 `question`/`result` 经双向管道返回后，生成层把它们**摘要化**（参考 pal：只回传最终结果，不回传全文/日志）写为一条 `message` 或 `handoff`。
- 会话 `closed` 时，若 task 未 complete，生成层标记 task 状态，保证事实层与进程层一致。

**双向通道实现**：子进程 stdin/stdout 走行帧协议（JSON line），host 可随时 `reply`。超时由 `timeout` 参数控制（默认 1800s，同 agent-link-mcp）。

### 4.4 Agent 画像与能力发现

参考 agent-link-mcp 的 `list_agents` 自动探测 + pal 的角色特化：

- `.vibebus/agents.json`（新，可选）声明本机可派生的 Agent CLI 及其角色画像：
  ```json
  {
    "profiles": {
      "codex": {"command": "codex", "roles": ["executor","reviewer"], "auth": "codex login"},
      "claude": {"command": "claude", "roles": ["planner","reviewer"]},
      "gemini": {"command": "gemini", "roles": ["researcher"]},
      "aider": {"command": "aider", "roles": ["executor"]}
    }
  }
  ```
- `vibebus_agent_spawn` 接受 `role` 参数，生成层据此注入角色化系统提示（planner/codereviewer/...），实现 pal 的角色特化 + 上下文隔离。

### 4.5 两击求助规则（agent-link-mcp 实践）

在 coordination SKILL 中增加规则：当主 Agent 对同一问题连续失败两次，自动 `vibebus_agent_spawn` 另一模型求第二意见，传入错误信息与相关文件作为 context。这把 agent-link-mcp 的 CLAUDE.md 实践固化进 VibeBus 协作纪律。

## 5. 任务调度策略

### 5.1 调度规则表

| 维度 | 策略 | 依据 |
| --- | --- | --- |
| 任务认领 | 原子 `task_claim`，冲突即认输选他任务 | VibeBus 已有不变量 #2 |
| 就绪判定 | 依赖全部 complete → ready | 已有不变量 #4 |
| 并行度（单 Agent 任务栏） | 建议 ≤3 个 working，其余保持 ready/blocked | 上下文压力控制 |
| 并行度（跨 Agent spawn） | `spawn_batch` 并行处理**独立**子任务；有依赖则串行 | agent-link-mcp 模式 |
| 优先级 | message priority: low/normal/high/urgent；urgent 抢占式提醒 | 已有 |
| 路径冲突 | reservation 排他锁 + 责任策略 + override | 已有不变量 #6/#7/#34 |
| 流水线编排 | research→impl→review 各阶段一个 spawn，上阶段结果作为下阶段 context | agent-link-mcp 多阶段 |
| 失败重试 | 两击后 spawn 求助；不盲目重试同模型 | 两击规则 |
| 超时 | spawn 默认 1800s；超时 → kill + 标记 task | agent-link-mcp |
| 终止 | terminal 任务不可回退；spawn 会话 closed 同步关闭 thread_binding | 已有不变量 #5/#18 |

### 5.2 分发决策（编排层 ②）

编排器在收到一个新工作时，按以下决策树分发：

```
新工作到达
  ├─ 粒度小 & 在本 Agent 责任域内 & 无路径冲突
  │     → 创建任务栏（本 Agent 内多任务）
  ├─ 需要另一模型特长（review/research/不同模型执行）
  │     → vibebus_agent_spawn(role=...)，异步事实通道 + 同步双向通道二选一
  ├─ 可拆分为独立子任务
  │     → vibebus_agent_spawn_batch 并行
  └─ 长链路多阶段
        → 流水线：spawn(research) → spawn(impl, ctx=上阶段) → spawn(review)
```

## 6. 统一 TUI 渲染层

### 6.1 命令与定位

新增 `vibebus tui` 子命令（基于 `ratatui` + `crossterm`）。它是一个**只读可视化 + 交互控制**的终端应用：状态来自 ④ 层事实总线 + 事件订阅，控制动作通过调用 CLI/MCP 落回 ④ 层。

### 6.2 界面布局

```
┌─ VibeBus · MyProject ────────────────────── [● 3 agents · 5 tasks] ─┐
├──────────────┬───────────────────────────────────┬───────────────────┤
│ AGENTS       │ TASK BARS  [A][B][C]               │ MESSAGES / EVENTS │
│ ● impl       │ ─────────────────────────────────  │ ─────────────────│
│ ○ review     │ 18:02  ▶ task auth-refactor        │ ▲ handoff from    │
│ ○ research   │        working · reserved src/auth │   review → impl   │
│              │ 18:05  ▶ task api-rate-limit       │ 18:03 ack required│
│ TASKS        │        claimed · src/api           │ 18:06 decision    │
│ ▣ auth-refac │ 18:10  ▶ task docs-update          │   confirmed:retry │
│ ▢ api-rate   │        ready · blocked by A        │ 18:07 spawn       │
│ ▢ docs-update│ ─────────────────────────────────  │   codex:reviewing │
│              │ LOG (active bar: auth-refactor)    │                  │
│ SPAWNS       │ $ cargo test --lib auth            │ SUBSCRIPTIONS     │
│ ◑ codex-rev  │ running test_auth_token ... ok     │ ◷ deliver #42     │
│ ○ gemini-res │ 18:04 ✓ commit a1b2c3 recorded     │   peek 12 events  │
│              │ 18:06 ✗ test failed (rate-limit)   │                  │
│              │                                    │                  │
├──────────────┴───────────────────────────────────┴───────────────────┤
│ > :claim auth-refactor | :spawn codex --role reviewer | :switch B     │
└──────────────────────────────────────────────────────────────────────┘
```

### 6.3 面板职责

| 面板 | 数据来源 | 交互 |
| --- | --- | --- |
| 左：Agents/Tasks/Spawns | `agents`/`task_list`/`agent_status` | 选中切换焦点 |
| 中：Task Bars + Log | `context_sync`(按选中栏) + spawn 子进程 stdout 流 | Tab 切换栏；栏内实时日志 |
| 右：Messages/Events/Subs | `inbox` + `event list` + `subscription peek` | ack/close/回复 |
| 底：命令栏 | — | `:claim`/`:spawn`/`:switch`/`:handoff` 等斜杠命令，调用 CLI/MCP |

### 6.4 实现要点

- **只读订阅驱动**：TUI 启动时为当前 Agent 建一条 `subscription`，peek/ack 拉事件增量刷新各面板；不轮询全量。
- **日志流**：spawn 子进程的 stdout 经生成层行帧解析后，按 sessionId 路由到对应任务栏的日志区（参考 codex-plugin-cc State Manager 的日志跟踪）。
- **崩溃恢复**：TUI 退出/崩溃不影响事实层；重启后 `handoff_snapshot` + `context_sync` 重建各栏视图。
- **交互控制**：斜杠命令转译为 `vibebus` CLI 调用（同进程或子进程），保持「TUI 不持有业务状态」。
- **密度**：中等密度（compact），适配终端高数据环境；关键状态用色：working=蓝、claimed=青、ready=灰、blocked=橙、failed=红、completed=绿。

## 7. 与 VibeBus 现有能力的关系

| 层 | 复用 | 新增 |
| --- | --- | --- |
| ④ 事实总线 | 全部（agents/tasks/messages/decisions/artifacts/reservations/subscriptions/handoffs/thread_bindings） | 无 |
| ③ 生成层 | thread_binding(`spawn:{id}`)、message、handoff、task | `vibebus agent spawn`/`reply`/`kill`/`status` 子命令 + MCP 工具；`.vibebus/agents.json` 画像 |
| ② 编排层 | task_dependencies、send、handoff_send、subscription | 任务栏多面板调度逻辑、分发决策树、两击求助规则（SKILL） |
| ① TUI | status、context_sync、inbox、events、subscription_peek | `vibebus tui`（ratatui） |

**核心结论**：跨 Agent 协作的「通信机制」几乎完全复用 VibeBus 已有事实原语（message/handoff/task/subscription）；真正新增的是「生成层」（spawn 双向通道）和「TUI 层」（统一可视化）。这保证了既有不变量（#1–#40）全部成立，新能力是叠加而非替换。

## 8. 落地路线图

| 阶段 | 交付 | 依赖 |
| --- | --- | --- |
| P1 · 生成层 MVP | `vibebus_agent_spawn`/`reply`/`kill`/`status`（单 agent，同步双向通道）+ thread_binding 衔接 + `.vibebus/agents.json` | ④ 已就绪 |
| P2 · 并行与角色 | `spawn_batch`、角色画像注入、两击求助规则进 SKILL | P1 |
| P3 · TUI 骨架 | `vibebus tui`：Agents/Tasks/Messages 三面板 + 订阅驱动刷新 + 斜杠命令 | P1 |
| P4 · 任务栏与日志流 | 多任务栏切换、context_sync 续接、spawn 日志路由 | P2/P3 |
| P5 · 流水线编排 | 多阶段 spawn、上下文衔接、handoff 自动化 | P2 |

每阶段均可在 `glm/trial` 工作树验证：Rust 编译 + 既有 30 测试不回归 + 新增 spawn/tui 单测。

## 9. 风险与边界

- **spawn 子进程稳定性**：依赖外部 Agent CLI（codex/claude/gemini）的登录与稳定性（同 codex-plugin-cc 的已知依赖）。生成层须对子进程崩溃做 `error` 状态标记 + 事实回写。
- **上下文膨胀**：spawn 回传必须摘要化（pal 模式），严禁回传全文日志/转录——这与 VibeBus「不共享整段转录」的非目标一致。
- **TUI 跨平台**：ratatui + crossterm 在 Windows 终端需验证 ANSI 支持；operator 交互式终端要求沿用。
- **安全边界**：spawn 的子代理继承本机用户权限，须在 SKILL 中明确「仅派生可信 agent CLI」，与 VibeBus 既有「不防护同用户恶意进程」边界一致。
