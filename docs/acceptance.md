# Acceptance record

Acceptance date: 2026-07-19. macOS adaptation acceptance: 2026-07-20.

## Automated checks

The repository is accepted with:

```powershell
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/test-lifecycle-hooks.ps1
./scripts/build-release.ps1
./scripts/test-installer.ps1 -MsiPath ./dist/VibeBus-0.10.0-windows-x64.msi -ExpectedVersion 0.10.0
./scripts/validate-plugin.ps1
```

The macOS ARM64 path is accepted with:

```sh
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/test-lifecycle-hooks.sh
./scripts/test-macos-keychain.sh
./scripts/package-plugin-macos.sh
./scripts/validate-plugin-macos.sh \
  ./dist/staging/VibeBus-0.10.0-macos-arm64/plugins/vibebus
(cd dist && shasum -a 256 -c SHA256SUMS-macos-arm64.txt)
```

Covered behaviors:

- directed inbox isolation and token rejection;
- read/ACK receipt state;
- recipient-only message closing, ACK-before-close, hidden-by-default history, and retry-stable close events;
- dependency locking and automatic unlock;
- optimistic task versions and owner-only updates;
- exactly one winner under concurrent claim;
- owner-only task/thread binding, idempotent bind/unbind, terminal auto-unbind, and exactly one concurrent binding winner;
- overlapping reservation conflict and release;
- absolute-path rejection and task transition rules;
- artifact project scope, SHA-256, metadata, and task filtering;
- immutable task-owner confirmed decisions, semantic-key exact replay, payload-drift conflicts, artifact/task validation, idempotency, and audit events;
- authenticated Agent context isolation across active owned tasks, direct dependencies, unread directed messages, relevant decisions/artifacts, bounded previews, byte/item budgets, and cursor pagination;
- SQLite integrity, WAL, foreign keys, schema version, and online backup;
- backup-first offline compaction on disposable data, WAL restoration, page reclamation, audit events, active-state refusal, busy-database fail-fast behavior, and redirected-input rejection before mutation;
- ancestor project discovery;
- single-use recovery-key rotation, legacy-agent migration, and invalidation of old secrets;
- project-scoped credential-vault isolation, explicit/environment/vault token precedence, successful secret redaction, rotation write-back, deletion, and safe write-failure fallback;
- owner-only reservation renewal and retry-safe reservation operations;
- strict responsibility-policy parsing, effective role inspection, task-scoped expiring override authorization, and reservation/artifact/Git-path enforcement;
- immutable Git commit and test-result semantic replay, drift conflict, bounded payloads, report-artifact validation, context projection, and review-only handoff proposals;
- concurrent idempotent message retries, payload-drift conflicts, and artifact content identity;
- ordered event filtering, durable subscription cursors, and repeated empty polls;
- replay-safe pending delivery, repeated peek identity, concurrent peek convergence, concurrent/idempotent ACK, empty filtered ranges, wrong-ID conflict, and legacy-poll exclusion;
- retention preview/apply confirmation, pending-delivery protection, stale-plan rejection, concurrent replay-safe apply, retained-history cursor rejection, and snapshot cursor clamping;
- separately hashed operator credentials, generation-bound plan approval, 60–3,600 second TTL validation, expiry rejection, rotation invalidation, atomic single consumption, and successful-run replay without a second approval;
- operator vault target isolation, successful secret redaction, generation refresh, safe write-failure fallback, restoration/deletion verification, and rejection of noninteractive CLI mutations including credential deletion;
- age-bounded cleanup for idempotency records, closed message receipts, orphaned messages, and terminal task/thread history;
- retry-safe structured handoff, ACK lifecycle, and authenticated resume snapshot;
- CLI end-to-end responsibility/fact/proposal, subscription/handoff, message/thread lifecycle, and retention plan/apply flows;
- MCP initialize negotiation, responsibility/fact/proposal tools, explicit absence of operator mutation tools, stored registration, no-token inbox access, vault-backed recovery, unapproved retention rejection, credential deletion, and rejection after deletion;
- seven deterministic PowerShell and seven native macOS lifecycle-Hook checks covering path-only Git facts, no-log test facts, unknown exit refusal, review-only Stop proposals, and plugin configuration;
- disposable macOS Keychain storage, redaction, vault-backed inbox access, recovery rotation, deletion, rejection after deletion, and cleanup.

