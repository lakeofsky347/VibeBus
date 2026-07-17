param(
    [string]$CargoPath = "cargo",
    [string]$PluginValidator = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$pluginRoot = Join-Path $repoRoot "plugins\vibebus"
$releaseBinary = Join-Path $repoRoot "target\release\vibebus.exe"
$pluginBin = Join-Path $pluginRoot "bin"
$packagedBinary = Join-Path $pluginBin "vibebus.exe"

Push-Location $repoRoot
try {
    & $CargoPath build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release failed with exit code $LASTEXITCODE"
    }

    New-Item -ItemType Directory -Force -Path $pluginBin | Out-Null
    Copy-Item -Force -LiteralPath $releaseBinary -Destination $packagedBinary

    if (-not [string]::IsNullOrWhiteSpace($PluginValidator)) {
        python $PluginValidator $pluginRoot
        if ($LASTEXITCODE -ne 0) {
            throw "plugin validation failed with exit code $LASTEXITCODE"
        }
    }

    $file = Get-Item -LiteralPath $packagedBinary
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $packagedBinary).Hash.ToLowerInvariant()
    [pscustomobject]@{
        plugin = $pluginRoot
        binary = $packagedBinary
        bytes = $file.Length
        sha256 = $hash
    } | ConvertTo-Json
} finally {
    Pop-Location
}
