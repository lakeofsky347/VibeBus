# macOS 开发与本地交付

VibeBus 拥有原生 macOS 路径，支持 CLI、stdio MCP、macOS 钥匙串（Keychain）凭据、SessionStart/PostToolUse/Stop Hook 与 Codex 插件打包。已验收的本地宿主机为 Apple Silicon（`arm64`）；打包脚本也将原生 Intel 构建命名为 `x64`，但 Intel 尚未获得同样的线上验收。

## 前置条件

安装 Xcode 命令行工具与仓库锁定的 Rust 工具链：

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.97.1 --profile minimal --component rustfmt,clippy
```

仓库 `rust-toolchain.toml` 会自动选择 Rust 1.97.1。登录 shell 必须加载 `$HOME/.cargo/env`。

## 构建与验收

在仓库根目录：

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/test-lifecycle-hooks.sh
./scripts/test-macos-keychain.sh
./scripts/package-plugin-macos.sh
./scripts/validate-plugin-macos.sh \
  ./dist/staging/VibeBus-0.11.0-macos-arm64/plugins/vibebus
(cd dist && shasum -a 256 -c SHA256SUMS-macos-arm64.txt)
```

钥匙串测试夹具使用隔离的项目与数据目录。它写入一次性智能体（Agent）凭据，证明基于凭据保险库（vault）的收件箱（inbox）访问与恢复轮换，删除该条目，并证明后续的无令牌拒绝。通过真实伪终端，它还会初始化已脱敏（redacted）的操作员（Operator），校验数据库/钥匙串生成一致，轮换之，并执行精确确认删除路径。两条钥匙串条目与全部临时状态均被移除。

`package-plugin-macos.sh` 构建原生发布二进制，默认应用临时（ad-hoc）本地签名，暂存含 macOS 专用 `.mcp.json` 的市场，校验之，并创建：

- `VibeBus-X.Y.Z-macos-arm64.tar.gz`；
- `VibeBus-Codex-plugin-X.Y.Z-macos-arm64.zip`；
- `SHA256SUMS-macos-arm64.txt`；
- `release-manifest-macos-arm64.json`。

临时签名仅用于本地完整性校验。macOS 将每个变更过的临时二进制视为不同的程序（用于基于文件的钥匙串授权）。因此 VibeBus 在 CLI/Hook 操作中禁用交互式钥匙串界面，并在重建二进制未获授权时返回可操作的错误，而非挂起无头进程。请先完成代码变更与打包，再注册持久化的本地智能体；或在本地重建后以新的智能体名称重新注册。

对于维护者持有的 Developer ID Application 证书，将 `VIBEBUS_CODESIGN_IDENTITY` 设置为证书标识。脚本随后以强化运行时与安全时间戳签名；它记录 `signature: "identity"` 而非声称已公证：

```sh
VIBEBUS_CODESIGN_IDENTITY='Developer ID Application: Example Corp (TEAMID)' \
  ./scripts/package-plugin-macos.sh
```

macOS 包是源码与 CI 验收输出，并非当前 GitHub Release 资产。未来的公开 macOS 分发契约必须校验所选 Developer ID 标识、公证、适用的公证钉扎（stapling）、校验和以及干净的下载制品 Gatekeeper 验收。仓库绝不存储该标识的私钥或公证凭据。

## 安装本地插件

先构建，再注册暂存市场，而非面向 Windows 的源码负载：

```sh
codex plugin marketplace add \
  "$PWD/dist/staging/VibeBus-0.11.0-macos-arm64"
codex plugin add vibebus@vibebus-local
```

安装后启动新的 Codex 任务。评审并信任 Hook 定义。已安装的 `.mcp.json` 启动 `./bin/vibebus mcp`；macOS Hook 命令以隐藏的 `hook` 子命令启动同一二进制。

## 初始化本地运行时状态

不要拷贝 Windows SQLite 运行时或导出 Windows 凭据。保留被跟踪的 `.vibebus/project.json`，然后创建全新的 Mac 本地状态：

```sh
./plugins/vibebus/bin/vibebus --root /path/to/project \
  register --name mac-worker --role implementation --store-credentials
./plugins/vibebus/bin/vibebus --root /path/to/project \
  credential status --agent mac-worker
./plugins/vibebus/bin/vibebus --root /path/to/project \
  inbox --agent mac-worker
```

成功存储会报告后端 `macos-keychain`、`stored=true` 与 `secretsRedacted=true`。智能体条目使用服务 `VibeBus:<project-id>:<agent>`、账户 `VibeBus`；操作员条目使用独立服务 `VibeBusOperator:<project-id>`。SQLite 仅存储凭据摘要。

默认数据库位于仓库之外：

```text
~/Library/Application Support/dev.VibeBus.VibeBus/projects/<project-id>/vibebus.db
```

对隔离测试使用 `VIBEBUS_DATA_HOME` 或 `--data-home`。`credential delete` 仅移除钥匙串条目；它不撤销或删除智能体数据库记录。操作员初始化与破坏性维护仍是明确的真实终端操作，绝不由安装脚本或 MCP 执行。

## 平台边界

- macOS 与 Windows 拥有原生当前用户凭据保险库后端；Linux/容器构建仍需要显式或环境注入的智能体令牌。
- macOS Hook 使用原生 Rust 二进制，运行时不需要 PowerShell、Node、Python、jq、转录访问、diff 或原始测试日志。Python 仅由仓库校验夹具使用。
- 当前包是宿主机架构原生包，而非 Universal 2 二进制。
- 运行时状态仅限单台宿主机。VibeBus 不跨机器同步数据库或钥匙串条目。
