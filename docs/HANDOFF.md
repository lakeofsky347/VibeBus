# VibeBus handoff

## Current baseline

VibeBus 0.10 is a working native Windows and macOS MVP with an accepted Linux `amd64` container path. Rust core, CLI, stdio MCP, Codex Skill, SessionStart/PostToolUse/Stop Hooks, repo marketplace, tests, health check, online backup, isolated restore/import drill, CLI-only backup-first offline compaction, repeatable Windows/macOS/Linux CI, Windows MSI/portable packaging, macOS ARM64 local packaging, checksums, guarded Alibaba Cloud ACR delivery, and fail-closed Windows production signing/publishing automation are present. Windows Credential Manager and macOS Keychain storage/token fallback, Agent recovery, responsibility-domain policy, task-scoped expiring overrides, reservation renewal, retry idempotency, immutable Git/test facts, ordered events, replay-safe named subscriptions, structured handoffs, review-only handoff proposals, message closing, durable task/thread bindings, operator-approved bounded retention, resume snapshots, immutable confirmed decisions, and Agent-scoped context sync are implemented. The original MVP comparison passes 12 of 12 criteria, and the planned local P1 responsibility/lifecycle slice is complete. The complete real-terminal Windows operator lifecycle and fixed two-real-task desktop acceptance remain accepted historical evidence: two independent user-owned Codex top-level tasks executed B1/A1/B2/A2, strict preflight passed 68/68, and the clean-checkout auditor passed 178/178 before the final root handoff was ACKed and closed.

The macOS adaptation is active on `codex/macos-adaptation`. On macOS 26.5.2 ARM64, Rust 1.97.1 is installed and loaded by zsh; all 39 Rust tests pass; Clippy/formatting are repository gates; native lifecycle Hooks pass 7/7; a disposable Agent/Operator real-Keychain lifecycle passes 11/11 and cleans up; the live Mac project reports schema 11/WAL/integrity `ok`; final-build Agent `macos-adaptation-final` is stored redacted in Keychain and is readable by the installed cache; and the locally staged/installed `vibebus@vibebus-local` 0.10.0 plugin uses the same accepted Mach-O. Keychain UI is disabled for headless operations so credentials tied to superseded ad-hoc builds fail fast rather than suspending a CLI, MCP server, or Hook. The package is ad-hoc signed for local validation, not Developer ID signed or notarized; the packaging script accepts an explicit `VIBEBUS_CODESIGN_IDENTITY` for hardened-runtime identity signing without claiming notarization.

The GitHub repository is public as of 2026-07-18 and anonymous API access confirms `visibility=public`. Before that change, all 24 reachable commits were checked against 13 high-confidence secret patterns and sensitive-path rules with zero findings. GitHub secret scanning and push protection are enabled, private vulnerability reporting is enabled, and the repository currently has zero open secret-scanning alerts. No production signing credential, Operator secret, Agent token, recovery key, or local database was published.

The active delivery stack now extends through cumulative draft PR #12 (`codex/public-integration` over `codex/backup-restore`). It merges the reviewed plugin-packaging commit `f70bd2e59f7ea2a076435ca41abf018d270bde95` as `aa4207d6cf1f03349821c1a2a3d0f160c27e2cf5` and adds the public security policy in `92744b1cb45f22ebaa52028a8c61260d2aca41ea`. GitGuardian, Linux container acceptance, and Windows build/test/package all pass on that cumulative branch. PR #10 is closed as superseded; its commit and branch remain available. PR #11 remains the backup/restore base for PR #12. Merge the older stack in order through PR #9 and PR #11, then refresh PR #12 before merge; do not push or merge these drafts directly to the default branch. The local plugin cache was previously accepted at 0.10.0 with changed Hooks reviewed in a fresh Codex task.

Draft PR #13 (`codex/offline-compaction` over `codex/public-integration`) adds the guarded offline maintenance implementation at `963c889d77565340234ddc473e7b8e7b90770846`. Local acceptance passed 38 Rust tests, Clippy, 7/7 Hooks, plugin validation, unsigned release build, isolated backup/restore, and MSI payload checks. The live project database was not compacted. Keep PR #13 stacked on PR #12 and refresh it after its base moves; never merge it ahead of the older stack.

