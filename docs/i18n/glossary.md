# VibeBus 术语表 / Glossary

> This glossary defines the canonical Simplified Chinese translations for VibeBus technical terms. Every translation under `docs/zh-CN/` MUST use these terms. Terms not listed here are translated by their plain meaning and noted in the translation-matrix comments.
>
> 本表定义 VibeBus 技术术语的标准简体中文译法。`docs/zh-CN/` 下的所有译文必须使用本表译法；未列出的术语按字面翻译并在翻译对照表中注明。

## 核心概念 / Core concepts

| English | 简体中文 | Notes / 说明 |
| --- | --- | --- |
| Agent | 智能体（Agent） | 保持英文原文，首次出现加括号 |
| Operator | 操作员（Operator） | 指 CLI-only 的维护者能力 |
| Bearer token | 持有者令牌（bearer token） | 或「访问令牌」 |
| Recovery key | 恢复密钥（recovery key） | 一次性恢复凭据 |
| Task | 任务（task） | |
| Message | 消息（message） | |
| Directed message | 定向消息 | |
| Inbox | 收件箱（inbox） | |
| Acknowledgement / ACK | 确认（ACK） | |
| Reservation | 路径预留（reservation） | 文件路径占用声明 |
| Artifact | 制品（artifact） | 发布物 |
| Decision | 决策（decision） | 不可变已确认事实 |
| Handoff | 交接（handoff） | 结构化交接 |
| Subscription | 订阅（subscription） | |
| Event | 事件（event） | |
| Retention | 留存（retention） | 数据保留清理 |
| Compaction | 压缩（compaction） | 离线物理压缩 |
| Audit event / Audit trail | 审计事件 / 审计轨迹 | |
| Fact bus | 事实总线 | |
| Project | 项目（project） | |
| Credential vault | 凭据保险库（vault） | OS Keychain / CredMgr |
| Worktree | 工作树（worktree） | |

## 状态与操作 / States and operations

| English | 简体中文 | Notes / 说明 |
| --- | --- | --- |
| pending / ready / claimed / working / review / blocked / completed / abandoned | 待处理 / 就绪 / 已认领 / 进行中 / 评审中 / 已阻塞 / 已完成 / 已放弃 | 任务状态机 |
| terminal state | 终态 | |
| idempotency | 幂等性 | |
| replay-safe | 可重放（replay-safe） | 至少一次交付 |
| at-least-once | 至少一次 | |
| exactly-once | 恰好一次 | |
| payload drift | 载荷漂移 | |
| semantic key | 语义键 | |
| optimistic version | 乐观版本号 | |
| fail closed | 失败即关闭（fail-closed） | 或「默认拒绝」 |
| fail fast | 快速失败（fail-fast） | |
| redacted | 已脱敏（redacted） | |
| override | 覆盖（override） | 职责越界授权 |

## 平台与工具 / Platforms and tooling

| English | 简体中文 | Notes / 说明 |
| --- | --- | --- |
| Windows Credential Manager | Windows 凭据管理器 | |
| macOS Keychain | macOS 钥匙串（Keychain） | |
| SQLite WAL | SQLite WAL | 保持原文 |
| MCP | MCP | 保持原文 |
| Hook | Hook | 保持原文（生命周期钩子） |
| SessionStart / PostToolUse / Stop | SessionStart / PostToolUse / Stop | 保持原文 |
| CLI | CLI | 命令行接口 |
| MSI | MSI | 保持原文 |
| CycloneDX SBOM | CycloneDX SBOM | 保持原文 |
| SignTool | SignTool | 保持原文 |
| WiX | WiX | 保持原文 |
| CI | CI | 持续集成 |
| GitHub Releases | GitHub Releases | 保持原文 |

## 命令族 / Command families

| English | 简体中文 | Notes / 说明 |
| --- | --- | --- |
| `vibebus init` | `vibebus init` | 命令名不翻译 |
| `vibebus register` | `vibebus register` | 命令名不翻译 |
| `context sync` | `context sync` | 命令名不翻译 |
| `handoff propose` | `handoff propose` | 命令名不翻译 |
| `maintenance compact` | `maintenance compact` | 命令名不翻译 |
| API provider | API 提供方（provider） | |
| Base URL | Base URL | 保持原文 |
| api format | api format | 保持原文 |
| env export | 环境变量导出 | |

## 发布与签名术语 / Release & signing terms

| English | 简体中文 | Notes / 说明 |
| --- | --- | --- |
| source-only | 纯源码（source-only） | 仅源码发布 |
| corrective patch release | 修正性补丁版本 | |
| release candidate | 发布候选 | |
| prerelease contract | 预发布契约 | |
| provenance | 来源证明（provenance） | 供应链证据 |
| digest | 摘要（digest） | 镜像/哈希摘要 |
| notarization | 公证（notarization） | Apple 公证 |
| stapling | 公证钉扎（stapling） | Gatekeeper 票钉扎 |
| ad-hoc signature | 临时（ad-hoc）签名 | 本地完整性签名 |
| Developer ID Application | Developer ID Application | 保持原文 |
| hardened runtime | 加固运行时 | |
| Gatekeeper | Gatekeeper | 保持原文 |
| ICE validation | ICE 校验 | MSI 内部一致性评估 |
| administrative image | 管理映像 | MSI 术语 |

## 平台与测试术语 / Platform & test terms

| English | 简体中文 | Notes / 说明 |
| --- | --- | --- |
| Keychain fixture | 钥匙串测试夹具 | |
| real pseudo-terminal | 真实伪终端 | |
| disposable project | 一次性项目 | |
| data home | 数据主目录 | `--data-home` 指向的目录 |
| cutover | 切换（cutover） | 破坏性恢复切换 |
| preflight | 预检（preflight） | |
| fixture | 测试夹具（fixture） | |
| audit evidence | 审计证据 | |
| recovery inventory | 恢复清单 | |

## 翻译约定 / Translation conventions

1. **命令、工具名、API 名、表名、文件路径永不翻译**（如 `vibebus_agent_spawn`、`providers` 表、`responsibility.json`）。
2. **代码块、表格、链接必须与英文原文逐字一致**——只翻译表格外的叙述文字和表格里的说明列（Notes），代码块与命令字段不翻译。
3. **首次出现核心术语时用「中文（English）」格式**，后续单独用中文或英文均可，但全文保持一致。
4. **标题层级、锚点、文件结构必须与英文原文完全一致**，保证两种语言切换时锚点跳转可用。
5. 英文文档更新后，中文译文必须在同一 PR 中同步更新（见 [maintenance.md](maintenance.md)）。
