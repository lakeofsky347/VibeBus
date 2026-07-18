$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "hook-common.ps1")

try {
    $hookInput = Read-VibeBusHookInput
    if ($hookInput.hook_event_name -ne "Stop" -or $hookInput.stop_hook_active -eq $true) {
        exit 0
    }
    $root = Find-VibeBusProjectRoot -WorkingDirectory ([string]$hookInput.cwd)
    if ($null -eq $root) {
        exit 0
    }
    $binding = Resolve-VibeBusTaskBinding -Root $root -SessionId ([string]$hookInput.session_id)
    if ($null -eq $binding) {
        exit 0
    }
    if ($env:VIBEBUS_HOOK_DRY_RUN -eq "1") {
        if ([string]::IsNullOrWhiteSpace($env:VIBEBUS_HOOK_TEST_PROPOSAL)) {
            throw "dry-run handoff proposal is unavailable"
        }
        $proposal = $env:VIBEBUS_HOOK_TEST_PROPOSAL | ConvertFrom-Json
    } else {
        $proposal = Invoke-VibeBusHookCli -Root $root -Arguments @(
            "handoff", "propose", "--agent", [string]$binding.agent,
            "--task", [string]$binding.taskId, "--item-limit", "10"
        )
    }
    if ([string]::IsNullOrWhiteSpace($env:PLUGIN_DATA)) {
        throw "PLUGIN_DATA is not available"
    }
    $proposalRoot = Join-Path $env:PLUGIN_DATA "handoff-proposals"
    New-Item -ItemType Directory -Force -Path $proposalRoot | Out-Null
    $safeSession = ([string]$hookInput.session_id) -replace '[^A-Za-z0-9._-]', '_'
    $timestamp = [DateTimeOffset]::UtcNow.ToString("yyyyMMddTHHmmssfffZ")
    $proposalPath = Join-Path $proposalRoot "$safeSession-$timestamp.json"
    $proposal | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $proposalPath -Encoding UTF8
    Write-VibeBusHookMessage "VibeBus prepared a bounded handoff proposal for task $($binding.taskId) at '$proposalPath'. Review it before explicitly sending; no handoff was sent automatically."
} catch {
    Write-VibeBusHookMessage "VibeBus handoff proposal generation failed: $($_.Exception.Message). No handoff was sent."
}

exit 0
