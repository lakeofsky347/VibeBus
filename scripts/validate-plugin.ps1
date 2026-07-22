param(
    [string]$PluginRoot = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($PluginRoot)) {
    $PluginRoot = Join-Path $repoRoot "plugins\vibebus"
}
$PluginRoot = (Resolve-Path -LiteralPath $PluginRoot).Path

function Require-Path {
    param([string]$Path, [string]$Label)
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "$Label does not exist: $Path"
    }
}

$manifestPath = Join-Path $PluginRoot ".codex-plugin\plugin.json"
$mcpPath = Join-Path $PluginRoot ".mcp.json"
$hooksPath = Join-Path $PluginRoot "hooks\hooks.json"
$skillRoot = Join-Path $PluginRoot "skills\vibebus-coordination"
$skillPath = Join-Path $skillRoot "SKILL.md"
$binaryPath = Join-Path $PluginRoot "bin\vibebus.exe"

Require-Path $manifestPath "Plugin manifest"
Require-Path $mcpPath "MCP configuration"
Require-Path $hooksPath "Hook configuration"
Require-Path $skillPath "Coordination skill"
Require-Path $binaryPath "Packaged executable"

$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
if ($manifest.name -ne "vibebus") {
    throw "Plugin name must be 'vibebus'."
}
if ($manifest.version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Plugin version must be semantic X.Y.Z."
}
foreach ($field in @("description", "license", "skills", "mcpServers")) {
    if ([string]::IsNullOrWhiteSpace([string]$manifest.$field)) {
        throw "Plugin manifest field '$field' is required."
    }
}
if ([string]::IsNullOrWhiteSpace([string]$manifest.author.name)) {
    throw "Plugin author.name is required."
}
$assetFields = @("composerIcon", "logo")
foreach ($field in $assetFields) {
    $assetReference = [string]$manifest.interface.$field
    if ([string]::IsNullOrWhiteSpace($assetReference) -or $assetReference -notmatch '^\./assets/[^/]+\.png$') {
        throw "Plugin interface.$field must be a relative PNG path under ./assets/."
    }
    $assetPath = Join-Path $PluginRoot ($assetReference -replace '^\./', '')
    Require-Path $assetPath "Plugin interface.$field asset"
    if ((Get-Item -LiteralPath $assetPath).Length -le 0) {
        throw "Plugin interface.$field asset is empty: $assetPath"
    }
}

$mcp = Get-Content -Raw -LiteralPath $mcpPath | ConvertFrom-Json
$server = $mcp.mcpServers.vibebus
if ($null -eq $server -or $server.command -ne "./bin/vibebus.exe" -or @($server.args).Count -ne 1 -or $server.args[0] -ne "mcp") {
    throw "MCP configuration must launch ./bin/vibebus.exe mcp."
}

$hooks = Get-Content -Raw -LiteralPath $hooksPath | ConvertFrom-Json
$sessionStart = @($hooks.hooks.SessionStart)
if ($sessionStart.Count -eq 0) {
    throw "SessionStart hook is required."
}
$windowsCommand = [string]$sessionStart[0].hooks[0].commandWindows
if ($windowsCommand -notmatch 'session-start\.ps1') {
    throw "SessionStart hook must invoke session-start.ps1 on Windows."
}
if ([string]$sessionStart[0].hooks[0].command -notmatch 'vibebus.*hook session-start') {
    throw "SessionStart hook must invoke the native VibeBus hook on Unix."
}
Require-Path (Join-Path $PluginRoot "scripts\session-start.ps1") "SessionStart script"
$postToolUse = @($hooks.hooks.PostToolUse)
if ($postToolUse.Count -ne 1 -or $postToolUse[0].matcher -ne "^Bash$") {
    throw "Exactly one Bash PostToolUse hook is required."
}
if ([string]$postToolUse[0].hooks[0].commandWindows -notmatch 'post-tool-facts\.ps1') {
    throw "PostToolUse hook must invoke post-tool-facts.ps1 on Windows."
}
if ([string]$postToolUse[0].hooks[0].command -notmatch 'vibebus.*hook post-tool-use') {
    throw "PostToolUse hook must invoke the native VibeBus hook on Unix."
}
$stop = @($hooks.hooks.Stop)
if ($stop.Count -ne 1 -or [string]$stop[0].hooks[0].commandWindows -notmatch 'stop-handoff\.ps1') {
    throw "Stop hook must invoke stop-handoff.ps1 on Windows."
}
if ([string]$stop[0].hooks[0].command -notmatch 'vibebus.*hook stop') {
    throw "Stop hook must invoke the native VibeBus hook on Unix."
}
Require-Path (Join-Path $PluginRoot "scripts\hook-common.ps1") "Hook helper script"
Require-Path (Join-Path $PluginRoot "scripts\post-tool-facts.ps1") "PostToolUse script"
Require-Path (Join-Path $PluginRoot "scripts\stop-handoff.ps1") "Stop script"

$skillText = Get-Content -Raw -LiteralPath $skillPath
if ($skillText -notmatch '(?s)^---\r?\nname:\s*vibebus-coordination\r?\ndescription:\s*.+?\r?\n---\r?\n') {
    throw "SKILL.md must have matching name and a description in YAML frontmatter."
}

$versionOutput = (& $binaryPath --version 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Packaged executable --version failed with exit code $LASTEXITCODE."
}
if ($versionOutput -ne "vibebus $($manifest.version)") {
    throw "Binary version '$versionOutput' does not match plugin version '$($manifest.version)'."
}

[pscustomobject]@{
    ok = $true
    plugin = $PluginRoot
    version = $manifest.version
    binary = $binaryPath
    binarySha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $binaryPath).Hash.ToLowerInvariant()
} | ConvertTo-Json
