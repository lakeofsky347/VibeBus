[CmdletBinding()]
param(
    [string]$ProjectRoot = "",
    [string]$DataHome = "",
    [string]$VibeBusPath = "",
    [string]$RunId = "desktop-20260717-01",
    [string]$AgentA = "",
    [string]$AgentB = "",
    [string]$EvidenceRecipient = "git-publisher-019f6eab",
    [switch]$SkipGit
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
    $ProjectRoot = Join-Path $PSScriptRoot ".."
}
$ProjectRoot = (Resolve-Path -LiteralPath $ProjectRoot).Path
if (-not [string]::IsNullOrWhiteSpace($DataHome)) {
    $DataHome = (Resolve-Path -LiteralPath $DataHome).Path
}

if ([string]::IsNullOrWhiteSpace($VibeBusPath)) {
    $VibeBusPath = Join-Path $ProjectRoot "plugins\vibebus\bin\vibebus.exe"
}
$VibeBusPath = (Resolve-Path -LiteralPath $VibeBusPath).Path

if ([string]::IsNullOrWhiteSpace($AgentA)) {
    $AgentA = "desktop-a-20260717-01"
}
if ([string]::IsNullOrWhiteSpace($AgentB)) {
    $AgentB = "desktop-b-20260717-01"
}

$taskIds = [ordered]@{
    Controller = "DESKTOP-ACCEPTANCE-001"
    BReady = "DESKTOP-B-READY-001"
    Claim = "DESKTOP-CLAIM-001"
    BResult = "DESKTOP-B-RESULT-001"
    AFinalize = "DESKTOP-A-FINALIZE-001"
}
$reservationPath = "acceptance/$RunId/shared-resource"
$backupRelativePath = "backups/vibebus-0.8-pre-desktop-acceptance.db"
$script:auditChecks = [System.Collections.Generic.List[object]]::new()

function Add-AuditCheck {
    param(
        [string]$Name,
        [bool]$Passed,
        [string]$Evidence,
        [bool]$Skipped = $false
    )

    $script:auditChecks.Add([pscustomobject]@{
        name = $Name
        passed = $Passed
        skipped = $Skipped
        evidence = $Evidence
    }) | Out-Null
}

function Format-AuditValue {
    param($Value)

    if ($null -eq $Value) {
        return "null"
    }
    if ($Value -is [bool]) {
        return $Value.ToString().ToLowerInvariant()
    }
    return [string]$Value
}

function Add-EqualCheck {
    param(
        [string]$Name,
        $Actual,
        $Expected
    )

    Add-AuditCheck -Name $Name -Passed ($Actual -eq $Expected) -Evidence (
        "expected={0}; actual={1}" -f (Format-AuditValue $Expected), (Format-AuditValue $Actual)
    )
}

function Invoke-VibeBusJson {
    param([string[]]$Arguments)

    $effectiveArguments = @($Arguments)
    if (-not [string]::IsNullOrWhiteSpace($DataHome)) {
        $effectiveArguments += @("--data-home", $DataHome)
    }
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $lines = & $VibeBusPath @effectiveArguments 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousPreference
    $raw = ($lines | ForEach-Object { $_.ToString() }) -join [Environment]::NewLine

    try {
        $json = $raw | ConvertFrom-Json
    } catch {
        return [pscustomobject]@{
            succeeded = $false
            result = $null
            error = "non-JSON output with exit code ${exitCode}: $raw"
        }
    }

    if ($exitCode -ne 0 -or $json.ok -ne $true) {
        $errorText = if ($null -ne $json.error) { [string]$json.error } else { $raw }
        return [pscustomobject]@{
            succeeded = $false
            result = $null
            error = "exit code ${exitCode}: $errorText"
        }
    }

    return [pscustomobject]@{
        succeeded = $true
        result = $json.result
        error = $null
    }
}

function Get-VibeBusResult {
    param(
        [string]$Name,
        [string[]]$Arguments
    )

    $call = Invoke-VibeBusJson -Arguments $Arguments
    $evidence = if ($call.succeeded) { "ok" } else { $call.error }
    Add-AuditCheck -Name "command.$Name" -Passed $call.succeeded -Evidence $evidence
    return $call.result
}

