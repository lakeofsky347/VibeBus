# VibeBus Cross-Agent Collaboration Architecture Design

> Status: design proposal (partially implemented) · distilled from the architectural patterns of three open-source projects: codex-plugin-cc, pal-mcp-server, and agent-link-mcp
> Goal: on top of VibeBus's existing "local fact bus", add three capabilities: single-Agent multi-task bars, cross-Agent collaboration, and a unified TUI
> Implementation progress: the spawn layer (Agent Spawn Bridge, Section 4.3) was implemented in 0.11.0; the TUI (ratatui) and task-bar orchestration remain future work

## 1. Design Goals and Reference Pattern Distillation

### 1.1 The Three Upgrade Goals

| # | Goal | Meaning |
| --- | --- | --- |
| 1 | Single-Agent multi-task-bar collaboration | Create multiple parallel task bars within one Agent, with context sharing and task dispatch across bars |
| 2 | Cross-Agent collaboration | Establish a communication channel between different Agent instances for task delegation, result reporting back, and status synchronization |
| 3 | Unified TUI presentation | Visualize all task status/progress/output in a single TUI, with task switching, real-time logs, and interactive control |

### 1.2 Distilled Patterns from Reference Projects

| Project | Core pattern | What we borrow |
| --- | --- | --- |
| `openai/codex-plugin-cc` | **Broker + State Manager**: a long-lived Broker reuses app-server over JSON-RPC; State Manager persists jobId/logs/progress/results and supports async `status`/`result` queries; Subagent delegation; SessionStart/End/Stop hook lifecycle | background job tracking model, long-lived session Broker, hook lifecycle binding |
| `BeehiveInnovations/pal-mcp-server` | **clink (CLI-to-CLI bridge) + role-based subagents**: derive isolated subagents (planner/codereviewer) from the current CLI, with context isolation and final-result-only replies; threaded sessions, multi-model consensus | role specialization, context isolation, subagents return only summaries rather than full transcripts |
| `agent-link-mcp` | **Bidirectional spawn/reply**: `spawn_agent`/`spawn_agents`/`reply`/`kill_agent`/`get_status`; the host can ask subagents questions and subagents can ask questions back; parallel subtasks; pipeline orchestration (research→impl→review) | bidirectional Q&A protocol, parallel spawn, session state machine, two-strike help rule |

### 1.3 Design Principles

VibeBus is already a mature **local structured fact bus** (SQLite WAL + 58 MCP tools + atomic task claiming + messages/decisions/artifacts/subscriptions/handoffs). This design **does not rebuild the bus**; instead it **maps the collaboration patterns of the three projects onto VibeBus's existing fact primitives**, adding only the minimal necessary "spawn layer" and "TUI layer".

- **Facts as source of truth**: all collaboration state (delegation, results, progress) is durable VibeBus facts, recoverable after a crash or reconnect.
- **Isolation first**: subagents run in separate processes/contexts and exchange only summaries through the fact bus, never polluting the main context.
- **Protocol reuse**: cross-Agent communication reuses the existing message/handoff/task/subscription primitives instead of inventing a new transport layer.

## 2. Overall Layered Architecture

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

Responsibilities of the four layers:

- **④ Fact bus layer (existing, zero changes)**: VibeBus's current core. All collaboration facts are persisted here; it is the single source of authority.
- **③ Agent spawn layer (new, thin)**: following agent-link-mcp, it wraps "spawning an external Agent CLI subprocess" into a trackable session. It owns the subprocess lifecycle, the bidirectional communication channel, and the session state machine. It **holds no business state** — all state is written back to layer ④.
- **② Coordination Orchestrator (new, medium)**: modeled on codex-plugin-cc's State Manager plus pal's role-based subagents. It manages the "task bar" concept, task dispatch policy, handoff orchestration, and subscription→notification. Everything it reads and writes are layer-④ facts.
- **① TUI rendering layer (new, medium)**: following codex-plugin-cc's `status`/`result` queries, this is a full-featured terminal UI. It only reads layer ④ and the subscribed event stream; it writes no business state (interaction control is done through CLI/MCP calls into layers ③/②).