The suite contains 39 tests: 1 policy unit test, 1 native-Hook unit test, 7 CLI workflows, 25 core workflows, 4 credential-vault workflows, and 1 MCP protocol workflow. All pass on the accepted checkout together with formatting, clippy-as-error, Windows 7/7 lifecycle-Hook checks, and macOS 7/7 native-Hook checks.

The 0.10 release layer additionally covers Cargo/plugin version agreement, repository-owned Codex plugin validation, pinned release tools, per-user MSI ICE validation, administrative extraction, all three Hooks and four Hook scripts, extracted binary execution/version, portable/plugin archives, post-build SHA-256 checksums, and a machine-readable signed-state manifest. Production publishing remains configured to fail before packaging when either signing Secret is absent.

The accepted local 0.10 package is intentionally unsigned because no production certificate was placed in the workspace or process environment. Its MSI reports `NotSigned`; this verifies the PR/CI acceptance state, not the production signing path. Real certificate timestamping and final install/uninstall on a disposable Windows user profile remain maintainer release acceptance items.

## Plugin checks

- Manifest and component paths pass the plugin validator.
- `.mcp.json` launches `./bin/vibebus.exe mcp` from the plugin root.
- The staged macOS `.mcp.json` launches `./bin/vibebus mcp`; the three Unix Hook commands launch the same binary's hidden native Hook entrypoints.
- SessionStart is read-only; PostToolUse stores only bounded Git/test facts; Stop writes a review-only proposal and never sends. All require normal Codex Hook trust review after definition changes.
- The Skill states responsibility inspection/override, task-scoped reservations, Git/test no-diff/no-log facts, proposal-versus-send, root/vault, retention, delivery, conflict, and non-interruption boundaries.
- The repository plugin manifest is version 0.10.0 and passes the repository validator.
- `codex plugin add vibebus@vibebus-local` refreshed the development install; `codex plugin list` reports version 0.10.0 installed and enabled from `vibebus-local`.
- The previously installed development cache remains accepted at `vibebus 0.10.0` with SHA-256 `fcc312a74af2b1f54900839001b92a98ff44e9e8809179a8551da4409c0321f0`. The current repository/plugin package binary includes offline compaction, reports the same version, is 7,796,736 bytes, and has SHA-256 `71e2ea693f5d8da0ce38ffe57cf1b5ae3f6663613f20935fe8e394602c158f71`.

## Release package acceptance

The accepted unsigned local artifacts are:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `VibeBus-0.10.0-windows-x64.msi` | 2,789,376 | `93356bd9e068e0b1736de5ebd0909bb74e4167a161a4dd404e91ce7c7b515a7d` |
| `VibeBus-0.10.0-windows-x64.zip` | 3,474,202 | `b07842222f3f74ca1ca5e9c75dd022ff8e55877d0d6eb4b018e3916d834203bd` |
| `VibeBus-Codex-plugin-0.10.0.zip` | 3,469,334 | `638680d50829a1ab2d4fa69cff663d5d70179a4a064292d4652f782dc106b3be` |

The release manifest records `signed=false`. The MSI passes all applicable stock ICEs, administrative extraction returns Windows Installer exit code 0, ten critical payload paths are present, and the extracted binary reports 0.10.0. The missing-signing-secret test rejects signing before any temporary PFX is created. YAML, JSON, and PowerShell AST parsing all pass.

## macOS adaptation acceptance

The accepted environment is macOS 26.5.2 on Apple Silicon `arm64` with Xcode Command Line Tools and Rust 1.97.1. The unchanged pre-adaptation core first passed all 38 existing tests. After adding the native Hook unit coverage, all 39 tests pass with formatting and Clippy-as-error.

