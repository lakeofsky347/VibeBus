# 维护者交接

## 当前运行模式

VibeBus 是持续维护的开源项目。GitHub Releases 是官方分发渠道；稳定版是从可从 `main` 到达的提交打出的 `vX.Y.Z` 标签。不要把拉取请求、CI 制品、本地输出或容器镜像当作发布。

当前发布契约为源码发布。GitHub 自动提供标签的源码 ZIP 与 tarball；工作流恰好上传 `SHA256SUMS.txt`、规范化 CycloneDX SBOM、`supply-chain-evidence.json` 与 `source-release-manifest.json`。它不构建或上传 MSI、便携 ZIP、Codex 插件 ZIP、签名可执行文件、安装程序或重复的源码 ZIP。在维护者建立独立的二进制分发契约之前，macOS、Windows 与 Linux 仍走源码与 CI 支持路径。

## 修改仓库之前

1. 阅读 `README.md`、`CONTRIBUTING.md`、`SECURITY.md`、`docs/en/release.md` 与受影响的平台文档。
2. 检查当前分支与工作树（worktree）。不要覆盖其他贡献者的变更。
3. 先运行最小相关校验，再在提出与发布相关的合并前运行完整的受影响平台套件。
4. 绝不提交智能体凭据、恢复密钥、签名材料、本地数据库、备份、生成的插件二进制或 `dist/` 资产。

## 发布与恢复规则

- 稳定标签必须为 `vX.Y.Z`，与 Cargo 及插件版本一致，并指向可从 `main` 到达的提交。
- 预发布发布已禁用；启用前先创建独立的评审策略。
- GitHub 自动提供打标签的源码归档。校验 `source-release-manifest.json` 的标签与源码修订版，然后用 `SHA256SUMS.txt` 校验 SBOM、供应链证据与清单。
- 不要重写已发布的标签或其资产。撤回有问题的发布，并在修正变更评审、合并后发布更高的补丁版本。
- 本源码发布不承担二进制安装、升级、签名或安装程序回滚的声明。后续的二进制发布契约必须先定义并校验这些生命周期规则，才能发布平台制品。

## 协调边界

VibeBus 是协调事实的持久来源。原子地认领任务，只绑定真实的原生 Codex 任务 ID，检查责任策略，预留精确路径，对跨域路径使用任务作用域、可过期的覆盖（override），并在关闭前用确认（ACK）处理必需的定向消息。使用可重放（replay-safe）的订阅（subscription）peek/ACK；至少一次（at-least-once）交付要求副作用具备幂等性。

操作员初始化、轮换、凭据删除、留存（retention）审批与压缩（compaction）需要维护者控制的真实终端。这些操作有意不通过 MCP 提供，且不得由任务自动化执行。
