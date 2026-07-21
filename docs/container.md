# Container delivery

VibeBus 0.10 includes a Linux `amd64` container path for CLI and stdio MCP use. The image is a command-style local service, not an HTTP daemon, so it exposes no network port.

The G1 production candidate excludes Linux distribution. Container acceptance and registry delivery remain useful evidence for the controlled Linux path, but must not be presented as a Windows `v0.10.0` GitHub Release asset.

## Security and platform boundary

- The container runs as the non-root user `10001:10001`.
- Project files are mounted at `/workspace`; SQLite state is kept under the `/data` volume through `VIBEBUS_DATA_HOME=/data`.
- Windows Credential Manager is unavailable in Linux. Do not pass `--store-credentials` in the container. Supply short-lived Agent credentials explicitly or through `VIBEBUS_AGENT_TOKEN`, preferably from the host's secret facility.
- Never bake Agent, recovery, operator, ACR, certificate, or cloud credentials into the image, Dockerfile, build arguments, labels, repository, or report.
- Operator mutations require a real interactive terminal and remain intentionally unavailable through redirected automation or MCP.
- The Dockerfile pins its Rust builder and Debian runtime bases to linux/amd64 manifest digests and labels each image with the Git source revision. The acceptance and ACR helper verify the resulting platform and revision.

## Build and acceptance

From PowerShell with Docker Desktop in Linux-container mode:

```powershell
./scripts/test-container.ps1 -ImageTag vibebus:0.10.0-local
```

The acceptance script builds the multi-stage image and verifies:

1. `vibebus 0.10.0` from the runtime image;
2. explicit project initialization on a disposable bind mount;
3. SQLite WAL, foreign keys, and `doctor.ok=true`;
4. Linux registration with secrets captured only in the script process;
5. an authenticated empty Inbox through `VIBEBUS_AGENT_TOKEN`;
6. stdio MCP initialization and `tools/list`;
7. non-root runtime and cleanup of disposable host data.

The accepted local run on 2026-07-18 produced:

| Field | Accepted value |
| --- | --- |
| Image | `vibebus:0.10.0-local` |
| Platform | `linux/amd64` |
| Size | 31,240,216 bytes |
| Runtime user | `10001:10001` |
| SQLite | WAL, foreign keys enabled |
| stdio MCP | initialize succeeded, 47 tools listed |

The local-image acceptance was followed by a remote ACR push and manifest inspection, recorded below.

Later clean rebuilds during repository intake reproduced the same 31,240,216-byte size, runtime user, version, SQLite checks, and 47-tool MCP surface but produced different local image IDs. They were not pushed. Local image IDs are not treated as bit-for-bit reproducibility evidence; the published ACR digest below remains the remote release identity.

To use the image manually:

```powershell
docker run --rm `
  --mount type=bind,source=D:\path\to\repo,target=/workspace `
  --mount type=volume,source=vibebus-data,target=/data `
  vibebus:0.10.0-local `
  doctor --root /workspace
```

For stdio MCP, keep stdin attached:

```powershell
docker run --rm -i `
  --mount type=bind,source=D:\path\to\repo,target=/workspace `
  --mount type=volume,source=vibebus-data,target=/data `
  vibebus:0.10.0-local `
  mcp --root /workspace
```

## Alibaba Cloud ACR push

Create the ACR instance, namespace, and repository in the Alibaba Cloud console. Configure a dedicated Registry credential or short-lived credential with only the required push/pull permissions. Log in from a user-controlled terminal so the password never enters the repository or an automated task:

```powershell
docker login <registry-host>.aliyuncs.com
```

Then run the repository-owned push helper with the complete repository path, excluding the tag:

```powershell
./scripts/push-acr.ps1 `
  -Repository <registry-host>.aliyuncs.com/<namespace>/vibebus `
  -SourceImage vibebus:0.10.0-local `
  -Tag 0.10.0
```

The helper refuses non-ACR targets, requires an existing Docker login, tags the accepted local image, pushes it, then verifies that the remote index digest matches the push result, exactly one runnable `linux/amd64` manifest is present when the remote is an index, and the pulled remote runtime label matches the checked-out Git revision. It never accepts or prints a password.

Record the returned `image`, index `digest`, `runnableManifestDigest`, `platform`, and `sourceRevision` in acceptance evidence. The image reference alone is not proof of a push; the remote digest and platform/revision verification are required.

## Accepted ACR result

The 2026-07-18 release push completed with the following non-secret evidence:

| Field | Accepted value |
| --- | --- |
| Image | `crpi-21kb7zn8owb85qa2.cn-beijing.personal.cr.aliyuncs.com/for_plugin/vibebus:0.10.0` |
| Remote index digest | `sha256:71e39f0a3af75e9626dd6d1c313f1edd3ef65d7446c0a8497147043036227118` |
| Runnable manifest | `sha256:8f43d9c7ae26c9eaedc3746b5f1e60c21737fef0d2cc45e579b3ed01a5d4eb94` |
| Platform | `linux/amd64` |
| Push time (UTC) | `2026-07-18T12:47:55Z` |

`docker push` returned the index digest and a separate `docker buildx imagetools inspect` returned the same value. A verbose remote manifest inspection confirmed the runnable platform. The other index entry is BuildKit provenance/attestation metadata and is not a second runtime platform.