The Security.framework backend reports `macos-keychain`. A disposable real-Keychain run passed 11/11 checks: stored Agent registration returned redacted metadata, status reported generation 1, inbox access succeeded without an explicit token, recovery rotated and stored generation 2 without exposing either secret, deletion reported absent state, and a later no-token inbox call failed. A real pseudo-terminal then initialized a redacted generation-1 Operator, proved database/Keychain readiness, rotated it through the vault to generation 2, and explicitly deleted the Operator entry with `ready=false`. The fixture deleted both Keychain entries and its temporary project/data directories.

The repository's tracked project marker was retained while Windows runtime state and credentials were not copied. Fresh Mac-local state was created under `~/Library/Application Support/dev.VibeBus.VibeBus`; `doctor` reports schema 11, WAL, foreign keys, integrity `ok`, and overall `ok=true`. Final-build Agent `macos-adaptation-final` is stored in Keychain at target `VibeBus:prj_51ac137e4aa342a7a80bda77d94cfbc5:macos-adaptation-final`, generation 1, with no secret exposed. The installed-cache binary reads that entry and its inbox successfully. An earlier `macos-adaptation` entry remains bound to a superseded ad-hoc binary; the final CLI rejects it in 0.01 seconds with an actionable authorization error instead of opening UI or hanging. The live project Operator remains unconfigured.

The fresh Mac runtime completed a self-directed required-ACK lifecycle. Message `msg_38a636d8c0ea47ba878241fd607e63f5` was created at `1784516524422`, read at `1784516531123`, ACKed at `1784516536871`, and closed at `1784516541824`. It is absent from the normal inbox, present in explicit closed history with all timestamps stable, and `doctor.ok=true` remains intact.

Native SessionStart output was accepted against the real project. The deterministic PostToolUse/Stop fixture passed 7/7 and proves the same bounded no-diff/no-log/review-only properties as the Windows scripts. The installed `vibebus@vibebus-local` cache is enabled at 0.10.0; source, staged, and installed Mach-O hashes match, the installed binary passes `codesign --verify`, and the installed binary returns a healthy `doctor` result.

The accepted local macOS artifacts are ad-hoc signed, not production-notarized:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `VibeBus-0.10.0-macos-arm64.tar.gz` | 3,341,874 | `f99a94473d975c131bc3a1120c95a1ace2a741a37dd3f2b7dd4a63650f61e2b6` |
| `VibeBus-Codex-plugin-0.10.0-macos-arm64.zip` | 3,336,833 | `a76f1d2dd4e9c6114ac87ff41d8b1988c859588e65fc3a4ada3fc78bb2084c61` |

Both checksum entries verify. The packaged Mach-O is a thin `arm64` executable, 6,603,024 bytes, with SHA-256 `2446edc1f2f39819e344b41c76fe2d4e61d8411e6d443e7d1d13dd02ffe9babb`; `codesign` reports a valid ad-hoc signature with identifier `dev.vibebus.cli`. Developer ID signing, notarization, stapling, and downloaded-quarantine Gatekeeper acceptance remain external production gates.

## Live project migration

The existing project database was opened by the 0.4 release binary and migrated in place from schema 6 to schema version 7. `doctor` reports integrity `ok`, WAL journal mode, foreign keys enabled, and overall `ok=true`. The migration explicitly adds `message_receipts.closed_at`, creates task/thread binding history, and preserves the existing project identity and records.

A live project subscription then received a real `task_updated` event. Two peeks returned the same delivery ID without advancing the committed cursor; the first ACK advanced it, the repeated ACK returned `replayed=true`, and the listed subscription had no remaining pending delivery.

The live `MESSAGE-LIFECYCLE-001` task was moved to `working` and bound to the current native Codex task ID. The authenticated handoff snapshot returned that binding. A self-directed `requiresAck` message rejected an early close, accepted ACK then close, returned the same `closedAt` on retry, disappeared from the default inbox, and remained visible only when closed history was explicitly requested.

