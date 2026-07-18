# VibeBus handoff

## Current baseline

VibeBus 0.8 is a working native Windows MVP. Rust core, CLI, stdio MCP, Codex Skill, SessionStart Hook, repo marketplace, tests, health check, backup, repeatable Windows CI, per-user MSI/portable packaging, checksums, and fail-closed production signing/publishing automation are present. Windows current-user credential storage and token fallback, Agent recovery, reservation renewal, retry idempotency, ordered events, replay-safe named subscriptions, structured handoffs, message closing, durable task/thread bindings, operator-approved bounded retention, and resume snapshots are implemented. The complete real-terminal operator lifecycle has also passed on a disposable project, including exact-plan consumption/replay, generation invalidation, and explicit vault cleanup. The fixed two-real-task desktop acceptance is now complete: two independent user-owned Codex top-level tasks executed B1/A1/B2/A2, strict preflight passed 68/68, and the clean-checkout auditor passed 178/178 before the final root handoff was ACKed and closed.

The consumed two-real-task fixture and its accepted evidence are documented in `docs/desktop-acceptance.md`; do not rerun or reuse that fixed fixture. The repository-owned preflight remains useful as a regression template, including its Windows PowerShell 5.1 empty-list hardening, but a future live run must use a new fixture/run ID and regenerated expected state.

Local recovery copies are kept under the ignored `backups/` directory:

- `vibebus-0.8-pre-desktop-acceptance.db` is the schema-v9 recovery point after creating the deterministic desktop fixture and before either user-owned top-level task exists; it is 512,000 bytes with SHA-256 `0079a09f200dd5c7210c1dbb563da3b77f29b80b17d5c2504168a1bae230611c` and is published under `DESKTOP-ACCEPTANCE-001`.
- `vibebus-0.8-desktop-acceptance.db` is the accepted post-run online backup after all B1/A1/B2/A2 evidence and the final root handoff ACK/close; it is 589,824 bytes with SHA-256 `5928201fd62fa0d5a7588a91650bfaf86ace173f0c43f2b10eb9f4c8f232d37b` and is published under `DESKTOP-ACCEPTANCE-001`.
- `vibebus-source-0.8-operator-cleanup.zip` is the committed 0.8 source after adding explicit operator-vault cleanup and its disposable acceptance runbook; SHA-256 `1c75669d8ae107ebcc71c7c0faebda0677bb96db48a258262008d288b6240dbc`.
- `vibebus-0.8-operator-cleanup.db` is the accepted live schema-v9 coordination snapshot before disposable real-terminal acceptance; SHA-256 `db029e78e31eafe16dbc7bfad83345a9c0c9ba7d9ed74200b5ef8abd59cd0372`.
- `vibebus-source-0.7-final.zip` is the final committed 0.7 source, CI/release workflows, installer authoring, documentation, marketplace, project marker, and packaged plugin produced with `git archive`; ignored toolchains, release outputs, runtime data, and credentials are excluded.
- `vibebus-0.7-final.db` is the accepted schema-v8 coordination snapshot after release-package and local plugin acceptance; SHA-256 `d16d1eb828beb40d947cbe851fdb92325577107ef7035f0d7fb6955e9b5715b5`.
- `vibebus-0.8-pre-migration.db` is the schema-v8 recovery point made by the 0.7 binary immediately before the live schema-v9 migration; SHA-256 `78d9479ec2b394cc247e6078aac89beea7fe615942aa367e6b1d848ea4a58ee5`.
- `vibebus-0.8-final.db` is the accepted schema-v9 coordination snapshot after migration and 0.8 package/plugin acceptance, before any project operator credential was initialized; SHA-256 `35a6763e2d0be92ec7f3a3efa5ddf87eaba870d1afffda9a3b90c3407cdff7a8`.
- `vibebus-source-0.6-final.zip` is the final committed 0.6 source, documentation, marketplace, project marker, and packaged plugin produced with `git archive`; ignored build toolchains, targets, runtime data, and credentials are excluded.
- `vibebus-0.6-final.db` is the accepted schema-v8 coordination snapshot after credential-vault acceptance; SHA-256 `86a77a9e07bf4d0246223c912fe7bc54d3d97c7568ad307086749f9f7233fe2f`.
- `vibebus-source-0.5-final-r2.zip` is the final 0.5 source, documentation, marketplace, project marker, and packaged plugin, excluding build toolchains and targets. The earlier `vibebus-source-0.5-final.zip` predates the replay-policy consistency check and is superseded.
- `vibebus-0.5-pre-retention.db` is the pre-cleanup recovery point; SHA-256 `f3035b043f44a8e11893f2f44e963ce875e0450d1ce1484296ddaaae5b1020ed`.
- `vibebus-0.5-final.db` is the accepted schema-v8 post-cleanup project snapshot; SHA-256 `4e100e59647f54428716744336fddb5728e5b165b264551fa34ed8c1a631a4c3`.
- The 0.1 through 0.4 source/database backups remain available for rollback/reference.

