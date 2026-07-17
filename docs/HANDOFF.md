# VibeBus handoff

## Current baseline

VibeBus 0.7 is a working native Windows MVP. Rust core, CLI, stdio MCP, Codex Skill, SessionStart Hook, repo marketplace, tests, health check, backup, repeatable Windows CI, per-user MSI/portable packaging, checksums, and fail-closed production signing/publishing automation are present. Windows current-user credential storage and token fallback, Agent recovery, reservation renewal, retry idempotency, ordered events, replay-safe named subscriptions, structured handoffs, message closing, durable task/thread bindings, preview-and-confirm bounded retention, and resume snapshots are implemented.

Local recovery copies are kept under the ignored `backups/` directory:

- `vibebus-source-0.7-final.zip` is the final committed 0.7 source, CI/release workflows, installer authoring, documentation, marketplace, project marker, and packaged plugin produced with `git archive`; ignored toolchains, release outputs, runtime data, and credentials are excluded.
- `vibebus-0.7-final.db` is the accepted schema-v8 coordination snapshot after release-package and local plugin acceptance; SHA-256 `d16d1eb828beb40d947cbe851fdb92325577107ef7035f0d7fb6955e9b5715b5`.
- `vibebus-source-0.6-final.zip` is the final committed 0.6 source, documentation, marketplace, project marker, and packaged plugin produced with `git archive`; ignored build toolchains, targets, runtime data, and credentials are excluded.
- `vibebus-0.6-final.db` is the accepted schema-v8 coordination snapshot after credential-vault acceptance; SHA-256 `86a77a9e07bf4d0246223c912fe7bc54d3d97c7568ad307086749f9f7233fe2f`.
- `vibebus-source-0.5-final-r2.zip` is the final 0.5 source, documentation, marketplace, project marker, and packaged plugin, excluding build toolchains and targets. The earlier `vibebus-source-0.5-final.zip` predates the replay-policy consistency check and is superseded.
- `vibebus-0.5-pre-retention.db` is the pre-cleanup recovery point; SHA-256 `f3035b043f44a8e11893f2f44e963ce875e0450d1ce1484296ddaaae5b1020ed`.
- `vibebus-0.5-final.db` is the accepted schema-v8 post-cleanup project snapshot; SHA-256 `4e100e59647f54428716744336fddb5728e5b165b264551fa34ed8c1a631a4c3`.
- The 0.1 through 0.4 source/database backups remain available for rollback/reference.

Run these first:

```powershell
git status --short
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
powershell -File .\scripts\build-release.ps1
powershell -File .\scripts\test-installer.ps1 -MsiPath .\dist\VibeBus-0.7.0-windows-x64.msi -ExpectedVersion 0.7.0
```

The project truth is in `README.md`, `docs/architecture.md`, `docs/protocol.md`, and `docs/acceptance.md`.

## Known boundaries

- The packaged binary is Windows-only; the hook has a Windows implementation and a no-op Unix command.
- Plugin MCP calls must pass an explicit absolute project `root`.
- Agent bearer tokens and recovery keys can be stored under `VibeBus:<project-id>:<agent>` in Windows Credential Manager and omitted from later calls. This protects at rest and against accidental repository/task disclosure, but all processes already running as the same Windows user remain inside the trust boundary.
- Message polling occurs at safe task boundaries. There is no promise of interrupting an active generation.
- Replay-safe subscription peek/ack provides at-least-once batch access, not exactly-once consumer side effects. Legacy poll still consumes and commits in one call.
- Retention removes bounded logical history but does not automatically run SQLite `VACUUM`; physical file compaction remains an explicit maintenance decision because it requires a more disruptive exclusive operation.
- Remote synchronization is not implemented; retention state and confirmation plans are local to one project database.
- The optional best-effort bridge to native Codex thread tools is not implemented.
- Pull-request CI produces unsigned acceptance packages. Production release automation requires both Windows signing Secrets and refuses unsigned publication; the repository has not been given a production certificate, tag, or real release during implementation.
- The installer is intentionally per-user and does not mutate Codex configuration through custom actions. The installed marketplace must be registered explicitly.
- WiX 4.0.6 is pinned to avoid automating WiX 7 OSMF EULA acceptance. See `docs/release.md` before changing the installer toolchain.
- Windows CLI callers should use `artifact publish --metadata-file` for complex JSON; MCP accepts metadata as a native object.

## Recommended next slice

1. Perform the two-real-task desktop acceptance recorded in `docs/acceptance.md`; plugin installation is complete, but creating user-owned top-level tasks requires explicit user action/authorization.
2. Configure the protected `release` environment and a real Windows code-signing certificate, then execute the tag, signed asset, disposable-profile install/uninstall, and downloaded-checksum acceptance in `docs/release.md`.
3. Define whether an operator-only authorization capability is needed beyond authenticated plan confirmation for destructive maintenance.
4. Only then evaluate an optional Codex thread notification bridge; keep SQLite authoritative and treat UI delivery as best effort.

## Startup prompt

```text
Read README.md, docs/architecture.md, docs/protocol.md, docs/acceptance.md, docs/release.md, and docs/HANDOFF.md. Verify the current checkout and run the test, clippy, unsigned release-build, and MSI acceptance commands before changing code. Preserve independent Codex top-level tasks and the single SQLite source of truth. Prefer `storeCredentials=true`, confirm `vibebus_credential_status`, and never place bearer, recovery, PFX, or certificate-password secrets in repository, task, event, message, or logs. Preserve message close, task/thread terminal, retained-history floor, and pending-delivery protection semantics. Never apply retention without a fresh reviewed plan and a pre-cleanup backup. Never publish an unsigned production release or accept third-party legal terms on behalf of the repository owner. Prefer subscription peek/ack for replay-safe delivery, keep consumer side effects idempotent, and use legacy poll only when consume-on-poll loss is acceptable. Start with the first incomplete item under Recommended next slice, and do not claim that VibeBus can interrupt an already-running model generation.
```
