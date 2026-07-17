# VibeBus

VibeBus 是一个面向独立 Codex 顶层任务的本地结构化事实总线。它保留 Codex 原生的任务与 worktree 隔离，只共享明确登记的消息、ACK、任务状态、依赖、文件租约和产物，不共享整段聊天上下文。

当前版本 0.8 是可运行的 Windows MVP：一个 Rust 单文件程序同时提供 CLI 和 stdio MCP，状态写入项目级 SQLite WAL 数据库，Agent 与 operator 秘密可写入 Windows 当前用户凭据管理器，并通过 Windows CI、当前用户级 MSI、便携包及可选本地 Authenticode 签名脚本交付为 Codex 插件。正式 tag 发布缺少签名凭据时会直接失败。

## 已实现

- 项目身份：仓库内 `.vibebus/project.json`，数据默认位于 `%LOCALAPPDATA%\VibeBus\projects\<project-id>\vibebus.db`。
- Agent 注册、单次恢复密钥、bearer token 轮换与哈希存储。
- 可选 Windows 当前用户凭据库存储、成功写入后的秘密脱敏、CLI/MCP 无明文 token 回退、状态检查和显式删除。
- 独立 operator 凭据、仅限真实终端的初始化/轮换/恢复，以及与精确 plan 和当前凭据代次绑定的短时单次 retention 批准。
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
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/build-release.ps1
$msi = Get-ChildItem ./dist/VibeBus-*-windows-x64.msi | Select-Object -First 1
./scripts/test-installer.ps1 -MsiPath $msi.FullName
```

发布脚本会生成 `plugins\vibebus\bin\vibebus.exe`、当前用户级 MSI、便携 marketplace 包、独立插件包、SHA-256 校验和与发布清单。插件的 `.mcp.json` 从打包后的二进制路径启动 stdio MCP 服务。单独更新插件二进制仍可运行 `powershell -File .\scripts\package-plugin.ps1`。

## 初始化项目

```powershell
.\target\release\vibebus.exe init --root D:\path\to\repo --name "My Project"
.\target\release\vibebus.exe doctor --root D:\path\to\repo
```

初始化必须由用户在预期根目录显式执行；插件不会偷偷创建项目。

## 最小 CLI 流程

```powershell
$registration = .\target\release\vibebus.exe register --root D:\path\to\repo --name api --role backend --store-credentials | ConvertFrom-Json
# 成功时 token 与 recoveryKey 不会出现在 JSON 中；后续命令按项目与 Agent 自动读取当前用户凭据库。
.\target\release\vibebus.exe credential status --root D:\path\to\repo --agent api

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
# 首次使用时，由本地维护者在真实交互式终端中执行 operator init，并按提示确认项目 ID。
.\target\release\vibebus.exe operator init --root D:\path\to\repo
# 每次执行清理前，维护者审阅候选并在真实终端中批准精确 planId；默认批准 10 分钟内有效。
.\target\release\vibebus.exe operator approve-retention --root D:\path\to\repo --plan $plan.result.planId
.\target\release\vibebus.exe retention apply --root D:\path\to\repo --agent api --plan $plan.result.planId
```

处理完一条消息后使用 `ack`（若发送方要求）和 `close`；普通 `inbox` 不返回已关闭消息，审计时可加 `--all --include-closed`。线程绑定只是把 VibeBus 任务与调用方提供的 Codex 任务 ID 建立持久关联，不会创建、打开、唤醒或控制 Codex 任务。

保留清理采用三道门槛：Agent 认证的 `retention plan` 只读返回候选计数和 `planId`；本地维护者通过 CLI `operator approve-retention` 审阅并交互式批准同一 plan；Agent 的 `retention apply` 才能原子消费该批准。operator mutation 不暴露为 MCP 工具，并拒绝重定向输入。期间若总线状态变化，旧计划会冲突；批准过期或 operator 轮换后必须重新批准。成功执行的重试返回原报告，不再要求第二次批准。自定义策略的所有参数必须在 plan、approve 和 apply 三步完全一致。

CLI 总是输出 JSON。认证优先级是显式 `--token`、`VIBEBUS_AGENT_TOKEN`、当前用户凭据库；旧调用方式保持兼容。`register --store-credentials`、`agent recover --store-credentials` 和 `agent provision-recovery --store-credentials` 成功写入后会从响应中移除秘密。若旋转身份后凭据库写入失败，VibeBus 会返回唯一仍可用的明文秘密并附带 `credentialStorageError`，调用者必须立即安全保存或重试写入，避免身份永久丢失。`credential delete` 只删除操作系统条目，不删除 Agent 或其总线状态。

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

MSI 安装到 `%LOCALAPPDATA%\Programs\VibeBus`，并将其中的 `plugins\vibebus\bin` 写入当前用户 PATH。安装器不会通过自定义操作修改 Codex 配置；安装后仍需对该目录显式执行 marketplace add 与 plugin add。详见[发布工程](docs/release.md)。

## 文档

- [架构](docs/architecture.md)
- [CLI 与 MCP 协议](docs/protocol.md)
- [方案对比与取舍](docs/design-research.md)
- [验收记录](docs/acceptance.md)
- [发布工程](docs/release.md)
- [后续接手](docs/HANDOFF.md)

## 安全边界

- token 与恢复密钥仅在注册、恢复或轮换时返回明文，数据库只保留 SHA-256 摘要；成功恢复会同时撤销旧 token 与旧恢复密钥。
- `--store-credentials` 使用 Windows Credential Manager 的 Generic Credential，目标名为 `VibeBus:<project-id>:<agent>`，凭据 BLOB 不进入仓库、SQLite、事件或消息。
- operator 秘密使用独立目标 `VibeBusOperator:<project-id>`；数据库只保存摘要与代次。operator 初始化、轮换、凭据恢复和批准只允许真实交互式 CLI，不向 MCP 暴露。
- `CRED_PERSIST_LOCAL_MACHINE` 让条目对同一 Windows 用户的后续本机登录会话可用；同一用户上下文中的其他进程仍属于信任边界，VibeBus 角色不是操作系统级秘密隔离。
- 收件箱必须使用收件人身份认证，不能读取其他 Agent 的消息。
- 任务更新要求当前所有者和最新版本。
- 每个任务同时最多有一个活动线程绑定，绑定/解绑只允许任务所有者；任务终态自动结束活动绑定。
- 租约路径必须是无 `..`、无盘符、非绝对的项目相对路径。
- 产物路径 canonicalize 后必须仍在项目根目录内。
- 备份拒绝覆盖已有目标文件。
- 幂等键按项目、Agent 与操作域隔离；同键重试必须使用完全相同的有效载荷。
- 清理执行需要已认证 Agent、未过时的确认计划和当前 operator 代次签发的未过期单次批准；批准消费与删除处于同一事务，执行重试返回原报告。
- 生产 tag 发布必须同时具备 PFX Base64 与密码 Secret；普通 PR 只产出明确标记为未签名的验收包。

许可证：MIT。
