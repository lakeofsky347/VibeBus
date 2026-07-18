$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "hook-common.ps1")

function Test-VibeBusGitCommitCommand {
    param([string]$Command)
    return $Command -match '(?i)(^|[;&|]\s*|\s)git(?:\.exe)?\s+(?:-[^\s]+\s+)*commit(?:\s|$)'
}

function Get-VibeBusTestSuite {
    param([string]$Command)
    foreach ($entry in @(
        @{ Pattern = '(?i)(^|\s)cargo\s+(?:[^;&|]*\s)?test(?:\s|$)'; Name = 'cargo test' },
        @{ Pattern = '(?i)(^|\s)(?:npm|pnpm|yarn)\s+(?:run\s+)?test(?:\s|$)'; Name = 'JavaScript tests' },
        @{ Pattern = '(?i)(^|\s)(?:python\s+-m\s+)?pytest(?:\s|$)'; Name = 'pytest' },
        @{ Pattern = '(?i)(^|\s)go\s+test(?:\s|$)'; Name = 'go test' },
        @{ Pattern = '(?i)(^|\s)dotnet\s+test(?:\s|$)'; Name = 'dotnet test' },
        @{ Pattern = '(?i)(^|\s)(?:pwsh|powershell)?[^;&|]*test-[\w-]+\.ps1(?:\s|$)'; Name = 'PowerShell acceptance' }
    )) {
        if ($Command -match $entry.Pattern) {
            return $entry.Name
        }
    }
    return $null
}

try {
    $hookInput = Read-VibeBusHookInput
    if ($hookInput.hook_event_name -ne "PostToolUse" -or $hookInput.tool_name -ne "Bash") {
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
    $command = Get-VibeBusHookCommand -HookInput $hookInput
    if ([string]::IsNullOrWhiteSpace($command)) {
        exit 0
    }
    $isCommit = Test-VibeBusGitCommitCommand -Command $command
    $suite = Get-VibeBusTestSuite -Command $command
    if (-not $isCommit -and $null -eq $suite) {
        exit 0
    }
    $exitCode = Get-VibeBusHookExitCode -HookInput $hookInput
    if ($null -eq $exitCode) {
        Write-VibeBusHookMessage "VibeBus skipped lifecycle fact capture because Bash did not expose a reliable exit code."
        exit 0
    }

    $messages = @()
    if ($isCommit -and $exitCode -eq 0) {
        if ($env:VIBEBUS_HOOK_DRY_RUN -eq "1") {
            if ([string]::IsNullOrWhiteSpace($env:VIBEBUS_HOOK_TEST_GIT)) {
                throw "dry-run Git metadata is unavailable"
            }
            $git = $env:VIBEBUS_HOOK_TEST_GIT | ConvertFrom-Json
        } else {
            $commitSha = (& git -C $root rev-parse HEAD 2>$null | Out-String).Trim()
            if ($LASTEXITCODE -ne 0) { throw "unable to resolve Git HEAD" }
            $subject = (& git -C $root show -s --format=%s HEAD 2>$null | Out-String).Trim()
            if ($LASTEXITCODE -ne 0) { throw "unable to resolve the Git subject" }
            $changedPaths = @(& git -C $root show --pretty=format: --name-only --no-renames HEAD 2>$null |
                ForEach-Object { ([string]$_).Trim() } |
                Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
                Select-Object -Unique)
            if ($LASTEXITCODE -ne 0) { throw "unable to resolve changed Git paths" }
            if ($changedPaths.Count -gt 200) { throw "the commit exceeds the 200-path fact limit" }
            $git = [pscustomobject]@{
                commitSha = $commitSha
                summary = $subject
                changedPaths = $changedPaths
            }
        }
        $commitKey = "hook-git-$($git.commitSha)"
        if ($env:VIBEBUS_HOOK_DRY_RUN -ne "1") {
            $arguments = @(
                "fact", "git-commit", "--agent", [string]$binding.agent,
                "--task", [string]$binding.taskId,
                "--commit-sha", [string]$git.commitSha,
                "--summary", (Limit-VibeBusText -Text ([string]$git.summary) -MaximumLength 512),
                "--idempotency-key", $commitKey
            )
            foreach ($path in @($git.changedPaths)) {
                $arguments += @("--changed-path", [string]$path)
            }
            Invoke-VibeBusHookCli -Root $root -Arguments $arguments | Out-Null
        }
        $messages += "VibeBus recorded Git commit $($git.commitSha) for task $($binding.taskId) using changed paths only."
    }

    if ($null -ne $suite) {
        $outcome = if ($exitCode -eq 0) { "passed" } else { "failed" }
        $boundedCommand = Limit-VibeBusText -Text $command -MaximumLength 512
        if ($env:VIBEBUS_HOOK_DRY_RUN -eq "1") {
            $head = "dry-run-head"
            $workingState = "dry-run-state"
        } else {
            $head = (& git -C $root rev-parse HEAD 2>$null | Out-String).Trim()
            if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($head)) { $head = "no-head" }
            $workingStateRaw = (& git -C $root status --porcelain=v1 -uno 2>$null | Out-String)
            if ($LASTEXITCODE -ne 0) { $workingStateRaw = "status-unavailable" }
            $workingState = Get-VibeBusSha256 -Text $workingStateRaw
        }
        $resultHash = Get-VibeBusSha256 -Text "$($binding.taskId)|$boundedCommand|$head|$workingState"
        $resultKey = "hook-test-$($resultHash.Substring(0, 48))"
        $idempotencyKey = "hook-test-$resultHash"
        if ($env:VIBEBUS_HOOK_DRY_RUN -ne "1") {
            Invoke-VibeBusHookCli -Root $root -Arguments @(
                "fact", "test-result", "--agent", [string]$binding.agent,
                "--task", [string]$binding.taskId,
                "--result-key", $resultKey,
                "--suite", $suite,
                "--outcome", $outcome,
                "--summary", "Observed test command exited with code $exitCode.",
                "--command", $boundedCommand,
                "--idempotency-key", $idempotencyKey
            ) | Out-Null
        }
        $messages += "VibeBus recorded a bounded $suite result ($outcome) for task $($binding.taskId); command output was not stored."
    }

    if ($messages.Count -gt 0) {
        Write-VibeBusHookMessage ($messages -join " ")
    }
} catch {
    Write-VibeBusHookMessage "VibeBus lifecycle fact capture failed: $($_.Exception.Message). The completed tool side effects were not changed."
}

exit 0