function Convert-DecisionMap {
    param(
        [string]$Name,
        $Body
    )

    $map = [ordered]@{}
    $errors = [System.Collections.Generic.List[string]]::new()
    foreach ($decision in @($Body.decisions)) {
        $parts = ([string]$decision).Split(@("="), 2, [System.StringSplitOptions]::None)
        if ($parts.Count -ne 2 -or [string]::IsNullOrWhiteSpace($parts[0])) {
            $errors.Add("invalid decision '$decision'") | Out-Null
            continue
        }
        if ($map.Contains($parts[0])) {
            $errors.Add("duplicate key '$($parts[0])'") | Out-Null
            continue
        }
        $map[$parts[0]] = $parts[1]
    }
    $formatEvidence = if ($errors.Count -eq 0) { "key=value decisions are unique" } else { $errors -join "; " }
    Add-AuditCheck -Name "$Name.decisions.format" -Passed ($errors.Count -eq 0) -Evidence $formatEvidence
    return $map
}

function Find-Handoff {
    param(
        [string]$Name,
        [object[]]$Messages,
        [string]$Sender,
        [string]$TaskId
    )

    $matches = [System.Collections.Generic.List[object]]::new()
    foreach ($message in @($Messages)) {
        if ($message.sender -ne $Sender) {
            continue
        }
        try {
            $body = ([string]$message.body) | ConvertFrom-Json
        } catch {
            continue
        }
        if ($body.kind -eq "handoff" -and $body.taskId -eq $TaskId) {
            $matches.Add([pscustomobject]@{
                message = $message
                body = $body
            }) | Out-Null
        }
    }

    Add-AuditCheck -Name "$Name.unique" -Passed ($matches.Count -eq 1) -Evidence "matchingMessages=$($matches.Count)"
    if ($matches.Count -ne 1) {
        return $null
    }
    $match = $matches[0]
    $match | Add-Member -NotePropertyName decisions -NotePropertyValue (Convert-DecisionMap -Name $Name -Body $match.body)
    return $match
}

function Get-Decision {
    param(
        [string]$HandoffName,
        $Handoff,
        [string]$Key
    )

    if ($null -eq $Handoff) {
        Add-AuditCheck -Name "$HandoffName.decision.$Key" -Passed $false -Evidence "handoff missing"
        return $null
    }
    $exists = $Handoff.decisions.Contains($Key)
    $decisionEvidence = if ($exists) { "present" } else { "missing" }
    Add-AuditCheck -Name "$HandoffName.decision.$Key" -Passed $exists -Evidence $decisionEvidence
    if (-not $exists) {
        return $null
    }
    return [string]$Handoff.decisions[$Key]
}

function Get-TaskBinding {
    param(
        [object[]]$Bindings,
        [string]$TaskId,
        [string]$ExpectedAgent
    )

    $matching = @($Bindings | Where-Object { $_.taskId -eq $TaskId })
    Add-AuditCheck -Name "binding.$TaskId.unique" -Passed ($matching.Count -eq 1) -Evidence "bindings=$($matching.Count)"
    if ($matching.Count -ne 1) {
        return $null
    }
    $binding = $matching[0]
    Add-EqualCheck -Name "binding.$TaskId.agent" -Actual $binding.agent -Expected $ExpectedAgent
    Add-AuditCheck -Name "binding.$TaskId.thread" -Passed (-not [string]::IsNullOrWhiteSpace([string]$binding.threadId)) -Evidence "threadIdPresent=$(-not [string]::IsNullOrWhiteSpace([string]$binding.threadId))"
    Add-AuditCheck -Name "binding.$TaskId.terminal" -Passed ($null -ne $binding.unboundAt) -Evidence "unboundAt=$(Format-AuditValue $binding.unboundAt)"
    return $binding
}

function Get-ExpectedTask {
    param(
        [object[]]$Tasks,
        [string]$TaskId,
        [string]$ExpectedOwner
    )

    $matching = @($Tasks | Where-Object { $_.taskId -eq $TaskId })
    Add-AuditCheck -Name "task.$TaskId.unique" -Passed ($matching.Count -eq 1) -Evidence "tasks=$($matching.Count)"
    if ($matching.Count -ne 1) {
        return $null
    }
    $task = $matching[0]
    Add-EqualCheck -Name "task.$TaskId.owner" -Actual $task.owner -Expected $ExpectedOwner
    Add-EqualCheck -Name "task.$TaskId.status" -Actual $task.status -Expected "completed"
    return $task
}