The consumed two-real-task fixture and its accepted evidence are documented in `docs/desktop-acceptance.md`; do not rerun or reuse that fixed fixture. The repository-owned preflight remains useful as a regression template, including its Windows PowerShell 5.1 empty-list hardening, but a future live run must use a new fixture/run ID and regenerated expected state.

Local recovery copies are kept under the ignored `backups/` directory:

- `vibebus-0.10-responsibility-hooks.db` is the accepted schema-v11 online backup after live responsibility override, task-scoped reservation, Hook test fact, Git commit fact, and proposal acceptance; it is 790,528 bytes with SHA-256 `21fcd08e335caa1cabb68e898c45fe578adf672ea2c5f417cbfd50e11f24e669` and artifact ID `art_5ed78932a89f4bd79ae68a4bf129c737`.
- `vibebus-source-0.10-responsibility-hooks.zip` is the committed 0.10 implementation source at `e1297e42ede568aed921c6c6ec144ad7e5f86aea`; it is 3,002,860 bytes with SHA-256 `63d74278c99c838e4c4b1ca6ec0ef1b18fad1550182f492e06f02d02d2b773c9` and artifact ID `art_89e754cf1aa04834adc61c5bb1adeefd`. It deliberately predates the final handoff-only documentation commit.
- `vibebus-0.9-pre-migration.db` is the schema-v9 recovery point created immediately before the live schema-v10 migration; it is 630,784 bytes with SHA-256 `9780a050226e1add79770205ec36b6f0a3b643ee81d66815984c393f31060681`.
- `vibebus-0.9-context-sync.db` is the accepted schema-v10 online backup after live confirmed-decision/context-sync and package/plugin acceptance, before closing the implementation task; it is 647,168 bytes with SHA-256 `b273c8f9425cbb210d3ac6e66a9a1b1fa56f6d9cf476edb93b1d51a29301331a`.
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
powershell -File .\scripts\test-lifecycle-hooks.ps1
powershell -File .\scripts\build-release.ps1
powershell -File .\scripts\test-installer.ps1 -MsiPath .\dist\VibeBus-0.10.0-windows-x64.msi -ExpectedVersion 0.10.0
```

On macOS run:

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/test-lifecycle-hooks.sh
./scripts/test-macos-keychain.sh
./scripts/package-plugin-macos.sh
./scripts/validate-plugin-macos.sh \
  ./dist/staging/VibeBus-0.10.0-macos-arm64/plugins/vibebus
```

The project truth is in `README.md`, `docs/architecture.md`, `docs/protocol.md`, and `docs/acceptance.md`.

## Known boundaries

