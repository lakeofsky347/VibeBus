# Acceptance record

Acceptance date: 2026-07-17.

## Automated checks

The repository is accepted with:

```powershell
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
./scripts/build-release.ps1
./scripts/test-installer.ps1 -MsiPath ./dist/VibeBus-0.8.0-windows-x64.msi -ExpectedVersion 0.8.0
python C:\Users\17430\.codex\skills\.system\plugin-creator\scripts\validate_plugin.py D:\MyProjects\CoWork\plugins\vibebus
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
- SQLite integrity, WAL, foreign keys, schema version, and online backup;
- ancestor project discovery;
- single-use recovery-key rotation, legacy-agent migration, and invalidation of old secrets;
- project-scoped credential-vault isolation, explicit/environment/vault token precedence, successful secret redaction, rotation write-back, deletion, and safe write-failure fallback;
- owner-only reservation renewal and retry-safe reservation operations;
- concurrent idempotent message retries, payload-drift conflicts, and artifact content identity;
- ordered event filtering, durable subscription cursors, and repeated empty polls;
- replay-safe pending delivery, repeated peek identity, concurrent peek convergence, concurrent/idempotent ACK, empty filtered ranges, wrong-ID conflict, and legacy-poll exclusion;
- retention preview/apply confirmation, pending-delivery protection, stale-plan rejection, concurrent replay-safe apply, retained-history cursor rejection, and snapshot cursor clamping;
- separately hashed operator credentials, generation-bound plan approval, 60–3,600 second TTL validation, expiry rejection, rotation invalidation, atomic single consumption, and successful-run replay without a second approval;
- operator vault target isolation, successful secret redaction, generation refresh, safe write-failure fallback, restoration/deletion verification, and rejection of noninteractive CLI mutations including credential deletion;
- age-bounded cleanup for idempotency records, closed message receipts, orphaned messages, and terminal task/thread history;
- retry-safe structured handoff, ACK lifecycle, and authenticated resume snapshot;
- CLI end-to-end subscription/handoff, message/thread lifecycle, and retention plan/apply flows;
- MCP initialize negotiation, expanded tool listing, explicit absence of operator mutation tools, stored registration, no-token inbox access, vault-backed recovery, unapproved retention rejection, credential deletion, and rejection after deletion.

The suite contains 30 tests: 5 CLI workflows, 20 core workflows, 4 credential-vault workflows, and 1 MCP protocol workflow. All pass on the accepted 0.8 checkout together with formatting and clippy-as-error checks.

The 0.8 release layer additionally covers Cargo/plugin version agreement, repository-owned and Codex plugin validation, pinned release tools, per-user MSI ICE validation, administrative extraction, required payload presence, extracted binary execution/version, portable/plugin archives, post-build SHA-256 checksums, and a machine-readable signed-state manifest. Production publishing remains configured to fail before packaging when either signing Secret is absent.

The accepted local 0.8 package is intentionally unsigned because no production certificate was placed in the workspace or process environment. Its MSI reports `NotSigned`; this verifies the PR/CI acceptance state, not the production signing path. Real certificate timestamping and final install/uninstall on a disposable Windows user profile remain maintainer release acceptance items.

## Plugin checks

- Manifest and component paths pass the plugin validator.
- `.mcp.json` launches `./bin/vibebus.exe mcp` from the plugin root.
- SessionStart is read-only and requires normal Codex hook trust review.
- The Skill states the root, `storeCredentials=true`, vault-status and failure-fallback handling, snapshot, message close lifecycle, task/thread association, operator-approved retention discipline, replay-safe peek/ACK, legacy polling, claim, renewal, idempotency, handoff, conflict, and non-interruption boundaries.
- The repository plugin manifest is version 0.8.0 and passes both the repository validator and the Codex plugin/Skill validators.
- The development reinstall uses cachebuster version `0.8.0+codex.20260717124544`; `codex plugin list` reports it installed and enabled from `vibebus-local`.
- The installed and packaged binaries both report `vibebus 0.8.0` and share SHA-256 `f2809d9828d571a14649929cd59c348ea9babc0fa8141dece90356330f2f47e7`; the installed Skill contains the operator-approval and explicit operator-credential-deletion rules, and the installed CLI exposes `operator delete-credential`.

## Release package acceptance

The accepted unsigned local artifacts are:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `VibeBus-0.8.0-windows-x64.msi` | 2,039,808 | `0cccfbffd79789f0c4349625b5020d3db340bf798d9cea6fe7198404718021e4` |
| `VibeBus-0.8.0-windows-x64.zip` | 2,662,643 | `6c0ca1d21088377c24ce5e5f338aa6f8e80788f0e111ac2359506b1914db1220` |
| `VibeBus-Codex-plugin-0.8.0.zip` | 2,656,496 | `144b7046775e163673befbf0ce2277fdc55ad0e16d736a3fb651c47594c65943` |

The release manifest records `signed=false`. The MSI passes all applicable stock ICEs, administrative extraction returns Windows Installer exit code 0, seven critical payload paths are present, and the extracted binary reports 0.8.0. The missing-signing-secret test rejects signing before any temporary PFX is created. YAML, JSON, and PowerShell AST parsing all pass.

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

## Remaining manual acceptance

1. Execute a signed production release and disposable-profile installer test after a real certificate and protected release environment are available.
2. Separately decide whether the retained desktop A/B Windows vault entries should remain as regression identities or be explicitly deleted. This is not a product acceptance blocker.
