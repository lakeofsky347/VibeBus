param(
    [string]$PluginRoot = "",
    [string]$StagingRoot = "",
    [string]$MarketplacePath = "",
    [string]$CargoPath = "cargo",
    [switch]$RequireAuthenticodeSignature
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$helper = Join-Path $PSScriptRoot "plugin_package.py"

function Assert-NoReparseAncestor {
    param([Parameter(Mandatory = $true)][string]$Path)
    $current = [IO.Path]::GetFullPath($Path)
    while (-not [string]::IsNullOrWhiteSpace($current)) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -Force -LiteralPath $current
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Windows staging path must not traverse a reparse point: $current"
            }
        }
        $parent = Split-Path -Parent $current
        if ([string]::IsNullOrWhiteSpace($parent) -or $parent -eq $current) {
            break
        }
        $current = $parent
    }
}

if (-not [string]::IsNullOrWhiteSpace($StagingRoot)) {
    Assert-NoReparseAncestor -Path $StagingRoot
    $StagingRoot = (Resolve-Path -LiteralPath $StagingRoot).Path
    $PluginRoot = Join-Path $StagingRoot "plugins\vibebus"
}
if ([string]::IsNullOrWhiteSpace($PluginRoot)) {
    throw "PluginRoot or StagingRoot is required; source plugin validation is not supported."
}
Assert-NoReparseAncestor -Path $PluginRoot
$PluginRoot = (Resolve-Path -LiteralPath $PluginRoot).Path
if ([string]::IsNullOrWhiteSpace($MarketplacePath)) {
    $staging = Split-Path -Parent (Split-Path -Parent $PluginRoot)
    $MarketplacePath = Join-Path $staging ".agents\plugins\marketplace.json"
}
$MarketplacePath = (Resolve-Path -LiteralPath $MarketplacePath).Path

$stagingRootResolved = Split-Path -Parent (Split-Path -Parent $PluginRoot)
foreach ($item in @(Get-Item -Force -LiteralPath $stagingRootResolved) + @(Get-ChildItem -Force -Recurse -LiteralPath $stagingRootResolved)) {
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Windows staging must not contain reparse points: $($item.FullName)"
    }
}

$output = & python $helper validate `
    --repo-root $repoRoot `
    --plugin-root $PluginRoot `
    --marketplace-path $MarketplacePath `
    --platform windows `
    --arch x64 `
    --cargo $CargoPath
$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) {
    throw "Windows plugin validation failed with exit code $exitCode."
}
$validation = (($output | Out-String).Trim() | ConvertFrom-Json -ErrorAction Stop)

if ($RequireAuthenticodeSignature) {
    $signature = Get-AuthenticodeSignature -LiteralPath $validation.binary
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Staged executable does not have a valid Authenticode signature: $($signature.Status)"
    }
}
$validation | ConvertTo-Json -Compress