## 3. Single-Agent Multi-Task-Bar Collaboration

### 3.1 Concept

A "task bar" is one of **multiple parallel task contexts within a single Agent session**. VibeBus already lets one Agent hold multiple tasks (`task_create` + `task_claim`); the task bar is how this multi-tasking capability is presented at the **interaction layer** as switchable panels.

```
Agent: implementation
├─ 任务栏 A  [task: auth-refactor]   working   ── 上下文: decisions·artifacts·依赖
├─ 任务栏 B  [task: api-rate-limit]  claimed   ── 上下文: reservation·test-fact
└─ 任务栏 C  [task: docs-update]     ready     ── blocked by A
```

### 3.2 Mechanism (Reusing Existing Primitives)

| Capability | Reused VibeBus primitive | Notes |
| --- | --- | --- |
| Create a task bar | `vibebus_task_create` + `vibebus_task_claim` | One task per bar, owned by the current Agent |
| Share context across bars | `vibebus_decision_confirm` + `vibebus_artifact_publish` | Decisions/artifacts are task-scoped but can be projected onto dependent tasks via `context_sync` |
| Dispatch tasks across bars | `vibebus_send` (directed message) + `vibebus_task_dependencies` | When A completes, C is unlocked through its dependency |
| Isolate paths across bars | `vibebus_reserve` + `vibebus_responsibility_override` | Each bar reserves non-overlapping paths to avoid edit conflicts |
| Sync bar status | `vibebus_subscription_create` + peek/ack | A `task_completed` event in one bar can be subscribed to by another bar |
| Project bar context | `vibebus_context_sync` | Pulls deterministic context per task scope without cross-contamination |

**Key point**: multiple task bars **require no new data model** — they are simply the TUI-layer, multi-panel rendering of "one Agent's multiple tasks + their individual `context_sync` views". Isolation is guaranteed by the existing task scope + reservation.

### 3.3 Task-Bar Scheduling

- Keep the number of simultaneously `working` task bars per Agent ≤ 3 (to avoid context fragmentation).
- When switching task bars, the TUI calls `vibebus_context_sync(taskId=<that bar>)` to re-pull the bar's deterministic context and resume the cursor where it left off.
- Bars left untouched for a long time can be set to `blocked` (waiting-on-owner) to relieve pressure on the main context.

## 4. Cross-Agent Collaboration: Communication Mechanisms

### 4.1 Choosing a Communication Model

Drawing on the three projects, VibeBus adopts a hybrid model of **"fact bus as backplane + spawn-layer bidirectional channel"**:

