# Operator real-terminal acceptance

This runbook verifies the project-scoped operator boundary on a disposable VibeBus project. It must not be run against a live coordination database. Operator initialization, approval, rotation, restoration, and credential deletion are human-presence operations: run them in a maintainer-opened real terminal, never through redirected input, MCP, a CI job, or model-driven shell automation.

The acceptance separates local operator consent from Agent execution. Do not paste an operator secret into a command, task, message, log, or repository file.

## 1. Prepare an isolated project

Choose a disposable root and data home, then initialize the project and store a disposable Agent identity in Windows Credential Manager:

```powershell
$vb = (Resolve-Path .\plugins\vibebus\bin\vibebus.exe).Path
$acceptRoot = 'D:\temp\vibebus-operator-acceptance\project'
$dataHome = 'D:\temp\vibebus-operator-acceptance\data'
$agent = 'operator-acceptance-agent'

New-Item -ItemType Directory -Path $acceptRoot | Out-Null
& $vb init --root $acceptRoot --data-home $dataHome --name 'VibeBus Operator Acceptance'
& $vb register --root $acceptRoot --data-home $dataHome --name $agent --role acceptance --store-credentials
& $vb doctor --root $acceptRoot --data-home $dataHome
& $vb credential status --root $acceptRoot --data-home $dataHome --agent $agent
& $vb operator status --root $acceptRoot --data-home $dataHome
```

Require schema 9, `doctor.ok=true`, stored Agent credentials, and initial operator state `configured=false`, `stored=false`, `ready=false`. Create a consistent recovery point before the first operator mutation:

```powershell
& $vb backup --root $acceptRoot --data-home $dataHome --output 'D:\temp\vibebus-operator-acceptance\pre-operator.db'
```

## 2. Initialize in a real terminal

In a terminal opened by the maintainer, run:

```powershell
& $vb operator init --root $acceptRoot --data-home $dataHome
```

Review the project path and type the complete project ID when prompted. Successful vault storage must redact the secret. Then verify `configured=true`, matching database/vault generation 1, and `ready=true`:

```powershell
& $vb operator status --root $acceptRoot --data-home $dataHome
```

## 3. Back up and plan

Operator initialization changes authoritative state, so create a fresh backup before planning cleanup:

```powershell
& $vb backup --root $acceptRoot --data-home $dataHome --output 'D:\temp\vibebus-operator-acceptance\pre-retention.db'
$plan = & $vb retention plan --root $acceptRoot --data-home $dataHome --agent $agent | ConvertFrom-Json
$plan.result | ConvertTo-Json -Depth 8
```

For the default zero-candidate acceptance, require all five candidate counts to be zero, `pendingDeliveryCount=0`, and a non-empty `rtp_` plan ID. If any value differs, stop and investigate rather than approving a plan by pattern.

## 4. Approve the exact plan

Do not perform any VibeBus write between the final plan and apply. In the maintainer's real terminal, approve the exact ID shown above:

```powershell
$planId = '<exact rtp_ plan ID from the reviewed preview>'
& $vb operator approve-retention --root $acceptRoot --data-home $dataHome --plan $planId
```

Review the full candidate and protection JSON printed by the CLI, then type the complete plan ID. If custom retention flags were used for planning, repeat the identical flags for approval and apply.

## 5. Apply through the Agent boundary and replay

After the maintainer confirms approval, a separately authenticated Agent or MCP caller applies it:

```powershell
$planId = '<exact plan ID approved by the maintainer>'
$applied = & $vb retention apply --root $acceptRoot --data-home $dataHome --agent $agent --plan $planId | ConvertFrom-Json
$replayed = & $vb retention apply --root $acceptRoot --data-home $dataHome --agent $agent --plan $planId | ConvertFrom-Json
& $vb retention status --root $acceptRoot --data-home $dataHome
& $vb doctor --root $acceptRoot --data-home $dataHome
```

Require the first result to report `replayed=false`, the retry to report `replayed=true`, both to share the same `appliedAt`, and `lastPlanId` to equal the approved plan. The approval must be consumed exactly once.