The 0.5 release binary then migrated the same live project from schema 7 to schema 8. Before any retention apply, SQLite online backup created `vibebus-0.5-pre-retention.db` with SHA-256 `f3035b043f44a8e11893f2f44e963ce875e0450d1ce1484296ddaaae5b1020ed`.

The default live retention preview reported latest event sequence 91, one subscription with slowest safe cursor 37, no pending delivery, retained floor 0, and zero candidates in all five cleanup domains. Applying that exact plan deleted nothing, appended one `retention_applied` audit event, and left `doctor.ok=true`; retrying the same plan returned the original timestamp with `replayed=true`. The accepted post-run backup `vibebus-0.5-final.db` has SHA-256 `4e100e59647f54428716744336fddb5728e5b165b264551fa34ed8c1a631a4c3`.

The 0.6 release binary then completed a real Windows Credential Manager acceptance against a disposable project. Stored registration returned `secretsRedacted=true` with neither secret field; an inbox read succeeded without a token; recovery omitted the recovery key, advanced the generation to 2, and remained redacted; recovery-key provisioning also used and refreshed the vault; `doctor.ok=true`; explicit deletion removed the entry; and the next no-token inbox call failed with exit code 1. The disposable OS credential and its verified workspace-local test directory were removed afterward.

The existing live coordination identity was also migrated with stored recovery-key provisioning. `vibebus_credential_status` reports backend `windows-credential-manager`, target `VibeBus:prj_51ac137e4aa342a7a80bda77d94cfbc5:git-publisher-019f6eab`, `stored=true`, and no secret was exposed during migration.

The accepted post-migration online backup `vibebus-0.6-final.db` is 290,816 bytes with SHA-256 `86a77a9e07bf4d0246223c912fe7bc54d3d97c7568ad307086749f9f7233fe2f`.

Before the 0.8 live migration, the 0.7 binary created `vibebus-0.8-pre-migration.db` (372,736 bytes, SHA-256 `78d9479ec2b394cc247e6078aac89beea7fe615942aa367e6b1d848ea4a58ee5`). The packaged 0.8 binary then opened the same project in place. `doctor` reports schema version 9, integrity `ok`, WAL, foreign keys enabled, and `ok=true`; all eight tasks, five Agents, existing artifacts, the current task/thread binding, and active reservations remained present.

Before the 0.9 live migration, the installed 0.8 service created `vibebus-0.9-pre-migration.db` (630,784 bytes, SHA-256 `9780a050226e1add79770205ec36b6f0a3b643ee81d66815984c393f31060681`). The packaged 0.9 binary then migrated that same project in place from schema 9 to schema 10. `doctor` reports integrity `ok`, WAL, foreign keys enabled, and `ok=true`; pre-existing Agents, 18 tasks, 54 artifacts, bindings, reservations, messages, subscriptions, events, and retention state remained readable.

The live schema-10 acceptance confirmed decision `context-sync.v09.design` for `CONTEXT-SYNC-001`. An exact retry returned the original decision ID `dec_6c2e02ee47104c41b4d0c64a6584b05e`, while automated tests reject payload drift under the same semantic key. A vault-authenticated `context sync` returned the active owned task, its related confirmed decision, and scoped evidence as 15 deterministic items using 8,991 serialized item bytes with no continuation required; unrelated project facts were excluded. CLI/core serialization parity, item and byte budgets, opaque cursor continuation, wrong-token rejection, ACK/read/close exclusion, direct-dependency expansion, and artifact-reference-only behavior are covered by the 32-test suite.

The accepted post-feature online backup `vibebus-0.9-context-sync.db` is 647,168 bytes with SHA-256 `b273c8f9425cbb210d3ac6e66a9a1b1fa56f6d9cf476edb93b1d51a29301331a`. The packaged 0.9 `doctor` reports schema 10, one confirmed decision, integrity `ok`, WAL, foreign keys enabled, and overall `ok=true`.