- **Async fact channel (default)**: cross-Agent task delegation/results/status flow through VibeBus's message + handoff + task + subscription. This channel is **durable, recoverable, and auditable**, and suits the vast majority of collaboration.
- **Sync bidirectional channel (on demand)**: when "the subagent asks a live question and the host answers immediately" is needed (agent-link-mcp's reply mode), the spawn layer opens a bidirectional stdin/stdout pipe and, once the session ends, **writes a summary back as facts**.

### 4.2 Cross-Agent Delegation Protocol (Async, Primary Path)

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

**Mapping to existing primitives**: delegation = send(requiresAck) + task(ownership delegated to B); return = handoff_send; status = a subscription on `task_*`/`message_*` events. **No new transport layer**.

### 4.3 Bidirectional Spawn/Reply Protocol (Sync, agent-link-mcp Pattern)

A new `vibebus agent spawn` command family (spawn layer ③) is added for "spawning an external Agent CLI subprocess and conversing bidirectionally with it":

| Command/MCP tool | Purpose | agent-link-mcp counterpart |
| --- | --- | --- |
| `vibebus_agent_spawn` | Spawn an agent CLI subprocess, send it a task, and return a sessionId | `spawn_agent` |
| `vibebus_agent_spawn_batch` | Spawn multiple subprocesses in parallel to handle independent subtasks | `spawn_agents` |
| `vibebus_agent_reply` | Reply to a subagent awaiting clarification and continue the conversation | `reply` |
| `vibebus_agent_kill` | Terminate a subagent session | `kill_agent` |
| `vibebus_agent_status` | List all active spawn sessions and their statuses | `get_status` |

**Session state machine**: `spawning → running → (question|result|error|timeout) → closed`.

**Integration with the fact bus (critical)**:
- On `spawn`, create a `task` (kind=delegate) + `thread_binding` (thread=`spawn:{sessionId}`) in layer ④, and `send` a requiresAck message.
- After the subagent's `question`/`result` comes back over the bidirectional pipe, the spawn layer **summarizes** them (following pal: return only final results, never full transcripts/logs) and writes a `message` or `handoff`.
- When the session goes `closed`, if the task is not yet complete, the spawn layer updates the task status so the fact layer and the process layer stay consistent.

**Bidirectional channel implementation**: the subprocess stdin/stdout uses a line-framing protocol (JSON lines); the host can `reply` at any time. Timeout is controlled by the `timeout` parameter (default 1800s, same as agent-link-mcp).

### 4.4 Agent Profiles and Capability Discovery

Following agent-link-mcp's `list_agents` auto-detection plus pal's role specialization:

- `.vibebus/agents.json` (new, optional) declares the Agent CLIs that can be spawned on this machine and their role profiles:
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
- `vibebus_agent_spawn` accepts a `role` parameter; the spawn layer uses it to inject a role-specific system prompt (planner/codereviewer/...), achieving pal-style role specialization + context isolation.

### 4.5 Two-Strike Help Rule (agent-link-mcp Practice)

Add a rule to the coordination SKILL: when the primary Agent fails twice in a row on the same problem, it automatically runs `vibebus_agent_spawn` with another model for a second opinion, passing the error message and the relevant files as context. This codifies agent-link-mcp's CLAUDE.md practice into VibeBus collaboration discipline.

## 5. Task Scheduling Strategy

### 5.1 Scheduling Rules

| Dimension | Policy | Basis |
| --- | --- | --- |
| Task claiming | Atomic `task_claim`; on conflict, back off and pick another task | Existing VibeBus invariant #2 |
| Readiness | A task is ready once all dependencies are complete | Existing invariant #4 |
| Parallelism (single-Agent task bars) | Keep ≤3 `working`, leave the rest `ready`/`blocked` | Context pressure control |
| Parallelism (cross-Agent spawn) | `spawn_batch` handles **independent** subtasks in parallel; dependent ones run serially | agent-link-mcp pattern |
| Priority | message priority: low/normal/high/urgent; urgent preempts with a reminder | Existing |
| Path conflicts | reservation exclusive lock + responsibility policy + override | Existing invariants #6/#7/#34 |
| Pipeline orchestration | one spawn per stage in research→impl→review; the previous stage's result becomes the next stage's context | agent-link-mcp multi-stage |
| Failure retry | After two strikes, spawn for help; never blindly retry the same model | Two-strike rule |
| Timeout | spawn defaults to 1800s; on timeout → kill + mark the task | agent-link-mcp |
| Termination | Terminal tasks are not reversible; when a spawn session goes closed, its thread_binding is closed too | Existing invariants #5/#18 |

### 5.2 Dispatch Decision (Orchestrator ②)

When the orchestrator receives new work, it dispatches it according to the following decision tree:

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

## 6. Unified TUI Rendering Layer

### 6.1 Command and Positioning

A new `vibebus tui` subcommand (built on `ratatui` + `crossterm`). It is a terminal app for **read-only visualization + interaction control**: state comes from the layer-④ fact bus + event subscriptions, and control actions are written back to layer ④ through CLI/MCP calls.

### 6.2 Interface Layout

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

### 6.3 Panel Responsibilities

| Panel | Data source | Interaction |
| --- | --- | --- |
| Left: Agents/Tasks/Spawns | `agents`/`task_list`/`agent_status` | Select to move focus |
| Center: Task Bars + Log | `context_sync` (for the selected bar) + spawn subprocess stdout stream | Tab to switch bars; real-time per-bar logs |
| Right: Messages/Events/Subs | `inbox` + `event list` + `subscription peek` | ack/close/reply |
| Bottom: command bar | — | Slash commands like `:claim`/`:spawn`/`:switch`/`:handoff`, invoking CLI/MCP |

### 6.4 Implementation Notes

- **Read-only subscription-driven**: on startup the TUI creates a `subscription` for the current Agent and uses peek/ack to pull event increments that refresh each panel; no full polling.
- **Log stream**: after the spawn subprocess's stdout is parsed into lines by the spawn layer, it is routed by sessionId to the corresponding task bar's log area (following codex-plugin-cc State Manager's log tracking).
- **Crash recovery**: TUI exit/crash does not affect the fact layer; on restart, `handoff_snapshot` + `context_sync` rebuild each bar's view.
- **Interaction control**: slash commands are translated into `vibebus` CLI invocations (in-process or as subprocesses), preserving the rule that "the TUI holds no business state".
- **Density**: medium (compact) density suited to high-data terminal environments; status colors: working=blue, claimed=cyan, ready=gray, blocked=orange, failed=red, completed=green.

## 7. Relationship with VibeBus's Existing Capabilities

| Layer | Reused | New |
| --- | --- | --- |
| ④ Fact bus | Everything (agents/tasks/messages/decisions/artifacts/reservations/subscriptions/handoffs/thread_bindings) | None |
| ③ Spawn layer | thread_binding(`spawn:{id}`), message, handoff, task | `vibebus agent spawn`/`reply`/`kill`/`status` subcommands + MCP tools; `.vibebus/agents.json` profiles |
| ② Orchestrator | task_dependencies, send, handoff_send, subscription | Multi-panel task-bar scheduling logic, dispatch decision tree, two-strike help rule (SKILL) |
| ① TUI | status, context_sync, inbox, events, subscription_peek | `vibebus tui` (ratatui) |

**Core conclusion**: the "communication mechanisms" of cross-Agent collaboration reuse VibeBus's existing fact primitives almost entirely (message/handoff/task/subscription); what is genuinely new is the "spawn layer" (bidirectional spawn channel) and the "TUI layer" (unified visualization). This keeps all existing invariants (#1–#40) intact; the spawn layer and the injection/registration capabilities are expressed as new invariants (#41–#44). New capabilities are additive, not replacements.

## 8. Implementation Roadmap

| Phase | Deliverable | Depends on |
| --- | --- | --- |
| P1 · Spawn-layer MVP | `vibebus_agent_spawn`/`reply`/`kill`/`status` (single agent, sync bidirectional channel) + thread_binding integration + `.vibebus/agents.json` | Layer ④ ready |
| P2 · Parallelism and roles | `spawn_batch`, role-profile injection, two-strike help rule in SKILL | P1 |
| P3 · TUI skeleton | `vibebus tui`: three-panel Agents/Tasks/Messages layout + subscription-driven refresh + slash commands | P1 |
| P4 · Task bars and log stream | Multi-task-bar switching, `context_sync` resumption, spawn log routing | P2/P3 |
| P5 · Pipeline orchestration | Multi-stage spawn, context chaining, handoff automation | P2 |

Each phase can be verified in the `glm/trial` worktree: Rust compiles, the existing 30 tests do not regress, and the new spawn/TUI unit tests pass.

## 9. Risks and Boundaries

- **Spawn subprocess stability**: depends on the login state and stability of external Agent CLIs (codex/claude/gemini), a known dependency of codex-plugin-cc as well. The spawn layer must mark subprocess crashes as `error` and write the facts back.
- **Context bloat**: spawn replies must be summarized (pal pattern); returning full logs/transcripts is strictly forbidden — consistent with VibeBus's non-goal of "not sharing full transcripts".
- **TUI cross-platform**: ratatui + crossterm ANSI support must be verified on Windows terminals; the operator interactive terminal requirements carry over.
- **Security boundary**: spawned subagents inherit the local user's permissions, so the SKILL must state "only spawn trusted agent CLIs", consistent with VibeBus's existing boundary of "not defending against malicious same-user processes".
