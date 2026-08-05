# VibeBus

[English](../en/README.md) · **简体中文** · [文档中心](../README.md)

![VibeBus logo](../../plugins/vibebus/assets/vibebus-logo-light.png)

VibeBus 是一个面向独立编码智能体（Agent）任务的**本地结构化事实总线**。每个任务保留自己的对话上下文与工作树（worktree），只通过项目级 SQLite 数据库共享显式事实——消息、确认（ACK）、任务状态、决策、路径预留、制品，以及有界的 Git/测试结果。

无守护进程、无云端依赖、不共享对话记录。一个轻量 Rust 二进制、每项目一个数据库、58 个可直接调用的 MCP 工具。

## 为什么需要 VibeBus

编码智能体能力强大但彼此隔离：每个 Codex、Claude Code、Cursor、OpenCode 或 Antigravity 任务独占自身上下文，要协调它们要么复制完整对话（上下文污染、隐私损失），要么发明临时文件格式（无原子性、无审计轨迹）。

VibeBus 位于它们之间，充当**可持久化的协调事实源**：

- **原子任务认领** — 竞争同一任务时恰好一个智能体胜出。
- **带 ACK/close 的定向收件箱** — 事实只发送给指定收件人，带回执。
- **路径预留** — 智能体声明将要修改的文件，重叠即冲突。
- **可重放的事件订阅** — 至少一次交付，配合幂等 ACK。
- **操作员批准的破坏性维护** — 留存清理与压缩必须由人类在真实终端操作。

## 功能特性

- **Rust CLI + stdio MCP 服务**（`vibebus mcp`，58 个 `vibebus_*` 工具）—— 单一二进制、单一领域模型。
- **按工具适配** Codex、Claude Code、Cursor、OpenCode、Antigravity——同一总线、同一技能（Skill）、按智能体接线。参见 [多工具适配器](multi-tool-adapters.md)。
- **OS 凭据保险库** — Windows 凭据管理器与 macOS 钥匙串；SQLite 仅存储摘要。
- **严格责任策略** — `.vibebus/responsibility.json` 将角色映射到经校验的项目相对路径，支持任务级限时覆盖（override）。
- **不可变事实** — 已确认决策、Git 提交事实、测试结果事实精确重放，漂移即冲突。
- **智能体派生桥（spawn bridge）** — 将 `codex`/`claude`/`gemini`/自定义 CLI 派生为可追踪会话，支持双向回复。
- **API 提供方注入** — 项目级 provider 元数据存于 `.vibebus/api.json`，密钥存于 OS 保险库，注入派生进程。默认零注入。
- **能力注册表** — `providers`、`skills`、`plugins` 三表，实现跨智能体能力复用。
- **有界上下文同步** — 按任务范围提供确定性、预算化、分页投影。
- **生命周期 Hook** — SessionStart / PostToolUse / Stop 在 Windows 与 macOS 上自动捕获有界 Git/测试事实。
- **原生 Windows、macOS 与 Linux** — Windows/macOS 原生支持；Linux 通过 `amd64` 容器工作流。

## 快速上手

### 1. 安装

从 Release 下载源码归档并按 [发布工程](release.md) 校验，用 Rust 1.97.1 构建；或直接安装本仓库的 Codex 插件：

```powershell
# Windows
./scripts/package-plugin.ps1
codex plugin marketplace add .
codex plugin add vibebus@vibebus-local
```

```sh
# macOS (Apple Silicon)
./scripts/package-plugin-macos.sh
codex plugin marketplace add "$PWD/dist/staging/VibeBus-0.11.0-macos-arm64"
codex plugin add vibebus@vibebus-local
```

### 2. 初始化项目

```sh
vibebus init --root /path/to/project --name "My Project"
vibebus register --root /path/to/project --name implementation --role implementation --store-credentials
vibebus credential status --root /path/to/project --agent implementation
```

`register` 会以明文返回一次持有者令牌与一次性恢复密钥。使用 `--store-credentials` 时二者写入 OS 保险库，响应被脱敏（`secretsRedacted=true`）。

### 3. 协调任务

每个智能体任务加载 `vibebus-coordination` 技能后：

1. `vibebus_status` / `vibebus_handoff_snapshot` — 查看项目状态与待处理交接。
2. `vibebus_task_claim` 认领一个就绪任务；认领冲突即证明其他任务胜出。
3. 编辑前用 `vibebus_reserve` 预留精确的项目相对路径。
4. 对必需消息执行 `vibebus_inbox` → 处理 → `vibebus_ack` → `vibebus_close`。
5. 用 `vibebus_context_sync` 获取确定性的任务级投影。
6. 交接时用 `vibebus_handoff_send` 发送摘要、决策与制品引用。

## 核心概念

| 概念 | 含义 |
| --- | --- |
| **智能体（Agent）** | 带持有者/恢复凭据的命名身份，限定于一个项目。 |
| **任务（Task）** | 原子工作单元：`pending → ready → claimed → working → ... → completed/abandoned`。 |
| **消息（Message）** | 发送给指定收件人的定向结构化事实，带 read/ACK/close 回执。 |
| **路径预留（Reservation）** | 对项目相对路径的 TTL 限定意图；不同智能体间重叠即冲突。 |
| **制品（Artifact）** | 已发布的文件引用，含 SHA-256、类型、摘要与 JSON 元数据。 |
| **决策（Decision）** | 带稳定语义键的不可变已确认事实。 |
| **订阅（Subscription）** | 智能体拥有的事件游标，可重放 peek/ACK 交付。 |
| **交接（Handoff）** | 高优先级、必须 ACK 的转移，含摘要、决策、制品与后续动作。 |
| **操作员（Operator）** | 仅限人类的能力，用于留存批准与离线压缩。 |

