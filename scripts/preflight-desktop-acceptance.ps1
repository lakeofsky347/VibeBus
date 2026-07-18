[CmdletBinding()]
param(
    [string]$ProjectRoot = "",
    [string]$DataHome = "",
    [string]$VibeBusPath = "",
    [string]$RunId = "desktop-20260717-01",
    [string]$AgentA = "",
    [string]$AgentB = "",
    [string]$ControllerAgent = "git-publisher-019f6eab",
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
    Prep = "DESKTOP-ACCEPTANCE-PREP-001"
    Controller = "DESKTOP-ACCEPTANCE-001"
    BReady = "DESKTOP-B-READY-001"
    Claim = "DESKTOP-CLAIM-001"
    BResult = "DESKTOP-B-RESULT-001"
    AFinalize = "DESKTOP-A-FINALIZE-001"
}
$fixtureTaskIds = @($taskIds.BReady, $taskIds.Claim, $taskIds.BResult, $taskIds.AFinalize)
$reservationPath = "acceptance/$RunId/shared-resource"
$backupRelativePath = "backups/vibebus-0.8-pre-desktop-acceptance.db"
$expectedBackupBytes = 512000L
$expectedBackupSha256 = "0079a09f200dd5c7210c1dbb563da3b77f29b80b17d5c2504168a1bae230611c"
$script:preflightChecks = [System.Collections.Generic.List[object]]::new()

function Add-PreflightCheck {
    param(
        [string]$Name,
        [bool]$Passed,
        [string]$Evidence,
        [bool]$Skipped = $false
    )

    $script:preflightChecks.Add([pscustomobject]@{
        name = $Name
        passed = $Passed
        skipped = $Skipped
        evidence = $Evidence
    }) | Out-Null
}

