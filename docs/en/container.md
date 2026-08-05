# Container development

VibeBus provides a Linux `amd64` container path for CLI and stdio MCP development. It is a command-style local service, not an HTTP daemon, and exposes no network port by default. Container images are not GitHub Release assets under the current distribution contract.

## Security and platform boundary

- The container runs as non-root user `10001:10001`.
- Mount the project at `/workspace` and keep SQLite state in a separate `/data` volume through `VIBEBUS_DATA_HOME=/data`.
- Linux has no Windows Credential Manager or macOS Keychain backend. Supply short-lived Agent credentials from the host environment; do not use `--store-credentials`.
- Never bake tokens, recovery keys, Operator secrets, certificates, registry credentials, or database files into an image, Dockerfile, build argument, label, repository, or report.

## Build and acceptance

From PowerShell with Docker Desktop in Linux-container mode:

```powershell
./scripts/test-container.ps1 -ImageTag vibebus:local
```

The acceptance script builds the multi-stage image and checks the CLI version, an isolated project, SQLite health, authenticated Inbox access, stdio MCP initialization, non-root runtime, and temporary-data cleanup.

To use the image manually:

```powershell
docker run --rm -i `
  --mount type=bind,source=D:\path\to\repo,target=/workspace `
  --mount type=volume,source=vibebus-data,target=/data `
  vibebus:local mcp --root /workspace
```

Publishing an OCI image is a separate maintainer decision. If a registry channel is added later, its tag, digest, supported platforms, SBOM, provenance, verification steps, and rollback policy must be documented and reviewed before it is described as an official distribution channel.
