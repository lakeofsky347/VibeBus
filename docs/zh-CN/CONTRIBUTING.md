# 为 VibeBus 做贡献（Contributing to VibeBus）

感谢你的贡献。参与即表示你同意遵守[行为准则（Code of Conduct）](CODE_OF_CONDUCT.md)。

## 从 issue 或讨论开始

可复现的 bug 与功能提案请使用 issue 模板。发现安全漏洞时不要提交公开 issue，请遵循 [SECURITY.md](SECURITY.md)。小的文档修复可以直接提出，但请说明它们解决的问题。

## 开发预期

1. 在独立分支上开展工作，并让每个 pull request 只涉及一个可评审的主题。
2. 维护项目的安全边界：不要添加凭据、恢复材料、本地数据库、备份、构建产物或发布归档。
3. 当行为、兼容性、安装或发布策略发生变化时，同步更新测试与文档。
4. 使用周围文件已确立的项目语言与风格。

对于 Rust、协议或插件相关的改动，请运行：

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

当你的改动影响平台打包或安装程序时，请运行相应的检查。纯文档改动可以使用范围更窄的验证，但 pull request 必须说明已检查和未运行的项目。

## Pull requests

请使用 pull-request 模板，说明面向用户的用途、验证方式、兼容性影响与 SemVer 影响。不要把构建产物放进 pull request；GitHub Releases 只从 `main` 上被接受的稳定标签创建。

维护者会从正确性、测试、文档、安全、兼容性与发布就绪度等角度评审改动。pull request 不等于发布，也不能授权创建标签、GitHub Release 或外部发布。

## 版本控制与变更日志

VibeBus 使用 SemVer。对于面向用户的修复、功能、破坏性变更、安全变更以及发布行为变更，请在 [CHANGELOG.md](CHANGELOG.md) 的 `Unreleased` 下添加条目。历史条目保持不可变；请描述当前行为，而不是改写发布历史。