$doctor = Get-VibeBusResult -Name "doctor" -Arguments @("doctor", "--root", $ProjectRoot)
$operator = Get-VibeBusResult -Name "operator-status" -Arguments @("operator", "status", "--root", $ProjectRoot)
$agents = Get-VibeBusResult -Name "agents" -Arguments @("agents", "--root", $ProjectRoot)
$credentialA = Get-VibeBusResult -Name "credential-a" -Arguments @("credential", "status", "--root", $ProjectRoot, "--agent", $AgentA)
$credentialB = Get-VibeBusResult -Name "credential-b" -Arguments @("credential", "status", "--root", $ProjectRoot, "--agent", $AgentB)
$tasks = Get-VibeBusResult -Name "tasks" -Arguments @("task", "list", "--root", $ProjectRoot)
$bindings = Get-VibeBusResult -Name "bindings" -Arguments @("thread", "list", "--root", $ProjectRoot, "--all")
$reservations = Get-VibeBusResult -Name "reservations" -Arguments @("reserve", "list", "--root", $ProjectRoot)
$subscriptions = Get-VibeBusResult -Name "subscription-b" -Arguments @("subscription", "list", "--root", $ProjectRoot, "--agent", $AgentB)
$inboxA = Get-VibeBusResult -Name "inbox-a" -Arguments @("inbox", "--root", $ProjectRoot, "--agent", $AgentA, "--all", "--include-closed")
$inboxB = Get-VibeBusResult -Name "inbox-b" -Arguments @("inbox", "--root", $ProjectRoot, "--agent", $AgentB, "--all", "--include-closed")
$inboxRoot = Get-VibeBusResult -Name "inbox-root" -Arguments @("inbox", "--root", $ProjectRoot, "--agent", $EvidenceRecipient, "--all", "--include-closed")
$retention = Get-VibeBusResult -Name "retention-status" -Arguments @("retention", "status", "--root", $ProjectRoot)
$eventFloor = if ($null -ne $retention) { [string]$retention.eventsPrunedThroughSequence } else { "0" }
$events = Get-VibeBusResult -Name "reservation-events" -Arguments @(
    "event", "list", "--root", $ProjectRoot, "--after", $eventFloor, "--limit", "500",
    "--event-types", "paths_reserved,paths_renewed,paths_released"
)
$artifacts = Get-VibeBusResult -Name "controller-artifacts" -Arguments @(
    "artifact", "list", "--root", $ProjectRoot, "--task", $taskIds.Controller
)

if ($null -ne $doctor) {
    Add-EqualCheck -Name "doctor.ok" -Actual $doctor.ok -Expected $true
    Add-EqualCheck -Name "doctor.schema" -Actual $doctor.schemaVersion -Expected 9
    Add-EqualCheck -Name "doctor.integrity" -Actual $doctor.integrity -Expected "ok"
    Add-EqualCheck -Name "doctor.journal" -Actual $doctor.journalMode.ToLowerInvariant() -Expected "wal"
    Add-EqualCheck -Name "doctor.foreignKeys" -Actual $doctor.foreignKeysEnabled -Expected $true
}

if ($null -ne $operator) {
    Add-EqualCheck -Name "operator.configured" -Actual $operator.operator.configured -Expected $false
    Add-EqualCheck -Name "operator.stored" -Actual $operator.credential.stored -Expected $false
    Add-EqualCheck -Name "operator.ready" -Actual $operator.ready -Expected $false
}

foreach ($agentSpec in @(
    @{ Name = $AgentA; Role = "desktop-acceptance-coordinator" },
    @{ Name = $AgentB; Role = "desktop-acceptance-receiver" }
)) {
    if ($null -eq $agents) {
        continue
    }
    $matching = @($agents | Where-Object { $_.name -eq $agentSpec.Name })
    Add-AuditCheck -Name "agent.$($agentSpec.Name).unique" -Passed ($matching.Count -eq 1) -Evidence "agents=$($matching.Count)"
    if ($matching.Count -eq 1) {
        Add-EqualCheck -Name "agent.$($agentSpec.Name).role" -Actual $matching[0].role -Expected $agentSpec.Role
        Add-EqualCheck -Name "agent.$($agentSpec.Name).status" -Actual $matching[0].status -Expected "working"
    }
}