The schema-10 `vibebus-0.9-context-sync.db` remains the accepted recovery point preceding 0.10. The packaged 0.10 binary migrated the live database in place to schema 11, adding reservation task association, responsibility overrides, Git commit facts, and test-result facts. `doctor` reports integrity `ok`, WAL, foreign keys enabled, 9 Agents, 19 tasks, 62 pre-0.10 artifacts, one confirmed decision, and overall `ok=true`. The vault-backed implementation identity reports `stored=true`, `hasRecoveryKey=true`, and token generation 1. A real owner-authenticated override for `installer/**` and a task-scoped reservation for `installer/Package.wxs` proved the policy boundary without widening the role configuration.

The refreshed installed 0.10 PostToolUse script resolved the active native-task binding from the session ID, used the Windows vault without exposing a token, and recorded one bounded `cargo test` pass while explicitly discarding command output. After commit `e1297e42ede568aed921c6c6ec144ad7e5f86aea`, the same script recorded one Git fact containing the subject and 33 normalized changed paths only; it read no diff. `handoff propose` then returned the task plus exactly one Git fact and one test fact without creating a message. The accepted post-feature backup `vibebus-0.10-responsibility-hooks.db` is 790,528 bytes with SHA-256 `21fcd08e335caa1cabb68e898c45fe578adf672ea2c5f417cbfd50e11f24e669`; the source archive at that implementation commit is 3,002,860 bytes with SHA-256 `63d74278c99c838e4c4b1ca6ec0ef1b18fad1550182f492e06f02d02d2b773c9`.

`operator status` on the live project reports DB configuration `false`, vault storage `false`, and `ready=false` with the isolated target `VibeBusOperator:prj_51ac137e4aa342a7a80bda77d94cfbc5`. No operator credential was initialized on the user's behalf. This proves the safe migration/default-deny state; interactive initialization, rotation, restoration, and a real operator-approved live cleanup remain explicit maintainer actions.

## Real-terminal operator acceptance

The complete operator lifecycle was accepted on disposable schema-9 project `prj_dafdc8aab7584786850b6d73097111d1` without changing the live project's operator state. Interactive initialization stored a redacted generation-1 operator credential and produced matching database/vault generations with `ready=true`.

The reviewed zero-candidate plan `rtp_cb6234128dfa5a16fb83c387144672bb8d87fecf255bb7f09b0ef95090d28452` was approved as `rap_9611d72e24c64ab6a09168554fc1c3af`. The first Agent apply returned `replayed=false`; retrying the same plan returned `replayed=true`; both shared `appliedAt=1784296232174`, and the approval was consumed exactly once. `retention status` recorded that plan and timestamp while `doctor.ok=true` remained intact.

A fresh zero-candidate plan `rtp_fd2b64c50ff188777e2d8255c2918f827a39bc867df3300845c8208c76533bdd` was approved at generation 1 but deliberately not applied. Interactive rotation advanced the operator and vault to generation 2 with `ready=true`. The unchanged plan then failed Agent apply with `operator_approval_required`; the successful retention-run count remained one, proving old-generation approval invalidation.

Explicit Agent credential deletion returned `deleted=true` and `stored=false`. Interactive `operator delete-credential` returned `deleted=true`, retained database configuration at generation 2, removed the vault entry, and produced the required final `configured=true`, `stored=false`, `ready=false` state. Four consistent recovery points cover pre-operator, pre-retention, pre-rotation, and pre-cleanup states; their hashes and the full non-secret evidence inventory are recorded in `docs/operator-acceptance.md`. The verified disposable project/data directories were removed afterward while those ignored recovery artifacts were retained.

## Desktop start-state preflight

