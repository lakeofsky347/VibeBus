# 安全策略（Security policy）

## 受支持的版本

安全修复面向最新的稳定 GitHub Release 与 `main` 分支开发。较旧的版本可能会获得指导，但不保证收到 backport。请查看 [GitHub Releases](https://github.com/lakeofsky347/VibeBus/releases) 获取当前稳定版本、其 GitHub 生成的源码归档及源码验证证据集。

稳定的纯源码（source-only）发布受以下措施保护：固定版本的 GitHub Actions、Rust 质量门禁、`cargo deny` 的 advisory/license/source 门禁、标准化的 CycloneDX SBOM 生成、不可变标签到 `main` 的验证，以及已上传证据文件的 SHA-256 校验和。这些控制降低了风险，但不能替代部署前的独立评审。

## 报告漏洞

对于疑似漏洞，不要提交公开 issue、讨论或 pull request。请使用仓库的[私有漏洞报告表单](https://github.com/lakeofsky347/VibeBus/security/advisories/new)。

请尽可能包含受影响版本或提交、支持平台、影响范围、最小安全复现步骤及建议的缓解措施。请勿附上 Agent 令牌、恢复密钥、Operator 机密、钥匙串或凭据管理器导出、签名材料、云凭据、本地数据库或无关的个人数据。

维护者会私下确认并分类处理报告。协调披露（coordinated disclosure）的时间取决于严重程度、可复现性、受影响用户以及安全修复的可用性。在维护者指明已发布的版本或公告之前，请勿假定问题已被修复。

## 涉及安全的贡献

安全相关的改动必须保持项目的作用域隔离、凭据脱敏、破坏性维护需显式 Operator 批准，以及受约束的 no-transcript/no-raw-log 生命周期事实边界。如果提议的改动削弱了上述任一控制，请在私有报告或 pull request 中相应说明威胁模型、备选方案、迁移、测试与回滚计划。
