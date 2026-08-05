# 翻译对照表 / Translation Matrix

> Last generated: 2026-08-05 · 对照表由维护规范驱动：英文（`docs/en/`）是规范源，简体中文（`docs/zh-CN/`）是翻译。行数差异用于快速发现结构漂移。
>
> 图例：✅ 已翻译且在同步状态 · ⚠️ 待更新（英文已变更） · ❌ 缺失翻译

## 核心文档 / Core documents

| # | 文档 | 英文源 (docs/en/) | 中文译 (docs/zh-CN/) | 英文行数 | 中文行数 | 结构一致 | 状态 |
| --- | --- | --- | --- | ---: | ---: | :---: | :---: |
| 1 | 项目概览 README | [README.md](../en/README.md) | [README.md](../zh-CN/README.md) | 182 | 186 | ✅ | ✅ |
| 2 | 架构 Architecture | [architecture.md](../en/architecture.md) | [architecture.md](../zh-CN/architecture.md) | 231 | 231 | ✅ | ✅ |
| 3 | CLI 与 MCP 协议 Protocol | [protocol.md](../en/protocol.md) | [protocol.md](../zh-CN/protocol.md) | 272 | 272 | ✅ | ✅ |
| 4 | 多工具适配器 Multi-tool adapters | [multi-tool-adapters.md](../en/multi-tool-adapters.md) | [multi-tool-adapters.md](../zh-CN/multi-tool-adapters.md) | 53 | 53 | ✅ | ✅ |
| 5 | 跨 Agent 协作设计 Cross-agent collaboration | [cross-agent-collaboration.md](../en/cross-agent-collaboration.md) | [cross-agent-collaboration.md](../zh-CN/cross-agent-collaboration.md) | 274 | 274 | ✅ | ✅ |
| 6 | 设计研究与决策 Design research | [design-research.md](../en/design-research.md) | [design-research.md](../zh-CN/design-research.md) | 66 | 66 | ✅ | ✅ |
| 7 | 备份、导出与恢复 Backup & restore | [backup-restore.md](../en/backup-restore.md) | [backup-restore.md](../zh-CN/backup-restore.md) | 49 | 49 | ✅ | ✅ |
| 8 | 发布工程 Release engineering | [release.md](../en/release.md) | [release.md](../zh-CN/release.md) | 47 | 47 | ✅ | ✅ |
| 9 | macOS 开发 macOS development | [macos.md](../en/macos.md) | [macos.md](../zh-CN/macos.md) | 93 | 93 | ✅ | ✅ |
| 10 | 容器开发 Container development | [container.md](../en/container.md) | [container.md](../zh-CN/container.md) | 31 | 31 | ✅ | ✅ |
| 11 | 维护者交接 Maintainer handoff | [HANDOFF.md](../en/HANDOFF.md) | [HANDOFF.md](../zh-CN/HANDOFF.md) | 28 | 28 | ✅ | ✅ |

## 仓库根级文档 / Root-level documents

> 英文原版按 GitHub 惯例保留在仓库根目录（`README.md`、`CHANGELOG.md`、`CONTRIBUTING.md`、`SECURITY.md`、`CODE_OF_CONDUCT.md`）。中文翻译位于 `docs/zh-CN/`；英文侧无 `docs/en/` 副本，避免双份英文源漂移。

| # | 文档 | 英文源 (根目录) | 中文译 (docs/zh-CN/) | 中文行数 | 状态 |
| --- | --- | --- | --- | ---: | :---: |
| 12 | 变更日志 Changelog | [CHANGELOG.md](../../CHANGELOG.md) | [CHANGELOG.md](../zh-CN/CHANGELOG.md) | 115 | ✅ |
| 13 | 贡献指南 Contributing | [CONTRIBUTING.md](../../CONTRIBUTING.md) | [CONTRIBUTING.md](../zh-CN/CONTRIBUTING.md) | 34 | ✅ |
| 14 | 安全策略 Security policy | [SECURITY.md](../../SECURITY.md) | [SECURITY.md](../zh-CN/SECURITY.md) | 19 | ✅ |
| 15 | 行为准则 Code of conduct | [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) | [CODE_OF_CONDUCT.md](../zh-CN/CODE_OF_CONDUCT.md) | 20 | ✅ |

## 插件文档 / Plugin documents

| # | 文档 | 英文/源 | 中文 | 状态 | 说明 |
| --- | --- | --- | --- | :---: | --- |
| 16 | Codex 插件 README（vibebus） | [plugins/vibebus/README.md](../../plugins/vibebus/README.md) | 同文件（中文为主） | ✅ | 源文件本身以中文为主，双语共存 |
| 17 | Claude Code 适配器 README | [plugins/claude-code/README.md](../../plugins/claude-code/README.md) | — | ❌ | 待翻译，见 [翻译待办](#待办事项-open-items) |
| 18 | Cursor 适配器 README | [plugins/cursor/README.md](../../plugins/cursor/README.md) | — | ❌ | 待翻译 |
| 19 | OpenCode 适配器 README | [plugins/opencode/README.md](../../plugins/opencode/README.md) | — | ❌ | 待翻译 |
| 20 | Antigravity 适配器 README | [plugins/antigravity/README.md](../../plugins/antigravity/README.md) | — | ❌ | 待翻译 |

## 运行时文件（刻意不翻译）/ Runtime files (intentionally not translated)

| 文件 | 原因 |
| --- | --- |
| `plugins/*/skills/vibebus-coordination/SKILL.md`（5 份副本） | MCP 与智能体工具按路径加载该文件；翻译会破坏插件运行契约。必须保持英文，并保持 5 份副本逐字一致。 |
| `plugins/opencode/AGENTS.md` | OpenCode 运行时指令文件，英文原文即运行契约。 |
| `plugins/cursor/.cursor/skills/vibebus-coordination/SKILL.md` | 同上，运行时文件。 |

## 待办事项 / Open items

| 优先级 | 项 | 说明 |
| --- | --- | --- |
| P1 | plugins 4 个适配器 README 中文翻译 | 由维护者确认后补齐 `docs/zh-CN/plugins-*.md` 或在各适配器目录内放双语副本 |
| P1 | 根 README 双语入口复核 | 根 `README.md` 顶部已加入语言切换链接，需人工确认渲染效果 |
| P2 | 术语表扩充 | 翻译过程中新出现的术语（corrective patch release → 修正性补丁版本、notarization → 公证、stapling → 公证钉扎、ad-hoc signature → 临时（ad-hoc）签名、source-only → 纯源码 等）已在使用，应回填进 [glossary.md](glossary.md) |
| P3 | 锚点跳转自动化测试 | 可考虑在 CI 中加入 markdownlint 检查标题结构与链接有效性 |

## 过期内容追踪 / Stale-content tracking

文档出现以下情况之一即视为**过期**，需要在同一 PR 中同步双语：

- 英文源被修改（代码、命令、表格、版本号变更）；
- 新增/删除章节导致行数漂移超过合理范围；
- 术语表更新了某个核心术语的译法；
- 新增 CLI 命令、MCP 工具、数据库表、Hook、配置文件字段。

同步步骤见 [maintenance.md](maintenance.md) 的「双语同步流程」。
