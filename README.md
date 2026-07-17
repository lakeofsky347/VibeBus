# VibeBus

VibeBus 是一个面向独立 Codex 顶层任务的本地结构化事实总线。它保留 Codex 原生的任务与 worktree 隔离，只共享明确登记的消息、ACK、任务状态、依赖、文件租约和产物，不共享整段聊天上下文。

当前版本 0.5 是可运行的 Windows MVP：一个 Rust 单文件程序同时提供 CLI 和 stdio MCP，状态写入项目级 SQLite WAL 数据库，并打包为 Codex 插件。

## 已实现

- 项目身份：仓库内 `.vibebus/project.json`，数据默认位于 `%LOCALAPPDATA%\VibeBus\projects\<project-id>\vibebus.db`。
- Agent 注册、单次恢复密钥、bearer token 轮换与哈希存储。
- 定向消息、未读收件箱、read/ACK/close 回执隔离；需 ACK 的消息必须先确认再关闭。
- 带依赖的任务、原子领取、所有者约束、状态机和乐观版本冲突。
- 任务所有者可将活动任务绑定到一个 Codex 任务 ID；任务完成或放弃时自动解绑并保留历史。
- 项目相对路径租约、重叠检测、TTL、所有者续期和显式释放。
- 产物登记、项目边界验证和 SHA-256 校验。
- 可重试写操作的幂等键与“同键异载荷”冲突语义。
- 有序事件流、按类型过滤、命名订阅、兼容型 consume-on-poll，以及可重放的 peek/ack 持久交付。
- 高优先级结构化 handoff、强制 ACK 意图与恢复快照。
- 先预览再确认的有界保留：保护最慢订阅游标与 pending delivery，并清理过期事件前缀、幂等记录、已关闭消息和终态线程历史。
- SQLite 健康检查和一致性在线备份。
- CLI、MCP、Codex Skill、SessionStart Hook 和本地 marketplace。

VibeBus 不承诺中断或唤醒正在生成中的模型。收件箱检查发生在任务启动、恢复、关键决策和交接等安全边界。

## 构建与验证

需要 Rust 2024 edition 兼容工具链。

```powershell
cargo test --all-targets
powershell -File .\scripts\package-plugin.ps1
```

第二条命令生成 `plugins\vibebus\bin\vibebus.exe`，插件的 `.mcp.json` 会从该路径启动 stdio MCP 服务。

## 初始化项目

```powershell
.\target\release\vibebus.exe init --root D:\path\to\repo --name "My Project"
.\target\release\vibebus.exe doctor --root D:\path\to\repo
```

初始化必须由用户在预期根目录显式执行；插件不会偷偷创建项目。

## 最小 CLI 流程

```powershell
$registration = .\target\release\vibebus.exe register --root D:\path\to\repo --name api --role backend | ConvertFrom-Json
$env:VIBEBUS_AGENT_TOKEN = $registration.result.token
# 将 $registration.result.recoveryKey 存入安全的任务私有凭据区；不要写入仓库或消息。

.\target\release\vibebus.exe task create --root D:\path\to\repo --agent api --id TASK-101 --title "Implement API"
.\target\release\vibebus.exe task claim --root D:\path\to\repo --agent api --id TASK-101
.\target\release\vibebus.exe thread bind --root D:\path\to\repo --agent api --task TASK-101 --thread 019f-example-codex-task
.\target\release\vibebus.exe reserve add --root D:\path\to\repo --agent api --path src/api --reason "TASK-101"
.\target\release\vibebus.exe inbox --root D:\path\to\repo --agent api
.\target\release\vibebus.exe subscription create --root D:\path\to\repo --agent api --name coordination --event-types message_sent,task_updated --from-sequence 0
$delivery = .\target\release\vibebus.exe subscription peek --root D:\path\to\repo --agent api --name coordination | ConvertFrom-Json
.\target\release\vibebus.exe subscription ack --root D:\path\to\repo --agent api --name coordination --delivery $delivery.result.delivery.deliveryId
.\target\release\vibebus.exe handoff snapshot --root D:\path\to\repo --agent api
$plan = .\target\release\vibebus.exe retention plan --root D:\path\to\repo --agent api | ConvertFrom-Json
.\target\release\vibebus.exe retention apply --root D:\path\to\repo --agent api --plan $plan.result.planId
```

处理完一条消息后使用 `ack`（若发送方要求）和 `close`；普通 `inbox` 不返回已关闭消息，审计时可加 `--all --include-closed`。线程绑定只是把 VibeBus 任务与调用方提供的 Codex 任务 ID 建立持久关联，不会创建、打开、唤醒或控制 Codex 任务。

保留清理永远分两步：`retention plan` 只读返回候选计数和 `planId`，`retention apply` 必须带回同一策略和该 ID。期间若总线状态变化，旧计划会冲突并要求重新预览。执行后，早于 `retention status` 所示历史下界的事件游标会被拒绝；订阅未读区间和 pending delivery 不会被删除。自定义策略时，关闭消息保留天数不得短于幂等记录保留天数。

CLI 总是输出 JSON。也可以逐条传入 `--token`，避免设置进程环境变量。

在 Windows 上登记复杂产物 metadata 时，优先把 JSON 写入文件并使用 `artifact publish --metadata-file <path>`，避免 PowerShell 改写 JSON 引号；MCP 可直接传原生 JSON 对象。

## 安装本地 Codex 插件

仓库已提供 `.agents/plugins/marketplace.json`。构建插件后二选一：

1. 在 Codex 桌面应用的 Plugins 中打开这个本地 marketplace；或
2. 使用 CLI 注册 marketplace 根目录，然后安装插件。

```powershell
codex plugin marketplace add D:\MyProjects\CoWork
codex plugin add vibebus@vibebus-local
```

安装或更新后启动一个新任务，使 Skill、MCP 和 Hook 重新加载。SessionStart Hook 首次使用前需要在 Codex 中审查并信任；它只向上寻找并读取 `.vibebus/project.json`。

## 文档

- [架构](docs/architecture.md)
- [CLI 与 MCP 协议](docs/protocol.md)
- [方案对比与取舍](docs/design-research.md)
- [验收记录](docs/acceptance.md)
- [后续接手](docs/HANDOFF.md)

## 安全边界

- token 与恢复密钥仅在注册、恢复或轮换时返回明文，数据库只保留 SHA-256 摘要；成功恢复会同时撤销旧 token 与旧恢复密钥。
- 收件箱必须使用收件人身份认证，不能读取其他 Agent 的消息。
- 任务更新要求当前所有者和最新版本。
- 每个任务同时最多有一个活动线程绑定，绑定/解绑只允许任务所有者；任务终态自动结束活动绑定。
- 租约路径必须是无 `..`、无盘符、非绝对的项目相对路径。
- 产物路径 canonicalize 后必须仍在项目根目录内。
- 备份拒绝覆盖已有目标文件。
- 幂等键按项目、Agent 与操作域隔离；同键重试必须使用完全相同的有效载荷。
- 清理执行需要已认证 Agent 和未过时的确认计划；执行重试返回原报告，不会重复删除。

许可证：MIT。
