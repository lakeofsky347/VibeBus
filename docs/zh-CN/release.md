# 发布工程

## 分发契约

GitHub Releases 是 VibeBus 唯一的官方分发渠道。当前发布契约仅为源码发布：GitHub 自动提供带标签的 **Source code (zip)** 与 **Source code (tar.gz)** 归档，而工作流仅上传源码校验证据。稳定版使用形式严格为 `vX.Y.Z` 的 SemVer 标签。工作流会校验标签解析到已检出的提交、且该提交可从 `main` 到达；它拒绝发布候选与分支名。拉取请求制品、本地包、容器镜像或手工拷贝的文件永远不能替代正式发布。

发布候选被有意禁用，直到项目采用独立的、可测试的预发布契约。不要创建 `vX.Y.Z-rc.N` 并期望稳定版工作流将其发布。

## 稳定版发布资产

源码发布工作流手工上传恰好 4 份证据文件：

| 资产 | 用途 |
| --- | --- |
| `SHA256SUMS.txt` | 其余 3 份已上传证据文件的 SHA-256 值。 |
| `VibeBus-X.Y.Z.cdx.json` | 打标签源码的规范化 CycloneDX SBOM。 |
| `supply-chain-evidence.json` | 打标签的源码修订版与已通过的 `cargo deny` 检查门。 |
| `source-release-manifest.json` | 版本、标签、源码修订版、GitHub 提供的源码归档来源与证据文件名。 |

GitHub 自动从不可变标签提供源码 ZIP 与 tarball；VibeBus 不构建或上传重复的源码 ZIP。本首次发布不包含 Windows MSI、便携 ZIP、Codex 插件 ZIP、签名可执行文件或安装程序资产。macOS、Windows 与 Linux 仍走源码与 CI 支持路径。未来的二进制分发必须使用经单独评审的契约，恢复适用的平台签名、包校验与下载制品验收。

## 维护者流程

1. 将已评审的变更合并到 `main`；确保必要的 CI、供应链检查与评审门全部通过。
2. 确认 `Cargo.toml` 与插件清单版本与预期的 SemVer 版本一致。打标签前更新 `CHANGELOG.md`。
3. 创建并推送指向可从 `main` 到达的提交的带注释 `vX.Y.Z` 标签。
4. 让 `.github/workflows/release.yml` 从该既有标签运行。它不会创建缺失的标签、不会从未打标签的提交发布、不使用受保护的发布环境，也不会为源码发布要求 Windows 签名材料。
5. 检查 GitHub 生成的源码归档、4 份已上传证据文件与工作流日志。不要编辑发布资产或重写标签；应发布修正性补丁版本。

## 校验下载

从同一 GitHub Release 下载 GitHub 生成的源码归档及全部 4 份已上传证据文件。首先校验 `source-release-manifest.json`：其 `tag` 与 `sourceRevision` 必须指向不可变发布标签和可从 `main` 到达的提交。然后校验 `SHA256SUMS.txt` 覆盖的 3 份证据文件：

```powershell
Get-Content .\SHA256SUMS.txt | ForEach-Object {
  $parts = $_ -split '\s{2,}', 2
  if ($parts.Count -ne 2) { throw "Malformed checksum entry: $_" }
  $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $parts[1]).Hash.ToLowerInvariant()
  if ($actual -ne $parts[0]) { throw "Checksum mismatch: $($parts[1])" }
}
```

确认清单提及 GitHub 自动生成的源码 ZIP 与 tarball、SBOM 不含任何本地构建路径，且供应链证据记录同一源码修订版。本源码发布不适用首次安装、升级、二进制签名或 MSI 回滚断言。

## 本地与 CI 验收

拉取请求 CI 继续在 Linux、macOS 与 Windows 上执行源码与包验收。源码发布工作流本身先在 Linux 上运行版本、Rust 质量、`cargo deny` 与规范化 SBOM 检查门，再发布证据；它不使用 Windows 运行器、不构建安装程序，也不创建二进制发布资产。
