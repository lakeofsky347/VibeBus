$ErrorActionPreference = "Stop"

try {
    $inputData = [Console]::In.ReadToEnd() | ConvertFrom-Json
    $cursor = (Resolve-Path -LiteralPath $inputData.cwd).Path

    while ($true) {
        $marker = Join-Path $cursor ".vibebus\project.json"
        if (Test-Path -LiteralPath $marker -PathType Leaf) {
            $project = Get-Content -Raw -LiteralPath $marker | ConvertFrom-Json
            $context = @(
                "VibeBus project '$($project.name)' is active at '$cursor'."
                "For every VibeBus MCP call, pass root='$cursor'."
                "Register this independent task once with storeCredentials=true, verify credential status, then use vault-backed handoff snapshots or inbox checks at turn boundaries without copying secrets into the task. If vault storage fails, retain the returned token and recovery key only in private credential context. Atomically claim tasks before work, bind a claimed task to the real Codex task ID when available, reserve precise project-relative paths before editing, close processed messages, use idempotency keys for retried writes, inspect retention status before history replay, and prefer replay-safe subscription peek/ack over legacy consume-on-poll."
                "VibeBus is a durable fact bus; it does not interrupt a model that is already generating."
            ) -join " "
            @{
                hookSpecificOutput = @{
                    hookEventName = "SessionStart"
                    additionalContext = $context
                }
            } | ConvertTo-Json -Depth 4 -Compress
            exit 0
        }

        $parent = Split-Path -Parent $cursor
        if ([string]::IsNullOrEmpty($parent) -or $parent -eq $cursor) {
            break
        }
        $cursor = $parent
    }
} catch {
    Write-Error "VibeBus session discovery failed: $($_.Exception.Message)"
    exit 1
}

exit 0
