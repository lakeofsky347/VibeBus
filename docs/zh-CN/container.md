# 容器开发

VibeBus 提供 Linux `amd64` 容器路径，用于 CLI 与 stdio MCP 开发。它是命令式本地服务，而非 HTTP 守护进程，默认不暴露任何网络端口。在当前分发契约下，容器镜像不是 GitHub Release 资产。

## 安全与平台边界

- 容器以非 root 用户 `10001:10001` 运行。
- 将项目挂载到 `/workspace`，并通过 `VIBEBUS_DATA_HOME=/data` 将 SQLite 状态保存在独立的 `/data` 卷中。
- Linux 没有 Windows 凭据管理器或 macOS 钥匙串后端。从宿主机环境提供短期智能体凭据；不要使用 `--store-credentials`。
- 切勿将令牌、恢复密钥（recovery key）、操作员机密、证书、注册表凭据或数据库文件烘焙进镜像、Dockerfile、构建参数、标签、仓库或报告。

## 构建与验收

在 Linux 容器模式的 Docker Desktop 下使用 PowerShell：

```powershell
./scripts/test-container.ps1 -ImageTag vibebus:local
```

验收脚本构建多阶段镜像，并检查 CLI 版本、隔离项目、SQLite 健康、经认证的收件箱（inbox）访问、stdio MCP 初始化、非 root 运行时与临时数据清理。

手动使用镜像：

```powershell
docker run --rm -i `
  --mount type=bind,source=D:\path\to\repo,target=/workspace `
  --mount type=volume,source=vibebus-data,target=/data `
  vibebus:local mcp --root /workspace
```

发布 OCI 镜像是独立的维护者决策。如果以后添加注册表渠道，其标签、摘要（digest）、支持的平台、SBOM、来源证明、校验步骤与回滚策略，必须先记录并评审，才可描述为官方分发渠道。
