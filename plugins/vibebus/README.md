# VibeBus Codex 插件

VibeBus 是独立 Codex 顶层任务之间的结构化本地总线：每个任务继续拥有自己的聊天上下文和 worktree，只通过明确登记的事实协作。插件把协调 Skill、stdio MCP、Windows 生命周期 Hooks、品牌资产和可安装的本地 marketplace 打包在一起。

![VibeBus logo](assets/vibebus-logo-light.png)

## 你会得到什么

- 任务、依赖和原子领取，避免两个任务同时声称同一件事。
- 定向消息、read/ACK/close 和结构化 handoff，边界清晰、可恢复。
- Codex task/thread 绑定、任务作用域路径 reservation、责任域与限时 override。
- Windows 当前用户凭据库、token 摘要存储、恢复密钥轮换和脱敏响应。
- SQLite WAL、可重放 subscription peek/ACK、不可变 Git/test 事实和 review-only Stop 提案。

共享的是事实、决定、依赖和产物引用，不是完整对话或隐藏推理。VibeBus 不会远程同步多台机器，也不会强制中断正在生成的模型、自动合并 Git 或声称已完成签名生产发布。

## 5 分钟上手（Windows）

在项目根目录准备已构建的 `vibebus.exe`，或先运行仓库根目录的发布/打包脚本，然后：

```powershell
codex plugin marketplace add .
codex plugin add vibebus@vibebus-local
```

安装/更新后启动一个新 Codex 任务，并在预期项目根目录显式初始化：

```powershell
vibebus.exe init --root D:\path\to\repo --name "My Project"
vibebus.exe register --root D:\path\to\repo --name ui --role frontend --store-credentials
vibebus.exe credential status --root D:\path\to\repo --agent ui
```

然后在任务中执行：

```text
注册本任务并检查 Inbox；领取对应 VibeBus task；检查责任策略；为要编辑的精确路径建立 reservation；完成后发送结构化 handoff。
```

首次安装或 Hook 定义变更后，必须由用户在 Codex 中审查并信任 Hooks。SessionStart 只向上寻找 `.vibebus/project.json`；PostToolUse 只记录有界 Git/test 事实；Stop 只生成待审阅提案，不自动发送消息。

## Linux / 容器路径

镜像是非 root、命令式的 CLI/std io MCP 交付，不是 HTTP 服务：

```powershell
./scripts/test-container.ps1 -ImageTag vibebus:0.10.0-local
docker run --rm `
  --mount type=bind,source=D:\path\to\repo,target=/workspace `
  --mount type=volume,source=vibebus-data,target=/data `
  vibebus:0.10.0-local doctor --root /workspace
```

容器内使用 `/workspace` 项目挂载和 `/data` SQLite volume，运行用户为 `10001:10001`。Linux 没有 Windows Credential Manager，不能使用 `--store-credentials`；请通过短时显式 token 或 `VIBEBUS_AGENT_TOKEN` 注入凭据，且不要把 token、恢复密钥、operator、云或签名凭据写入镜像、参数记录、仓库或报告。

阿里云 ACR 的登录、推送和 digest 验证见 [容器交付](../../docs/container.md)。

## 安全模型

- Agent token 和 recovery key 只在注册/恢复/轮换边界出现；SQLite 仅保存摘要，成功写入 Windows 凭据库后响应脱敏。
- Inbox 必须使用收件人身份认证；reservation、task ownership、责任域和限时 override 是相互独立的约束。
- operator 初始化、轮换、恢复、删除和 retention 批准只允许真实交互式 CLI；MCP 不提供这些 operator mutation。
- subscription peek/ACK 是至少一次交付；副作用必须幂等。ACK 后再 close 需要 ACK 的消息。
- 任务完成后自动结束活动 thread binding；插件不创建、打开、唤醒或控制原生 Codex 任务。

## 开发与验收入口

从仓库根目录运行：

```powershell
./scripts/validate-plugin.ps1 -PluginRoot ./plugins/vibebus
./scripts/package-plugin.ps1
./scripts/build-release.ps1
$msi = Get-ChildItem ./dist/VibeBus-*-windows-x64.msi | Select-Object -First 1
./scripts/test-installer.ps1 -MsiPath $msi.FullName
./scripts/test-container.ps1 -ImageTag vibebus:0.10.0-local
```

生产 tag 发布还需要代码签名证书和密码；缺少它们时发布路径应失败关闭。普通 PR 的本地包明确是未签名验收包，不是生产发布。

项目级入口：

- [根 README](../../README.md)
- [架构](../../docs/architecture.md)
- [CLI 与 MCP 协议](../../docs/protocol.md)
- [验收记录](../../docs/acceptance.md)
- [发布工程](../../docs/release.md)
- [容器交付](../../docs/container.md)
- [协调 Skill](skills/vibebus-coordination/SKILL.md)

## 品牌资产

标志以原创的三节点总线几何图形表达“独立任务 + 显式路由 + 本地事实层”，不使用第三方视觉。manifest 通过官方支持的 `interface.composerIcon` 与 `interface.logo` 引用透明 PNG；明暗变体同时保留在 `assets/` 供安装面和文档预览使用。

![VibeBus icon on dark surface](assets/vibebus-icon-dark.png)