foreach ($credentialSpec in @(
    @{ Label = "a"; Value = $credentialA },
    @{ Label = "b"; Value = $credentialB }
)) {
    if ($null -eq $credentialSpec.Value) {
        continue
    }
    Add-EqualCheck -Name "credential.$($credentialSpec.Label).stored" -Actual $credentialSpec.Value.stored -Expected $true
    Add-EqualCheck -Name "credential.$($credentialSpec.Label).recovery" -Actual $credentialSpec.Value.hasRecoveryKey -Expected $true
    Add-EqualCheck -Name "credential.$($credentialSpec.Label).generation" -Actual $credentialSpec.Value.tokenGeneration -Expected 1
}

if ($null -ne $tasks) {
    $null = Get-ExpectedTask -Tasks $tasks -TaskId $taskIds.BReady -ExpectedOwner $AgentB
    $null = Get-ExpectedTask -Tasks $tasks -TaskId $taskIds.Claim -ExpectedOwner $AgentA
    $null = Get-ExpectedTask -Tasks $tasks -TaskId $taskIds.BResult -ExpectedOwner $AgentB
    $null = Get-ExpectedTask -Tasks $tasks -TaskId $taskIds.AFinalize -ExpectedOwner $AgentA
}

$bindingBReady = $null
$bindingClaim = $null
$bindingBResult = $null
$bindingAFinalize = $null
if ($null -ne $bindings) {
    $bindingBReady = Get-TaskBinding -Bindings $bindings -TaskId $taskIds.BReady -ExpectedAgent $AgentB
    $bindingClaim = Get-TaskBinding -Bindings $bindings -TaskId $taskIds.Claim -ExpectedAgent $AgentA
    $bindingBResult = Get-TaskBinding -Bindings $bindings -TaskId $taskIds.BResult -ExpectedAgent $AgentB
    $bindingAFinalize = Get-TaskBinding -Bindings $bindings -TaskId $taskIds.AFinalize -ExpectedAgent $AgentA
}
if ($null -ne $bindingClaim -and $null -ne $bindingAFinalize) {
    Add-EqualCheck -Name "binding.agent-a.same-thread" -Actual $bindingClaim.threadId -Expected $bindingAFinalize.threadId
}
if ($null -ne $bindingBReady -and $null -ne $bindingBResult) {
    Add-EqualCheck -Name "binding.agent-b.same-thread" -Actual $bindingBReady.threadId -Expected $bindingBResult.threadId
}
if ($null -ne $bindingClaim -and $null -ne $bindingBReady) {
    Add-AuditCheck -Name "binding.two-distinct-top-level-tasks" -Passed ($bindingClaim.threadId -ne $bindingBReady.threadId) -Evidence "distinct=$($bindingClaim.threadId -ne $bindingBReady.threadId)"
}

$acceptanceReservations = @()
if ($null -ne $reservations) {
    $acceptanceReservations = @($reservations | Where-Object { $_.pathPattern -eq $reservationPath })
    Add-EqualCheck -Name "reservation.activeAcceptance" -Actual $acceptanceReservations.Count -Expected 0
}

$handoffAB = if ($null -ne $inboxB) {
    Find-Handoff -Name "handoff.a-b" -Messages @($inboxB) -Sender $AgentA -TaskId $taskIds.Claim
} else { $null }
$handoffBA = if ($null -ne $inboxA) {
    Find-Handoff -Name "handoff.b-a" -Messages @($inboxA) -Sender $AgentB -TaskId $taskIds.BResult
} else { $null }
$handoffRoot = if ($null -ne $inboxRoot) {
    Find-Handoff -Name "handoff.a-root" -Messages @($inboxRoot) -Sender $AgentA -TaskId $taskIds.AFinalize
} else { $null }