function Format-PreflightValue {
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

    Add-PreflightCheck -Name $Name -Passed ($Actual -eq $Expected) -Evidence (
        "expected={0}; actual={1}" -f (Format-PreflightValue $Expected), (Format-PreflightValue $Actual)
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
    Add-PreflightCheck -Name "command.$Name" -Passed $call.succeeded -Evidence $evidence
    return $call.result
}

function Get-UniqueTask {
    param(
        [object[]]$Tasks,
        [string]$TaskId
    )

    $matching = @($Tasks | Where-Object { $_.taskId -eq $TaskId })
    Add-PreflightCheck -Name "task.$TaskId.unique" -Passed ($matching.Count -eq 1) -Evidence "tasks=$($matching.Count)"
    if ($matching.Count -ne 1) {
        return $null
    }
    return $matching[0]
}

function Test-TaskFixture {
    param(
        [object[]]$Tasks,
        [string]$TaskId,
        [string]$ExpectedStatus,
        [string[]]$ExpectedDependencies
    )

    $task = Get-UniqueTask -Tasks $Tasks -TaskId $TaskId
    if ($null -eq $task) {
        return
    }
    Add-EqualCheck -Name "task.$TaskId.status" -Actual $task.status -Expected $ExpectedStatus
    Add-EqualCheck -Name "task.$TaskId.owner" -Actual $task.owner -Expected $null
    Add-EqualCheck -Name "task.$TaskId.version" -Actual ([int]$task.version) -Expected 1
    Add-EqualCheck -Name "task.$TaskId.blockedReason" -Actual $task.blockedReason -Expected $null

    $actualDependencies = @($task.dependsOn)
    $dependencyDifference = @(
        Compare-Object -ReferenceObject @($ExpectedDependencies) -DifferenceObject $actualDependencies
    )
    Add-PreflightCheck -Name "task.$TaskId.dependencies" -Passed ($dependencyDifference.Count -eq 0) -Evidence (
        "expected={0}; actual={1}" -f (@($ExpectedDependencies) -join ","), ($actualDependencies -join ",")
    )
}

Add-EqualCheck -Name "fixture.runId" -Actual $RunId -Expected "desktop-20260717-01"
Add-EqualCheck -Name "fixture.agentA" -Actual $AgentA -Expected "desktop-a-20260717-01"
Add-EqualCheck -Name "fixture.agentB" -Actual $AgentB -Expected "desktop-b-20260717-01"
Add-EqualCheck -Name "fixture.controllerAgent" -Actual $ControllerAgent -Expected "git-publisher-019f6eab"

$doctor = Get-VibeBusResult -Name "doctor" -Arguments @("doctor", "--root", $ProjectRoot)
$operator = Get-VibeBusResult -Name "operator-status" -Arguments @("operator", "status", "--root", $ProjectRoot)
$agents = @(Get-VibeBusResult -Name "agents" -Arguments @("agents", "--root", $ProjectRoot))
$credentialA = Get-VibeBusResult -Name "credential-a" -Arguments @(
    "credential", "status", "--root", $ProjectRoot, "--agent", $AgentA
)
$credentialB = Get-VibeBusResult -Name "credential-b" -Arguments @(
    "credential", "status", "--root", $ProjectRoot, "--agent", $AgentB
)
$tasks = @(Get-VibeBusResult -Name "tasks" -Arguments @("task", "list", "--root", $ProjectRoot))
$bindings = @(Get-VibeBusResult -Name "bindings" -Arguments @("thread", "list", "--root", $ProjectRoot, "--all"))
$reservations = @(Get-VibeBusResult -Name "reservations" -Arguments @("reserve", "list", "--root", $ProjectRoot))
$artifacts = @(Get-VibeBusResult -Name "controller-artifacts" -Arguments @(
    "artifact", "list", "--root", $ProjectRoot, "--task", $taskIds.Controller
))

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

if ($null -ne $agents) {
    foreach ($agentName in @($AgentA, $AgentB)) {
        $matchingAgents = @($agents | Where-Object { $_.name -eq $agentName })
        Add-EqualCheck -Name "agent.$agentName.absent" -Actual $matchingAgents.Count -Expected 0
    }
}

foreach ($credentialSpec in @(
    @{ Label = "a"; Value = $credentialA },
    @{ Label = "b"; Value = $credentialB }
)) {
    if ($null -eq $credentialSpec.Value) {
        continue
    }
    Add-EqualCheck -Name "credential.$($credentialSpec.Label).stored" -Actual $credentialSpec.Value.stored -Expected $false
    Add-EqualCheck -Name "credential.$($credentialSpec.Label).recovery" -Actual $credentialSpec.Value.hasRecoveryKey -Expected $false
    Add-EqualCheck -Name "credential.$($credentialSpec.Label).generation" -Actual $credentialSpec.Value.tokenGeneration -Expected $null
}

if ($null -ne $tasks) {
    $prep = Get-UniqueTask -Tasks @($tasks) -TaskId $taskIds.Prep
    if ($null -ne $prep) {
        Add-EqualCheck -Name "task.$($taskIds.Prep).status" -Actual $prep.status -Expected "completed"
    }

    $controller = Get-UniqueTask -Tasks @($tasks) -TaskId $taskIds.Controller
    if ($null -ne $controller) {
        Add-EqualCheck -Name "task.$($taskIds.Controller).status" -Actual $controller.status -Expected "blocked"
        Add-EqualCheck -Name "task.$($taskIds.Controller).owner" -Actual $controller.owner -Expected $ControllerAgent
        $reason = [string]$controller.blockedReason
        $mentionsAuthorization = $reason -like "*explicit user authorization*two user-owned top-level Codex tasks*"
        Add-PreflightCheck -Name "task.$($taskIds.Controller).authorizationGate" -Passed $mentionsAuthorization -Evidence (
            "explicitAuthorizationGate=$mentionsAuthorization"
        )
    }

    Test-TaskFixture -Tasks @($tasks) -TaskId $taskIds.BReady -ExpectedStatus "ready" -ExpectedDependencies @($taskIds.Prep)
    Test-TaskFixture -Tasks @($tasks) -TaskId $taskIds.Claim -ExpectedStatus "pending" -ExpectedDependencies @($taskIds.BReady)
    Test-TaskFixture -Tasks @($tasks) -TaskId $taskIds.BResult -ExpectedStatus "pending" -ExpectedDependencies @($taskIds.BReady)
    Test-TaskFixture -Tasks @($tasks) -TaskId $taskIds.AFinalize -ExpectedStatus "pending" -ExpectedDependencies @($taskIds.BResult)
}

if ($null -ne $bindings) {
    $fixtureBindings = @($bindings | Where-Object { $fixtureTaskIds -contains $_.taskId })
    Add-EqualCheck -Name "binding.fixtureHistory" -Actual $fixtureBindings.Count -Expected 0
}

if ($null -ne $reservations) {
    $fixtureReservations = @($reservations | Where-Object { $_.pathPattern -eq $reservationPath })
    Add-EqualCheck -Name "reservation.fixtureActive" -Actual $fixtureReservations.Count -Expected 0
}

if ($null -ne $artifacts) {
    $matchingBackup = @($artifacts | Where-Object {
        $_.kind -eq "database-backup" -and $_.path -eq $backupRelativePath
    })
    Add-EqualCheck -Name "backup.artifact" -Actual $matchingBackup.Count -Expected 1
    if ($matchingBackup.Count -eq 1) {
        Add-EqualCheck -Name "backup.artifact.sha256" -Actual $matchingBackup[0].sha256 -Expected $expectedBackupSha256
        $backupPath = Join-Path $ProjectRoot $backupRelativePath
        $backupExists = Test-Path -LiteralPath $backupPath -PathType Leaf
        Add-PreflightCheck -Name "backup.file.exists" -Passed $backupExists -Evidence "exists=$backupExists"
        if ($backupExists) {
            $actualBytes = (Get-Item -LiteralPath $backupPath).Length
            Add-EqualCheck -Name "backup.bytes" -Actual $actualBytes -Expected $expectedBackupBytes
            $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $backupPath).Hash.ToLowerInvariant()
            Add-EqualCheck -Name "backup.sha256" -Actual $actualHash -Expected $expectedBackupSha256
        }
    }
}

