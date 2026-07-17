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
Require-Path (Join-Path $PluginRoot "scripts\session-start.ps1") "SessionStart script"

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