foreach ($closedHandoff in @(
    @{ Name = "handoff.a-b"; Value = $handoffAB },
    @{ Name = "handoff.b-a"; Value = $handoffBA }
)) {
    if ($null -eq $closedHandoff.Value) {
        continue
    }
    Add-EqualCheck -Name "$($closedHandoff.Name).priority" -Actual $closedHandoff.Value.message.priority -Expected "high"
    Add-EqualCheck -Name "$($closedHandoff.Name).requiresAck" -Actual $closedHandoff.Value.message.requiresAck -Expected $true
    Add-AuditCheck -Name "$($closedHandoff.Name).acked" -Passed ($null -ne $closedHandoff.Value.message.ackAt) -Evidence "ackAt=$(Format-AuditValue $closedHandoff.Value.message.ackAt)"
    Add-AuditCheck -Name "$($closedHandoff.Name).closed" -Passed ($null -ne $closedHandoff.Value.message.closedAt) -Evidence "closedAt=$(Format-AuditValue $closedHandoff.Value.message.closedAt)"
}
if ($null -ne $handoffRoot) {
    Add-EqualCheck -Name "handoff.a-root.priority" -Actual $handoffRoot.message.priority -Expected "high"
    Add-EqualCheck -Name "handoff.a-root.requiresAck" -Actual $handoffRoot.message.requiresAck -Expected $true
    Add-EqualCheck -Name "handoff.a-root.unacked" -Actual $handoffRoot.message.ackAt -Expected $null
    Add-EqualCheck -Name "handoff.a-root.open" -Actual $handoffRoot.message.closedAt -Expected $null
}

$claimOwner = Get-Decision -HandoffName "handoff.a-b" -Handoff $handoffAB -Key "claimOwner"
$reservationId = Get-Decision -HandoffName "handoff.a-b" -Handoff $handoffAB -Key "reservationId"
$originalExpiryText = Get-Decision -HandoffName "handoff.a-b" -Handoff $handoffAB -Key "originalExpiry"
$renewedExpiryText = Get-Decision -HandoffName "handoff.a-b" -Handoff $handoffAB -Key "renewedExpiry"
Add-EqualCheck -Name "handoff.a-b.claimOwner" -Actual $claimOwner -Expected $AgentA
Add-AuditCheck -Name "handoff.a-b.reservationId" -Passed ($reservationId -match '^rsv_[0-9a-f]+$') -Evidence "validReservationId=$($reservationId -match '^rsv_[0-9a-f]+$')"
$originalExpiry = 0L
$renewedExpiry = 0L
$originalExpiryValid = [int64]::TryParse($originalExpiryText, [ref]$originalExpiry)
$renewedExpiryValid = [int64]::TryParse($renewedExpiryText, [ref]$renewedExpiry)
Add-AuditCheck -Name "handoff.a-b.expiries" -Passed ($originalExpiryValid -and $renewedExpiryValid -and $renewedExpiry -gt $originalExpiry) -Evidence "renewedAfterOriginal=$($renewedExpiry -gt $originalExpiry)"

$deliveryId = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "deliveryId"
$firstAckReplayed = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "firstAckReplayed"
$retryAckReplayed = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "retryAckReplayed"
$subscriptionAckAt = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "subscriptionAckAt"
$aToBAckAt = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "aToBHandoffAckAt"
$aToBClosedAt = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "aToBHandoffClosedAt"
$claimConflictKind = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "claimConflictKind"
$claimOwnerBefore = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "claimOwnerBefore"
$claimOwnerAfter = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "claimOwnerAfter"
$claimStatusBefore = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "claimStatusBefore"
$claimStatusAfter = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "claimStatusAfter"
$reservationConflictKind = Get-Decision -HandoffName "handoff.b-a" -Handoff $handoffBA -Key "reservationConflictKind"
Add-AuditCheck -Name "handoff.b-a.deliveryId" -Passed ($deliveryId -match '^sdl_[0-9a-f]+$') -Evidence "validDeliveryId=$($deliveryId -match '^sdl_[0-9a-f]+$')"
Add-EqualCheck -Name "handoff.b-a.firstAck" -Actual $firstAckReplayed -Expected "false"
Add-EqualCheck -Name "handoff.b-a.retryAck" -Actual $retryAckReplayed -Expected "true"
Add-AuditCheck -Name "handoff.b-a.subscriptionAckAt" -Passed ($subscriptionAckAt -match '^\d+$') -Evidence "timestampPresent=$($subscriptionAckAt -match '^\d+$')"
Add-EqualCheck -Name "handoff.b-a.claimConflict" -Actual $claimConflictKind -Expected "conflict"
Add-EqualCheck -Name "handoff.b-a.claimOwnerBefore" -Actual $claimOwnerBefore -Expected $AgentA
Add-EqualCheck -Name "handoff.b-a.claimOwnerAfter" -Actual $claimOwnerAfter -Expected $AgentA
Add-EqualCheck -Name "handoff.b-a.claimStatusBefore" -Actual $claimStatusBefore -Expected "working"
Add-EqualCheck -Name "handoff.b-a.claimStatusAfter" -Actual $claimStatusAfter -Expected "working"
Add-EqualCheck -Name "handoff.b-a.reservationConflict" -Actual $reservationConflictKind -Expected "conflict"
if ($null -ne $handoffAB) {
    Add-EqualCheck -Name "handoff.b-a.aToBAckAt" -Actual $aToBAckAt -Expected ([string]$handoffAB.message.ackAt)
    Add-EqualCheck -Name "handoff.b-a.aToBClosedAt" -Actual $aToBClosedAt -Expected ([string]$handoffAB.message.closedAt)
}

