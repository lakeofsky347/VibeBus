$ErrorActionPreference = "Stop"

function Read-VibeBusHookInput {
    $raw = [Console]::In.ReadToEnd()
    if ([string]::IsNullOrWhiteSpace($raw)) {
        throw "hook input is empty"
    }
    return $raw | ConvertFrom-Json
}

function Find-VibeBusProjectRoot {
    param([Parameter(Mandatory = $true)][string]$WorkingDirectory)

    $cursor = (Resolve-Path -LiteralPath $WorkingDirectory).Path
    while ($true) {
        if (Test-Path -LiteralPath (Join-Path $cursor ".vibebus\project.json") -PathType Leaf) {
            return $cursor
        }
        $parent = Split-Path -Parent $cursor
        if ([string]::IsNullOrWhiteSpace($parent) -or $parent -eq $cursor) {
            return $null
        }
        $cursor = $parent
    }
}

function Get-VibeBusCliPath {
    if (-not [string]::IsNullOrWhiteSpace($env:VIBEBUS_HOOK_CLI)) {
        return $env:VIBEBUS_HOOK_CLI
    }
    if ([string]::IsNullOrWhiteSpace($env:PLUGIN_ROOT)) {
        throw "PLUGIN_ROOT is not available"
    }
    $candidate = Join-Path $env:PLUGIN_ROOT "bin\vibebus.exe"
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "the packaged VibeBus CLI is unavailable"
    }
    return $candidate
}

function Invoke-VibeBusHookCli {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    $cli = Get-VibeBusCliPath
    $raw = (& $cli --root $Root @Arguments 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "VibeBus CLI failed with exit code $LASTEXITCODE"
    }
    $response = $raw | ConvertFrom-Json
    if ($response.ok -ne $true) {
        throw "VibeBus CLI returned an unsuccessful response"
    }
    return $response.result
}

function Resolve-VibeBusTaskBinding {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$SessionId
    )

    if ($env:VIBEBUS_HOOK_DRY_RUN -eq "1" -and
        -not [string]::IsNullOrWhiteSpace($env:VIBEBUS_HOOK_TEST_BINDING)) {
        return $env:VIBEBUS_HOOK_TEST_BINDING | ConvertFrom-Json
    }

    $bindings = @(Invoke-VibeBusHookCli -Root $Root -Arguments @("thread", "list", "--all"))
    $active = @($bindings | Where-Object {
        $null -eq $_.unboundAt -and
        ($_.threadId -eq $SessionId -or $_.threadId -eq "codex:$SessionId")
    })
    if ($active.Count -eq 0) {
        return $null
    }
    if ($active.Count -gt 1) {
        throw "multiple active VibeBus task bindings match this Codex task"
    }
    return $active[0]
}

function Get-VibeBusHookCommand {
    param([Parameter(Mandatory = $true)]$HookInput)

    $command = $HookInput.tool_input.command
    if ($null -eq $command) {
        return ""
    }
    if ($command -is [System.Array]) {
        return (($command | ForEach-Object { [string]$_ }) -join " ").Trim()
    }
    return ([string]$command).Trim()
}

function Get-VibeBusHookExitCode {
    param([Parameter(Mandatory = $true)]$HookInput)

    $response = $HookInput.tool_response
    foreach ($candidate in @(
        $response.exit_code,
        $response.exitCode,
        $response.metadata.exit_code,
        $response.metadata.exitCode
    )) {
        if ($null -ne $candidate -and [string]$candidate -match '^-?\d+$') {
            return [int]$candidate
        }
    }
    if ($response -is [string] -and
        $response -match '(?im)^\s*(?:exit code|process exited with code)\s*[:=]?\s*(-?\d+)\s*$') {
        return [int]$Matches[1]
    }
    return $null
}

function Get-VibeBusSha256 {
    param([Parameter(Mandatory = $true)][string]$Text)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace("-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Limit-VibeBusText {
    param(
        [Parameter(Mandatory = $true)][string]$Text,
        [Parameter(Mandatory = $true)][int]$MaximumLength
    )

    $normalized = ($Text -replace '\s+', ' ').Trim()
    if ($normalized.Length -le $MaximumLength) {
        return $normalized
    }
    return $normalized.Substring(0, $MaximumLength)
}

function Write-VibeBusHookMessage {
    param([Parameter(Mandatory = $true)][string]$Message)

    @{ systemMessage = (Limit-VibeBusText -Text $Message -MaximumLength 1200) } |
        ConvertTo-Json -Compress
}