## 6. Verify rotation invalidation

Create and approve a fresh zero-candidate plan, but do not apply it. In the maintainer's real terminal, rotate the operator and type `rotate:<project-id>`:

```powershell
& $vb operator rotate --root $acceptRoot --data-home $dataHome
```

Require `operator status` to report matching generation 2 and `ready=true`. Attempting to apply the previously approved plan must fail with `operator_approval_required`, proving the generation-1 approval cannot survive rotation. A new plan/approval at generation 2 may then be applied if an additional positive-path check is desired.

## 7. Remove disposable credentials

Agent credential deletion is explicit but does not require operator authority:

```powershell
& $vb credential delete --root $acceptRoot --data-home $dataHome --agent $agent
```

In the maintainer's real terminal, remove the disposable operator vault entry and type `delete:<project-id>`:

```powershell
& $vb operator delete-credential --root $acceptRoot --data-home $dataHome
& $vb operator status --root $acceptRoot --data-home $dataHome
```

Require `deleted=true`, `configured=true`, `stored=false`, and `ready=false`. The database digest intentionally remains configured; deleting the disposable data directory removes that final test-only state. Resolve and verify the exact disposable paths before deleting them, and never aim recursive cleanup at a repository root, user profile, drive root, or unresolved environment variable.

## Evidence to retain

Record only non-secret evidence: project ID, schema version, backup hashes, plan IDs, candidate/protection counts, operator generations, approval ID, apply/replay timestamps, expected rotation failure kind, deletion status, and final `doctor` result. Never retain the operator secret, Agent token, recovery key, credential BLOB, or terminal transcript containing secret material.

## Accepted execution evidence (2026-07-17)

The runbook was completed against disposable project `prj_dafdc8aab7584786850b6d73097111d1`; the live coordination project's operator remained unconfigured. The disposable database stayed on schema 9 with `doctor.ok=true`, integrity `ok`, WAL, and foreign keys enabled through the final pre-cleanup check.

Recovery points were created at every authority-changing boundary:

| Recovery point | SHA-256 |
| --- | --- |
| Before operator initialization | `489bacca651372f501fc9894695875ace463c40e339612d81dc760604827dfb2` |
| After initialization, before first approval | `acfc08a0785683483a89d8a919113de2cd8f120799cf22caf4accd4ca490cd26` |
| After apply/replay, before rotation | `19871a156630956651bdd8e3d6b18c73ee68e9377abed9f23ae2724e157a87d5` |
| After rotation invalidation, before credential cleanup | `de6bbe4349be5067db6df43917add61e5290679ee18fe18bca25a90aa1402faf` |

The generation-1 positive path used plan `rtp_cb6234128dfa5a16fb83c387144672bb8d87fecf255bb7f09b0ef95090d28452`. All five candidate counts were zero, pending deliveries and subscriptions were zero, and both safe/latest event sequence values were 1. Interactive approval `rap_9611d72e24c64ab6a09168554fc1c3af` was consumed once. The first Agent apply returned `replayed=false`; its retry returned `replayed=true`; both reported `appliedAt=1784296232174` and the same approval ID.

The rotation-invalidation path used fresh plan `rtp_fd2b64c50ff188777e2d8255c2918f827a39bc867df3300845c8208c76533bdd` and generation-1 approval `rap_04b44c72c6184a0a90e15eac40e07184`. Its five candidate counts were zero, pending deliveries and subscriptions were zero, and both safe/latest event sequence values were 2. After an interactive rotation advanced the database and vault to generation 2 with `ready=true`, applying that unchanged generation-1-approved plan failed with `operator_approval_required`. The successful retention-run count remained one and the invalidated approval remained unconsumed.

Explicit Agent credential deletion returned `deleted=true` and `stored=false`. Interactive operator credential deletion then returned `deleted=true`, retained database state `configured=true` at generation 2, and reported `stored=false` plus `ready=false`. After exact path and parent validation, only the disposable `project` and `data` directories were recursively removed; the four non-secret recovery databases remain as ignored local evidence artifacts.