- Windows and macOS have native binaries and OS-vault backends. macOS Hooks run the native binary; Linux remains the explicit-token container path without a secure-vault backend.
- Plugin MCP calls must pass an explicit absolute project `root`.
- Agent bearer tokens and recovery keys can be stored under `VibeBus:<project-id>:<agent>` in Windows Credential Manager or macOS Keychain and omitted from later calls. This protects at rest and against accidental repository/task disclosure, but all processes already running as the same OS user remain inside the trust boundary.
- Message polling occurs at safe task boundaries. There is no promise of interrupting an active generation.
- `context sync` continuation is deterministic and monotonic for a stable database state, but it is not an atomic snapshot across concurrent writes; restart without a cursor when a fresh view is required.
- Responsibility policy is an application authorization boundary for VibeBus-declared reservations, task artifacts, and Git facts, not an operating-system filesystem sandbox. Raw writes made outside VibeBus remain external evidence.
- PostToolUse records only reliable exit status, commit identity/subject/path lists, and bounded test summaries. It never reads transcript, diff, or raw logs. Specialized host tools may opt out of lifecycle Hooks, so Hooks are observable automation rather than a complete enforcement boundary.
- Stop writes a bounded proposal under plugin data and never sends a message, chooses a recipient, blocks completion, or undoes side effects. Changed Hooks require Codex trust review before execution.
- The accepted desktop Agent vault entries `desktop-a-20260717-01` and `desktop-b-20260717-01` remain stored intentionally for regression. Deleting them is an explicit, separate local-vault action and does not remove database audit history.
- Replay-safe subscription peek/ack provides at-least-once batch access, not exactly-once consumer side effects. Legacy poll still consumes and commits in one call.
- Retention removes bounded logical history but never runs SQLite `VACUUM` automatically. `maintenance compact --backup <new-path>` is the separate CLI-only path: real-terminal `compact:<project-id>`, vault-backed Operator, total project downtime, zero active state, fail-fast exclusive SQLite, new verified backup, free-space gate, WAL restore, audit events, and final verification are mandatory. It is absent from MCP.
- Remote synchronization is not implemented; retention state and confirmation plans are local to one project database.
- The optional best-effort bridge to native Codex thread tools is not implemented.
- Destructive retention is default-deny until a local maintainer initializes the separate operator credential in a real terminal. MCP has no operator mutation tools. Every new apply needs a short-lived approval for the exact plan and current operator generation; completed-run replay remains approval-free and cannot delete twice.
- Operator vault cleanup is explicit and CLI-only: `operator delete-credential` requires a real terminal plus `delete:<project-id>`, removes only the current platform vault entry, and leaves the database credential configured with `ready=false`.
- The current live Mac project intentionally reports `operator.ready=false`. No operator credential was initialized on the user's behalf. The stored target, when explicitly initialized, is `VibeBusOperator:<project-id>` and remains inside the same-OS-user trust boundary.
- Pull-request CI produces Windows unsigned and macOS ad-hoc-signed acceptance packages. Windows production release automation requires both signing Secrets and refuses unsigned publication. macOS public distribution still requires Developer ID signing, hardened runtime, notarization, and downloaded Gatekeeper acceptance. The repository has not been given those production credentials, a tag, or a real release during adaptation.
- The installer is intentionally per-user and does not mutate Codex configuration through custom actions. The installed marketplace must be registered explicitly.
- WiX 4.0.6 is pinned to avoid automating WiX 7 OSMF EULA acceptance. See `docs/release.md` before changing the installer toolchain.
- Windows CLI callers should use `artifact publish --metadata-file` for complex JSON; MCP accepts metadata as a native object.

## Recommended next slice

1. Configure the protected `release` environment and real Windows/Apple signing credentials, then execute the platform-specific signed asset, notarization, disposable-profile, downloaded-checksum, and Gatekeeper/Authenticode acceptance in `docs/release.md`.
2. Define stale-Agent/offline visibility and identity lifecycle guidance; decide separately whether the retained desktop A/B vault entries and intentionally unconfigured live Operator state should change.
3. Evaluate an optional Codex task notification bridge only as best-effort UI delivery over the authoritative SQLite Inbox. Remote synchronization, Supervisor scheduling, and automatic merge remain separate product decisions.

## Startup prompt

```text
Read README.md, docs/macos.md, docs/architecture.md, docs/protocol.md, docs/acceptance.md, docs/plan-gap-analysis.md, docs/release.md, and docs/HANDOFF.md. Verify the current checkout, installed VibeBus 0.10.0 macOS cache, and fresh Mac-local runtime, then run formatting, 39 Rust tests, Clippy, the native 7/7 lifecycle-Hook fixture, disposable 11/11 Agent/Operator Keychain fixture, macOS package validation, and checksums before changing code. Start a new Codex task so changed MCP/Hook definitions load, and review/trust changed Hooks. Treat the fixed Windows desktop-20260717-01 fixture as consumed accepted evidence; do not rerun or reuse it. Preserve independent Codex top-level tasks, the single SQLite source of truth, strict responsibility policy, narrow task-scoped overrides, reservation conflicts, immutable Git/test facts, context budgets/cursors, and Stop's proposal-only boundary. Prefer storeCredentials=true, confirm credential status, and never place bearer, recovery, Operator, PFX, Apple signing/notary credential, certificate password, transcript, diff, raw test-log content, or local databases in repository, task, event, message, or Hook facts. Never apply retention without a fresh backup, reviewed plan, and user-performed interactive Operator approval. Never compact a live project through automation: stop every project user, test a restored disposable copy, require a new verified backup and real-terminal compact:<project-id> confirmation, and keep compaction absent from MCP. Never publish an unsigned or merely ad-hoc-signed production release or accept third-party legal terms on behalf of the repository owner. Prefer subscription peek/ack, keep consumer side effects idempotent, and do not claim that VibeBus can interrupt an already-running model generation.
```
