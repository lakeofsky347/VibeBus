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