`scripts/preflight-desktop-acceptance.ps1` now verifies the fixed two-real-task fixture before either user-owned top-level task is created. It is read-only and fails closed on fixture identity drift, Agent or vault residue, task state/version/dependency drift, binding history, a live acceptance reservation, schema or Operator drift, backup size/hash drift, a dirty checkout, or a `HEAD`/upstream mismatch.

Windows PowerShell 5.1 accepted the pristine live fixture with `-SkipGit` during development: 68 checks, 66 passed, 0 failed, and 2 explicitly skipped Git gates. A deliberate wrong run ID exited 1 with one failed fixture-identity check. The final clean-checkout preflight ran without `-SkipGit` at `2026-07-18T05:21:07Z` and passed all 68 checks with zero failures and zero skips. During preparation, the script was hardened to preserve empty JSON lists as PowerShell arrays so the active-reservation gate cannot silently disappear under Windows PowerShell 5.1.

## Two-real-task desktop acceptance

The fixed desktop run `desktop-20260717-01` completed on 2026-07-18 using two independent, user-owned Codex top-level tasks:

- Task B `019f73ad-0618-76a1-9c42-e17a8fda1486` executed B1 and B2 as `desktop-b-20260717-01`.
- Task A `019f73af-839c-7b03-a62b-09fd7eb07ec0` executed A1 and A2 as `desktop-a-20260717-01`.
- The two task IDs differ, while each Agent's readiness/result or claim/finalize bindings reuse that Agent's real native task ID and are closed after terminal task completion.

Both Agent credentials are stored in Windows Credential Manager with recovery-key retention and redacted responses. B completed `DESKTOP-B-READY-001`, created subscription `desktop-20260717-01`, and stopped at the required `READY_FOR_A` gate. A then claimed and completed `DESKTOP-CLAIM-001`, acquired reservation `rsv_8c26986f53cc46729b68f0ec3877d782`, renewed its expiry from `1784353036068` to `1784353342645`, sent the structured A-to-B handoff `msg_42112ccc95584a26988c263ae60ed0b2`, and stopped at `WAITING_FOR_B_RESULT`.

B's second phase proved replay-safe delivery `sdl_d287418ff18a4ef78d732a8c413afa6d`: repeated peek returned the same delivery, first ACK returned `replayed=false`, retry returned `replayed=true`, and the subscription ended with no pending delivery. B also proved that a competing claim and overlapping reservation both return `conflict` without changing A's ownership. B completed `DESKTOP-B-RESULT-001` and sent result handoff `msg_71fff6874eff46078a8ae895f975feff`. A ACKed and closed that handoff, released the reservation, completed `DESKTOP-A-FINALIZE-001`, and sent final root handoff `msg_438b66a994574f6393b977f7d958beec`.

Before the root recipient mutated that final handoff or edited the repository, `scripts/audit-desktop-acceptance.ps1` ran from a clean pushed checkout at `2026-07-18T05:39:45Z` and passed all 178 checks with zero failures and zero skips. The root then ACKed the final handoff at `1784353201423` and closed it at `1784353206244`. `doctor.ok=true`, the Operator remains unconfigured, no acceptance reservation remains, and the accepted post-run online backup `backups/vibebus-0.8-desktop-acceptance.db` is 589,824 bytes with SHA-256 `5928201fd62fa0d5a7588a91650bfaf86ace173f0c43f2b10eb9f4c8f232d37b`.

The two desktop Agent vault entries remain stored intentionally for repeatable regression and were not deleted implicitly. Their deletion, if desired, is a separate explicit local-vault decision; the database Agent, task, message, subscription, and binding history remains authoritative audit evidence either way.

## Linux container acceptance

The repository-owned `Dockerfile` produced the local `linux/amd64` image `vibebus:0.10.0-local` on 2026-07-18. Docker reports size 31,240,216 bytes, working directory `/workspace`, and runtime user `10001:10001`. The local image ID is deliberately not used as the release identity because later clean rebuilds produced different IDs while preserving all accepted runtime properties.