## CLI 与 MCP 接口

一个二进制、两个接口，共享同一领域模型。CLI 响应为 JSON；MCP 工具以文本内容返回格式化 JSON。完整能力表（身份、消息、任务、线程、预留、制品、决策、事实、上下文、事件、订阅、留存、压缩、交接、派生、API 注入、能力注册表）见 [CLI 与 MCP 协议](protocol.md)。

```sh
vibebus init | register | agents | agent recover | credential status
vibebus send | inbox | read | ack | close
vibebus task create|claim|update|complete|show|list
vibebus thread bind|unbind|list
vibebus reserve add|renew|release|list
vibebus artifact publish|list
vibebus decision confirm | context sync
vibebus fact git-commit | fact test-result
vibebus event list | subscription create|list|poll|peek|ack
vibebus retention plan|apply|status
vibebus maintenance compact --backup <path>
vibebus handoff send|snapshot|propose
vibebus agent spawn | api provider|use|unuse|restore|health|run|key
vibebus skill register|list|install | plugin list
vibebus status | doctor | backup | mcp
```

从插件启动时，MCP 工具每次调用都必须传入绝对项目根目录作为 `root`。工具名清单见 [CLI 与 MCP 协议](protocol.md)。

## 仓库结构

```text
src/              Rust CLI + MCP 服务（store、mcp、spawn、inject、hooks、vault、policy 等）
plugins/          按工具适配包（vibebus/Codex、claude-code、cursor、opencode、antigravity）
scripts/          打包、校验与验收脚本（PowerShell + shell + Python）
docs/             架构、协议、发布、平台与运维文档（docs/en 英文 / docs/zh-CN 中文）
tests/            集成测试套件（CLI 工作流、核心工作流、MCP 协议、保险库）
```

## 版本发布

GitHub Releases 是唯一官方分发渠道。稳定版是提交可从 `main` 到达的 `vX.Y.Z` 标签；普通分支、PR 产物与本地构建均不算发布。版本号采用语义化版本（SemVer）：

- patch 版本修复兼容性行为；
- minor 版本新增向后兼容能力；
- major 版本可能引入不兼容变更并附带迁移指南。

当前稳定发布为纯源码形式。GitHub 自动提供标签对应的 Source code ZIP 与 tarball；工作流仅上传 `SHA256SUMS.txt`、规范化 CycloneDX SBOM、`supply-chain-evidence.json` 与 `source-release-manifest.json` 四个证据文件。校验文件覆盖其他三个已上传证据文件。首个发布不含 MSI、便携 ZIP、Codex 插件 ZIP、签名可执行文件或安装器资产。在采用独立的、经评审的二进制分发契约之前，macOS、Windows 与 Linux 仍保持源码与 CI 支持路径。当前不发布候选版本：`vX.Y.Z-rc.N` 这类标签会失败关闭，而不会被当作稳定发布。

校验下载的稳定发布：从同一 GitHub Release 下载 GitHub 生成的源码归档与四个证据文件，核对 manifest 的 tag 与 source revision 指向该源码归档，再用 `SHA256SUMS.txt` 校验其列出的三个证据文件。该纯源码发布不含安装、签名、升级或二进制回滚承诺。

维护者与校验者详细说明见 [发布工程](release.md)。

## 开发

仓库固定 Rust `1.97.1`。按所改动文件运行对应检查：

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
python3 scripts/normalize-cyclonedx.py --self-test
git diff --check
```

Windows 打包与 MSI 校验需要 Windows 工具链：

```powershell
./scripts/build-release.ps1
$msi = Get-ChildItem ./dist/VibeBus-*-windows-x64.msi | Select-Object -First 1
./scripts/test-installer.ps1 -MsiPath $msi.FullName
```

贡献规范见 [CONTRIBUTING.md](../../CONTRIBUTING.md)，私有漏洞上报见 [SECURITY.md](../../SECURITY.md)，社区准则见 [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md)。

## 安全与运行边界

- 智能体凭据属于 Windows 凭据管理器或 macOS 钥匙串；切勿提交令牌、恢复密钥、签名材料、数据库或生成的发布资产。
- 操作员生命周期与破坏性维护需要真实交互式终端；这些操作刻意不通过 MCP 提供。
- VibeBus 记录持久事实，但不会唤醒、中断、合并或以其他方式控制原生 Codex 任务。
- 可重放订阅为至少一次交付；消费方必须保证副作用幂等。
- API 提供方密钥只存于 OS 保险库；`api.json`、日志与 MCP 响应只携带变量名与存在性，绝不携带密钥值。

## 文档

- [架构](architecture.md)
- [CLI 与 MCP 协议](protocol.md)
- [多工具适配器](multi-tool-adapters.md)
- [设计研究与决策](design-research.md)
- [备份、导出与恢复](backup-restore.md)
- [发布工程](release.md)
- [macOS 开发](macos.md)
- [容器开发](container.md)
- [维护者交接](HANDOFF.md)
- [变更日志](CHANGELOG.md)
- [贡献指南](CONTRIBUTING.md)
- [安全策略](SECURITY.md)
- [行为准则](CODE_OF_CONDUCT.md)

VibeBus 以 [MIT License](../../LICENSE) 发布。
