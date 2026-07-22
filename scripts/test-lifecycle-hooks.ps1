$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$pluginRoot = Join-Path $repoRoot "plugins\vibebus"
$hookRoot = Join-Path $pluginRoot "scripts"
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("vibebus-hooks-" + [Guid]::NewGuid().ToString("N"))
$pluginData = Join-Path $testRoot "plugin-data"

function Invoke-HookFixture {
    param(
        [Parameter(Mandatory = $true)][string]$Script,
        [Parameter(Mandatory = $true)][hashtable]$InputObject
    )
    $json = $InputObject | ConvertTo-Json -Depth 12 -Compress
    $output = $json | & powershell -NoProfile -ExecutionPolicy Bypass -File $Script
    if ($LASTEXITCODE -ne 0) {
        throw "hook '$Script' failed with exit code $LASTEXITCODE"
    }
    if ([string]::IsNullOrWhiteSpace(($output | Out-String))) {
        return $null
    }
    return ($output | Out-String).Trim() | ConvertFrom-Json
}

New-Item -ItemType Directory -Force -Path $pluginData | Out-Null
try {
    $env:PLUGIN_ROOT = $pluginRoot
    $env:PLUGIN_DATA = $pluginData
    $env:VIBEBUS_HOOK_DRY_RUN = "1"
    $env:VIBEBUS_HOOK_TEST_BINDING = @{
        agent = "hook-test-agent"
        taskId = "HOOK-TEST-001"
        threadId = "hook-session"
        unboundAt = $null
    } | ConvertTo-Json -Compress
    $env:VIBEBUS_HOOK_TEST_GIT = @{
        commitSha = "0123456789abcdef0123456789abcdef01234567"
        summary = "Hook fixture"
        changedPaths = @("src/lib.rs", "tests/core_workflows.rs")
    } | ConvertTo-Json -Compress
    $env:VIBEBUS_HOOK_TEST_PROPOSAL = @{
        taskId = "HOOK-TEST-001"
        status = "working"
        gitCommits = @()
        testResults = @()
        artifacts = @()
        decisions = @()
        nextActions = @("Review and send explicitly")
    } | ConvertTo-Json -Compress

    $postTool = Join-Path $hookRoot "post-tool-facts.ps1"
    $commitResult = Invoke-HookFixture -Script $postTool -InputObject @{
        session_id = "hook-session"
        cwd = $repoRoot
        hook_event_name = "PostToolUse"
        tool_name = "Bash"
        tool_input = @{ command = "git commit -m fixture" }
        tool_response = @{ exit_code = 0; output = "content is deliberately ignored" }
    }
    if ($commitResult.systemMessage -notmatch 'changed paths only') {
        throw "Git hook did not report its bounded path-only behavior."
    }

    $testResult = Invoke-HookFixture -Script $postTool -InputObject @{
        session_id = "hook-session"
        cwd = $repoRoot
        hook_event_name = "PostToolUse"
        tool_name = "Bash"
        tool_input = @{ command = "cargo test --all-targets --locked" }
        tool_response = @{ exitCode = 0; output = "test logs are deliberately ignored" }
    }
    if ($testResult.systemMessage -notmatch 'command output was not stored') {
        throw "Test hook did not report its bounded no-log behavior."
    }

    $unknownExit = Invoke-HookFixture -Script $postTool -InputObject @{
        session_id = "hook-session"
        cwd = $repoRoot
        hook_event_name = "PostToolUse"
        tool_name = "Bash"
        tool_input = @{ command = "cargo test" }
        tool_response = @{ output = "no exit metadata" }
    }
    if ($unknownExit.systemMessage -notmatch 'reliable exit code') {
        throw "Test hook must skip unknown outcomes instead of guessing."
    }

    $stopResult = Invoke-HookFixture -Script (Join-Path $hookRoot "stop-handoff.ps1") -InputObject @{
        session_id = "hook-session"
        cwd = $repoRoot
        hook_event_name = "Stop"
        stop_hook_active = $false
        last_assistant_message = "This field is deliberately ignored."
    }
    if ($stopResult.systemMessage -notmatch 'no handoff was sent automatically') {
        throw "Stop hook did not preserve the explicit-send boundary."
    }
    $proposalFiles = @(Get-ChildItem -LiteralPath (Join-Path $pluginData "handoff-proposals") -Filter "*.json")
    if ($proposalFiles.Count -ne 1) {
        throw "Expected one bounded proposal file, found $($proposalFiles.Count)."
    }
    $proposal = Get-Content -Raw -LiteralPath $proposalFiles[0].FullName | ConvertFrom-Json
    if ($proposal.taskId -ne "HOOK-TEST-001") {
        throw "Stop hook proposal was not task scoped."
    }

    $hooks = Get-Content -Raw -LiteralPath (Join-Path $pluginRoot "hooks\hooks.json") | ConvertFrom-Json
    if (@($hooks.hooks.PostToolUse).Count -ne 1 -or @($hooks.hooks.Stop).Count -ne 1) {
        throw "Plugin lifecycle hook configuration is incomplete."
    }

    [pscustomobject]@{
        ok = $true
        checks = 7
        failures = 0
        skipped = 0
    } | ConvertTo-Json
} finally {
    Remove-Item Env:\VIBEBUS_HOOK_DRY_RUN -ErrorAction SilentlyContinue
    Remove-Item Env:\VIBEBUS_HOOK_TEST_BINDING -ErrorAction SilentlyContinue
    Remove-Item Env:\VIBEBUS_HOOK_TEST_GIT -ErrorAction SilentlyContinue
    Remove-Item Env:\VIBEBUS_HOOK_TEST_PROPOSAL -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $testRoot -PathType Container) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
