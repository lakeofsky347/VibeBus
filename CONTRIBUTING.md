# VibeBus contribution guide

## 提交记录要求

每次修改必须形成可追踪的 Git 提交，并在提交正文中注明：

1. 修改项：本次实际改变的代码、协议、文档或交付物。
2. 验证：执行过的测试、静态检查或人工验证；未执行时必须说明原因。
3. 后续跟进方向：仍需完成的工作、已知边界或下一阶段建议。确实没有时写明“无”并说明理由。

推荐格式：

~~~text
<type>: <简短摘要>

修改项:
- ...

验证:
- ...

后续跟进方向:
- ...
~~~

一个提交应只覆盖一个清晰的逻辑变化。不得提交 bearer token、recovery key、数据库、备份、构建缓存或其他本地秘密。

## Pull Request 要求

Pull Request 同样必须包含“修改项、验证、后续跟进方向”，并说明对用户、CLI、MCP、数据库迁移或插件兼容性的影响。仓库提供的 PR 模板应保持启用。

## 基础验证

Rust、协议或插件实现发生变化时，至少运行：

~~~powershell
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
python C:\Users\17430\.codex\skills\.system\plugin-creator\scripts\validate_plugin.py D:\MyProjects\CoWork\plugins\vibebus
~~~

纯文档变更可按影响缩减验证范围，但必须在提交正文中明确记录。