if ($SkipGit) {
    Add-PreflightCheck -Name "git.clean" -Passed $true -Evidence "skipped by explicit -SkipGit" -Skipped $true
    Add-PreflightCheck -Name "git.upstream" -Passed $true -Evidence "skipped by explicit -SkipGit" -Skipped $true
} else {
    $gitStatus = @(& git -C $ProjectRoot status --porcelain=v1 2>&1)
    $gitStatusExit = $LASTEXITCODE
    Add-PreflightCheck -Name "git.clean" -Passed ($gitStatusExit -eq 0 -and $gitStatus.Count -eq 0) -Evidence (
        "exit=$gitStatusExit; changedPaths=$($gitStatus.Count)"
    )

    $head = (& git -C $ProjectRoot rev-parse HEAD 2>&1 | Out-String).Trim()
    $headExit = $LASTEXITCODE
    $upstream = (& git -C $ProjectRoot rev-parse '@{upstream}' 2>&1 | Out-String).Trim()
    $upstreamExit = $LASTEXITCODE
    Add-PreflightCheck -Name "git.upstream" -Passed (
        $headExit -eq 0 -and $upstreamExit -eq 0 -and $head -eq $upstream
    ) -Evidence "headMatchesUpstream=$($head -eq $upstream)"
}

$failed = @($script:preflightChecks | Where-Object { -not $_.passed })
$report = [pscustomobject]@{
    ok = $failed.Count -eq 0
    mode = "non-destructive; no Agent registration, authentication, task mutation, reservation mutation, or inbox read"
    runId = $RunId
    projectRoot = $ProjectRoot
    checkedAt = [DateTimeOffset]::UtcNow.ToString("o")
    summary = [pscustomobject]@{
        total = $script:preflightChecks.Count
        passed = @($script:preflightChecks | Where-Object { $_.passed -and -not $_.skipped }).Count
        failed = $failed.Count
        skipped = @($script:preflightChecks | Where-Object { $_.skipped }).Count
    }
    checks = $script:preflightChecks
}

$report | ConvertTo-Json -Depth 8
if (-not $report.ok) {
    exit 1
}