The disposable operator acceptance recovery points are retained under ignored `.tools/operator-acceptance/`: `pre-operator.db`, `pre-retention.db`, `pre-rotation.db`, and `pre-cleanup.db`. Their hashes and accepted lifecycle evidence are recorded in `docs/operator-acceptance.md`; the disposable live `project` and `data` directories and both Windows vault entries were removed after verification.

Run these first:

```powershell
git status --short
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
powershell -File .\scripts\build-release.ps1
powershell -File .\scripts\test-installer.ps1 -MsiPath .\dist\VibeBus-0.8.0-windows-x64.msi -ExpectedVersion 0.8.0
```

The project truth is in `README.md`, `docs/architecture.md`, `docs/protocol.md`, and `docs/acceptance.md`.

## Known boundaries

- The packaged binary is Windows-only; the hook has a Windows implementation and a no-op Unix command.
- Plugin MCP calls must pass an explicit absolute project `root`.
- Agent bearer tokens and recovery keys can be stored under `VibeBus:<project-id>:<agent>` in Windows Credential Manager and omitted from later calls. This protects at rest and against accidental repository/task disclosure, but all processes already running as the same Windows user remain inside the trust boundary.
- Message polling occurs at safe task boundaries. There is no promise of interrupting an active generation.
- The accepted desktop Agent vault entries `desktop-a-20260717-01` and `desktop-b-20260717-01` remain stored intentionally for regression. Deleting them is an explicit, separate local-vault action and does not remove database audit history.
- Replay-safe subscription peek/ack provides at-least-once batch access, not exactly-once consumer side effects. Legacy poll still consumes and commits in one call.
- Retention removes bounded logical history but does not automatically run SQLite `VACUUM`; physical file compaction remains an explicit maintenance decision because it requires a more disruptive exclusive operation.
- Remote synchronization is not implemented; retention state and confirmation plans are local to one project database.
- The optional best-effort bridge to native Codex thread tools is not implemented.
- Destructive retention is default-deny until a local maintainer initializes the separate operator credential in a real terminal. MCP has no operator mutation tools. Every new apply needs a short-lived approval for the exact plan and current operator generation; completed-run replay remains approval-free and cannot delete twice.
- Operator vault cleanup is explicit and CLI-only: `operator delete-credential` requires a real terminal plus `delete:<project-id>`, removes only the Windows vault entry, and leaves the database credential configured with `ready=false`.
- The current live project intentionally reports `operator.ready=false`. No operator credential was initialized on the user's behalf. The stored target, when explicitly initialized, is `VibeBusOperator:<project-id>` and remains inside the same-Windows-user trust boundary.
- Pull-request CI produces unsigned acceptance packages. Production release automation requires both Windows signing Secrets and refuses unsigned publication; the repository has not been given a production certificate, tag, or real release during implementation.
- The installer is intentionally per-user and does not mutate Codex configuration through custom actions. The installed marketplace must be registered explicitly.
- WiX 4.0.6 is pinned to avoid automating WiX 7 OSMF EULA acceptance. See `docs/release.md` before changing the installer toolchain.
- Windows CLI callers should use `artifact publish --metadata-file` for complex JSON; MCP accepts metadata as a native object.

## Recommended next slice

1. Implement Agent-scoped context sync and confirmed decisions as specified in `docs/plan-gap-analysis.md`; this closes the sole partial original MVP criterion.
2. Add responsibility-domain policy plus deterministic Git-commit/test-result Hooks without weakening reservations, authentication, idempotency, or bounded payload rules.
3. Configure the protected `release` environment and a real Windows code-signing certificate, then execute the tag, signed asset, disposable-profile install/uninstall, and downloaded-checksum acceptance in `docs/release.md`.
4. Decide separately whether the retained desktop A/B vault entries and the live project's intentionally unconfigured Operator state should change; neither decision is implicit in normal development.
5. Evaluate an optional Codex task notification bridge only after authoritative context projection exists; keep SQLite authoritative and treat UI delivery as best effort.

## Startup prompt

```text
Read README.md, docs/architecture.md, docs/protocol.md, docs/acceptance.md, docs/plan-gap-analysis.md, docs/release.md, and docs/HANDOFF.md. Verify the current checkout and run the test, clippy, unsigned release-build, and MSI acceptance commands before changing code. Treat the fixed desktop-20260717-01 fixture as consumed accepted evidence; do not rerun or reuse it. Preserve independent Codex top-level tasks and the single SQLite source of truth. Prefer `storeCredentials=true`, confirm `vibebus_credential_status`, and never place bearer, recovery, operator, PFX, or certificate-password secrets in repository, task, event, message, or logs. Preserve message close, task/thread terminal, retained-history floor, and pending-delivery protection semantics. Never apply retention without a fresh backup, reviewed plan, and user-performed interactive operator approval; never invoke operator mutation through automation. Never publish an unsigned production release or accept third-party legal terms on behalf of the repository owner. Prefer subscription peek/ack for replay-safe delivery, keep consumer side effects idempotent, and use legacy poll only when consume-on-poll loss is acceptable. Start with Agent-scoped context sync and confirmed decisions, and do not claim that VibeBus can interrupt an already-running model generation.
```
