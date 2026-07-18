param(
    [string]$BinaryPath = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-VibeBusJson {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,
        [switch]$Sensitive
    )

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & $script:VibeBusBinary @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if ($exitCode -ne 0) {
        if ($Sensitive) {
            throw "VibeBus command failed while handling redacted registration data"
        }
        throw "vibebus $($Arguments -join ' ') failed: $($output -join [Environment]::NewLine)"
    }

    try {
        return (($output -join "`n") | ConvertFrom-Json)
    }
    catch {
        if ($Sensitive) {
            throw "VibeBus returned invalid JSON while handling redacted registration data"
        }
        throw
    }
}

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($BinaryPath)) {
    $candidates = @(
        (Join-Path $repoRoot "target\release\vibebus.exe"),
        (Join-Path $repoRoot "target\release\vibebus"),
        (Join-Path $repoRoot "target\debug\vibebus.exe"),
        (Join-Path $repoRoot "target\debug\vibebus")
    )
    $BinaryPath = $candidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
}
if ([string]::IsNullOrWhiteSpace($BinaryPath) -or -not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
    throw "VibeBus binary was not found; build it or pass -BinaryPath"
}
$script:VibeBusBinary = (Resolve-Path -LiteralPath $BinaryPath).Path

$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$drillRoot = [IO.Path]::GetFullPath((Join-Path $temporaryBase ("vibebus-restore-" + [guid]::NewGuid().ToString("N"))))
if (-not $drillRoot.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase)) {
    throw "disposable restore directory escaped the system temporary directory"
}

$sourceRoot = Join-Path $drillRoot "source-project"
$sourceData = Join-Path $drillRoot "source-data"
$restoredRoot = Join-Path $drillRoot "restored-project"
$restoredData = Join-Path $drillRoot "restored-data"
$backupPath = Join-Path $drillRoot "export\vibebus.db"
$previousAgentToken = $env:VIBEBUS_AGENT_TOKEN
$result = $null

try {
    Write-Host "[restore] initializing disposable source"
    $initialized = Invoke-VibeBusJson -Arguments @(
        "init", "--root", $sourceRoot, "--data-home", $sourceData,
        "--name", "Backup Restore Acceptance"
    )
    $projectId = [string]$initialized.project.projectId
    if ([string]::IsNullOrWhiteSpace($projectId) -or -not $projectId.StartsWith("prj_")) {
        throw "source project did not return a valid project ID"
    }

    Write-Host "[restore] creating pre-backup state"
    $registration = Invoke-VibeBusJson -Sensitive -Arguments @(
        "register", "--root", $sourceRoot, "--data-home", $sourceData,
        "--name", "restore-agent", "--role", "operations"
    )
    $env:VIBEBUS_AGENT_TOKEN = [string]$registration.result.token
    if ([string]::IsNullOrWhiteSpace($env:VIBEBUS_AGENT_TOKEN)) {
        throw "registration token was missing"
    }
    $null = Invoke-VibeBusJson -Arguments @(
        "task", "create", "--root", $sourceRoot, "--data-home", $sourceData,
        "--agent", "restore-agent", "--id", "RESTORE-PRE-001",
        "--title", "State included in backup"
    )

    Write-Host "[restore] exporting online backup"
    $backup = Invoke-VibeBusJson -Arguments @(
        "backup", "--root", $sourceRoot, "--data-home", $sourceData,
        "--output", $backupPath
    )
    $backupSha256 = [string]$backup.result.sha256
    if ($backupSha256.Length -ne 64 -or -not (Test-Path -LiteralPath $backupPath -PathType Leaf)) {
        throw "online backup did not return a complete SHA-256 artifact"
    }

    Write-Host "[restore] creating post-backup drift"
    $null = Invoke-VibeBusJson -Arguments @(
        "task", "create", "--root", $sourceRoot, "--data-home", $sourceData,
        "--agent", "restore-agent", "--id", "RESTORE-POST-001",
        "--title", "State created after backup"
    )
    $sourceTasks = @( (Invoke-VibeBusJson -Arguments @(
        "task", "list", "--root", $sourceRoot, "--data-home", $sourceData
    )).result )
    if ($sourceTasks.Count -ne 2) {
        throw "source project must contain exactly two tasks after drift"
    }

    Write-Host "[restore] importing into isolated data home"
    $restoredMarker = Join-Path $restoredRoot ".vibebus\project.json"
    $restoredDatabase = Join-Path $restoredData "projects\$projectId\vibebus.db"
    [IO.Directory]::CreateDirectory((Split-Path -Parent $restoredMarker)) | Out-Null
    [IO.Directory]::CreateDirectory((Split-Path -Parent $restoredDatabase)) | Out-Null
    Copy-Item -LiteralPath (Join-Path $sourceRoot ".vibebus\project.json") -Destination $restoredMarker
    Copy-Item -LiteralPath $backupPath -Destination $restoredDatabase

    $restoredSha256 = (Get-FileHash -LiteralPath $restoredDatabase -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($restoredSha256 -ne $backupSha256) {
        throw "restored database hash does not match the exported backup"
    }

    Write-Host "[restore] verifying restored state and authentication"
    $doctor = Invoke-VibeBusJson -Arguments @(
        "doctor", "--root", $restoredRoot, "--data-home", $restoredData
    )
    if (-not $doctor.result.ok -or $doctor.result.journalMode -ne "wal" -or -not $doctor.result.foreignKeysEnabled) {
        throw "restored database did not pass doctor/WAL/foreign-key checks"
    }
    $restoredTasks = @( (Invoke-VibeBusJson -Arguments @(
        "task", "list", "--root", $restoredRoot, "--data-home", $restoredData
    )).result )
    if ($restoredTasks.Count -ne 1 -or $restoredTasks[0].taskId -ne "RESTORE-PRE-001") {
        throw "restored point-in-time task set is incorrect"
    }
    $restoredInbox = Invoke-VibeBusJson -Arguments @(
        "inbox", "--root", $restoredRoot, "--data-home", $restoredData,
        "--agent", "restore-agent"
    )
    if (@($restoredInbox.result).Count -ne 0) {
        throw "restored disposable Agent inbox must be empty"
    }

    $result = [ordered]@{
        ok = $true
        projectId = $projectId
        backupBytes = [long]$backup.result.bytes
        backupSha256 = $backupSha256
        restoredSha256 = $restoredSha256
        schemaVersion = [int]$doctor.result.schemaVersion
        journalMode = [string]$doctor.result.journalMode
        foreignKeysEnabled = [bool]$doctor.result.foreignKeysEnabled
        restoredAgents = [int]$doctor.result.agents
        sourceTasksAfterBackup = $sourceTasks.Count
        restoredTasks = $restoredTasks.Count
        authenticatedInbox = $true
        postBackupMutationExcluded = $true
        secretsPrinted = $false
    }
}
finally {
    Remove-Variable registration -ErrorAction SilentlyContinue
    if ($null -eq $previousAgentToken) {
        Remove-Item Env:VIBEBUS_AGENT_TOKEN -ErrorAction SilentlyContinue
    }
    else {
        $env:VIBEBUS_AGENT_TOKEN = $previousAgentToken
    }
    if ([IO.Directory]::Exists($drillRoot)) {
        [IO.Directory]::Delete($drillRoot, $true)
    }
}

$result["cleanupComplete"] = -not [IO.Directory]::Exists($drillRoot)
$result | ConvertTo-Json