$subscription = $null
if ($null -ne $subscriptions) {
    $matching = @($subscriptions | Where-Object { $_.name -eq $RunId })
    Add-AuditCheck -Name "subscription.unique" -Passed ($matching.Count -eq 1) -Evidence "subscriptions=$($matching.Count)"
    if ($matching.Count -eq 1) {
        $subscription = $matching[0]
        Add-AuditCheck -Name "subscription.eventType" -Passed (@($subscription.eventTypes) -contains "message_sent") -Evidence "messageSent=$(@($subscription.eventTypes) -contains 'message_sent')"
        Add-EqualCheck -Name "subscription.pending" -Actual $subscription.pendingDelivery -Expected $null
        Add-EqualCheck -Name "subscription.lastDelivery" -Actual $subscription.lastAckedDeliveryId -Expected $deliveryId
    }
}

if ($null -ne $events -and -not [string]::IsNullOrWhiteSpace($reservationId)) {
    $reservedEvents = @($events | Where-Object { $_.eventType -eq "paths_reserved" -and $_.entityId -eq $reservationId })
    $renewedEvents = @($events | Where-Object { $_.eventType -eq "paths_renewed" -and $_.entityId -eq $reservationId })
    $releasedEvents = @($events | Where-Object { $_.eventType -eq "paths_released" -and $_.entityId -eq $reservationId })
    Add-EqualCheck -Name "event.reserved.unique" -Actual $reservedEvents.Count -Expected 1
    Add-EqualCheck -Name "event.renewed.unique" -Actual $renewedEvents.Count -Expected 1
    Add-EqualCheck -Name "event.released.unique" -Actual $releasedEvents.Count -Expected 1
    if ($reservedEvents.Count -eq 1) {
        Add-EqualCheck -Name "event.reserved.actor" -Actual $reservedEvents[0].actor -Expected $AgentA
        Add-EqualCheck -Name "event.reserved.path" -Actual $reservedEvents[0].payload.pathPattern -Expected $reservationPath
        Add-EqualCheck -Name "event.reserved.expiry" -Actual ([int64]$reservedEvents[0].payload.expiresAt) -Expected $originalExpiry
    }
    if ($renewedEvents.Count -eq 1) {
        Add-EqualCheck -Name "event.renewed.actor" -Actual $renewedEvents[0].actor -Expected $AgentA
        Add-EqualCheck -Name "event.renewed.expiry" -Actual ([int64]$renewedEvents[0].payload.expiresAt) -Expected $renewedExpiry
        Add-EqualCheck -Name "event.renewed.ttl" -Actual ([int64]$renewedEvents[0].payload.ttlSeconds) -Expected 900
    }
    if ($releasedEvents.Count -eq 1) {
        Add-EqualCheck -Name "event.released.actor" -Actual $releasedEvents[0].actor -Expected $AgentA
    }
}