`scripts/test-container.ps1 -SkipBuild` accepted the image from disposable host directories. The runtime reported `vibebus 0.10.0`; project initialization and `doctor` confirmed SQLite WAL mode and foreign keys; registration plus an authenticated empty Inbox succeeded through the inherited `VIBEBUS_AGENT_TOKEN` environment variable without printing credentials; and stdio MCP initialization plus `tools/list` returned 47 tools. The script removed its disposable project/data directories after the run.

The accepted image was pushed to Alibaba Cloud ACR as `crpi-21kb7zn8owb85qa2.cn-beijing.personal.cr.aliyuncs.com/for_plugin/vibebus:0.10.0` at 2026-07-18T12:47:55Z. Both the push response and an independent remote `docker buildx imagetools inspect` returned index digest `sha256:71e39f0a3af75e9626dd6d1c313f1edd3ef65d7446c0a8497147043036227118`. A verbose remote manifest inspection confirmed the runnable `linux/amd64` manifest `sha256:8f43d9c7ae26c9eaedc3746b5f1e60c21737fef0d2cc45e579b3ed01a5d4eb94`; the index also contains the BuildKit provenance/attestation manifest.

Repository intake reran the complete build and acceptance after hardening native stderr handling for Windows PowerShell. Repeated local builds again passed all seven container gates with the same size, runtime user, version, WAL/foreign-key state, and 47-tool MCP surface, while their local image IDs differed because build metadata is not bit-for-bit reproducible. They were not pushed and do not replace the recorded remote digest. CI now runs the same repository script on `ubuntu-latest`; both test and push helpers derive their default version/tag from `Cargo.toml` to fail closed on release-version drift.

## Backup restore drill

`scripts/test-backup-restore.ps1` now exercises online backup, export identity, isolated import, point-in-time recovery, authenticated reads, and cleanup without touching the live project. The local 2026-07-18 run created disposable project `prj_b9a1ffd18c8942cc93829c23163e369b`; its 278,528-byte backup and imported copy both hashed to `8040cf3ffafec017bc8c8beffe9829a143cc202cbfea81a797456966e0d53715` before the restored database was opened.

The source contained one Agent and one task at backup time, then received a second task after the backup. The isolated restore reported schema 11, WAL, foreign keys, one Agent, and exactly the pre-backup task; the post-backup mutation was absent. The original in-process Agent token authenticated an empty restored Inbox without being printed, and the temporary source, export, and restored data trees were removed. Every run uses a new project ID and therefore a new artifact hash; acceptance compares each imported copy with the hash returned by that run's online backup rather than pinning the sample hash.

Windows CI runs the same drill against the just-built release binary before MSI acceptance. The production runbook in `docs/backup-restore.md` requires an isolated verification root and an explicit maintainer-controlled offline cutover; it never automates a live-database overwrite.

## Disposable offline compaction

Three core workflows execute `Bus::compact_offline` only against fresh `TempDir` project/data homes. The success fixture creates and deletes 4 MiB of disposable SQLite payload, then proves a new verified backup, Operator generation binding, `compaction_started`/`compaction_completed` audit facts, freelist reduction to zero, smaller final bytes, distinct 64-character hashes, WAL restoration, schema 11, foreign-key cleanliness, integrity `ok`, and `doctor.ok=true`. Separate fixtures prove that any non-terminal task prevents backup/`VACUUM`, and that an outstanding `BEGIN IMMEDIATE` causes fail-fast conflict without creating the backup.

The CLI workflow invokes `maintenance compact --backup` with redirected input and proves validation failure before database mutation or backup creation. `tools/list` remains unchanged because no compaction MCP tool exists. The real project at `D:\MyProjects\CoWork` was not compacted; its Operator remains intentionally not ready, and the implementation used its database only for VibeBus coordination facts.

## Remaining manual acceptance

1. Execute a signed production release and disposable-profile installer test after a real certificate and protected release environment are available.
2. Separately decide whether the retained desktop A/B Windows vault entries should remain as regression identities or be explicitly deleted. This is not a product acceptance blocker.