$finalKeys = @(
    "agentAStored", "agentAHasRecoveryKey", "agentATokenGeneration",
    "agentBStored", "agentBHasRecoveryKey", "agentBTokenGeneration",
    "claimOwner", "reservationId", "originalExpiry", "renewedExpiry",
    "deliveryId", "firstAckReplayed", "retryAckReplayed", "subscriptionAckAt",
    "aToBHandoffAckAt", "aToBHandoffClosedAt", "bToAHandoffAckAt", "bToAHandoffClosedAt",
    "claimConflictKind", "reservationConflictKind", "acceptanceReservations", "subscriptionPendingDelivery"
)
$finalValues = @{}
foreach ($key in $finalKeys) {
    $finalValues[$key] = Get-Decision -HandoffName "handoff.a-root" -Handoff $handoffRoot -Key $key
}
$bToAHandoffAckAt = if ($null -ne $handoffBA) { [string]$handoffBA.message.ackAt } else { $null }
$bToAHandoffClosedAt = if ($null -ne $handoffBA) { [string]$handoffBA.message.closedAt } else { $null }
$expectedFinalValues = [ordered]@{
    agentAStored = "true"
    agentAHasRecoveryKey = "true"
    agentATokenGeneration = "1"
    agentBStored = "true"
    agentBHasRecoveryKey = "true"
    agentBTokenGeneration = "1"
    claimOwner = $AgentA
    reservationId = $reservationId
    originalExpiry = $originalExpiryText
    renewedExpiry = $renewedExpiryText
    deliveryId = $deliveryId
    firstAckReplayed = "false"
    retryAckReplayed = "true"
    subscriptionAckAt = $subscriptionAckAt
    aToBHandoffAckAt = $aToBAckAt
    aToBHandoffClosedAt = $aToBClosedAt
    bToAHandoffAckAt = $bToAHandoffAckAt
    bToAHandoffClosedAt = $bToAHandoffClosedAt
    claimConflictKind = "conflict"
    reservationConflictKind = "conflict"
    acceptanceReservations = "0"
    subscriptionPendingDelivery = "false"
}
foreach ($key in $expectedFinalValues.Keys) {
    Add-EqualCheck -Name "handoff.a-root.value.$key" -Actual $finalValues[$key] -Expected $expectedFinalValues[$key]
}

if ($null -ne $artifacts) {
    $matchingBackup = @($artifacts | Where-Object {
        $_.kind -eq "database-backup" -and $_.path -eq $backupRelativePath
    })
    Add-AuditCheck -Name "backup.artifact.unique" -Passed ($matchingBackup.Count -eq 1) -Evidence "artifacts=$($matchingBackup.Count)"
    if ($matchingBackup.Count -eq 1) {
        $backupPath = Join-Path $ProjectRoot $backupRelativePath
        $backupExists = Test-Path -LiteralPath $backupPath -PathType Leaf
        Add-AuditCheck -Name "backup.file.exists" -Passed $backupExists -Evidence "exists=$backupExists"
        if ($backupExists) {
            $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $backupPath).Hash.ToLowerInvariant()
            Add-EqualCheck -Name "backup.sha256" -Actual $actualHash -Expected $matchingBackup[0].sha256
        }
    }
}

if ($SkipGit) {
    Add-AuditCheck -Name "git.clean" -Passed $true -Evidence "skipped by explicit -SkipGit" -Skipped $true
    Add-AuditCheck -Name "git.upstream" -Passed $true -Evidence "skipped by explicit -SkipGit" -Skipped $true
} else {
    $gitStatus = @(& git -C $ProjectRoot status --porcelain=v1 2>&1)
    $gitStatusExit = $LASTEXITCODE
    Add-AuditCheck -Name "git.clean" -Passed ($gitStatusExit -eq 0 -and $gitStatus.Count -eq 0) -Evidence "exit=$gitStatusExit; changedPaths=$($gitStatus.Count)"

    $head = (& git -C $ProjectRoot rev-parse HEAD 2>&1 | Out-String).Trim()
    $headExit = $LASTEXITCODE
    $upstream = (& git -C $ProjectRoot rev-parse '@{upstream}' 2>&1 | Out-String).Trim()
    $upstreamExit = $LASTEXITCODE
    Add-AuditCheck -Name "git.upstream" -Passed ($headExit -eq 0 -and $upstreamExit -eq 0 -and $head -eq $upstream) -Evidence "headMatchesUpstream=$($head -eq $upstream)"
}

$failed = @($script:auditChecks | Where-Object { -not $_.passed })
$report = [pscustomobject]@{
    ok = $failed.Count -eq 0
    mode = "non-destructive; authenticated reads may refresh Agent lastSeenAt"
    runId = $RunId
    projectRoot = $ProjectRoot
    checkedAt = [DateTimeOffset]::UtcNow.ToString("o")
    summary = [pscustomobject]@{
        total = $script:auditChecks.Count
        passed = @($script:auditChecks | Where-Object { $_.passed -and -not $_.skipped }).Count
        failed = $failed.Count
        skipped = @($script:auditChecks | Where-Object { $_.skipped }).Count
    }
    checks = $script:auditChecks
}

$report | ConvertTo-Json -Depth 8
if (-not $report.ok) {
    exit 1
}
