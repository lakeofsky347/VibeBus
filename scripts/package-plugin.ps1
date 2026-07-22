param(
    [string]$CargoPath = "cargo",
    [string]$PluginValidator = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$pluginRoot = Join-Path $repoRoot "plugins\vibebus"
$releaseBinary = Join-Path $repoRoot "target\release\vibebus.exe"
$pluginBin = Join-Path $pluginRoot "bin"
$packagedBinary = Join-Path $pluginBin "vibebus.exe"

$trackedBinary = & git -C $repoRoot ls-files --error-unmatch -- "plugins/vibebus/bin/vibebus.exe" 2>$null
if ($LASTEXITCODE -eq 0) {
    throw "plugins/vibebus/bin/vibebus.exe must be generated from source and must not be tracked."
}

if ($CargoPath -eq "cargo" -and $null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $localCargo = Join-Path $repoRoot ".tools\cargo\bin\cargo.exe"
    $localCargoHome = Join-Path $repoRoot ".tools\cargo"
    $localRustupHome = Join-Path $repoRoot ".tools\rustup"
    if (-not (Test-Path -LiteralPath $localCargo)) {
        throw "Cargo was not found on PATH or at '$localCargo'."
    }
    $env:CARGO_HOME = $localCargoHome
    $env:RUSTUP_HOME = $localRustupHome
    $CargoPath = $localCargo
}

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        & $CargoPath build --release --locked
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --release --locked failed with exit code $LASTEXITCODE"
        }
    }

    New-Item -ItemType Directory -Force -Path $pluginBin | Out-Null
    Copy-Item -Force -LiteralPath $releaseBinary -Destination $packagedBinary

    if (-not [string]::IsNullOrWhiteSpace($PluginValidator)) {
        python $PluginValidator $pluginRoot
        if ($LASTEXITCODE -ne 0) {
            throw "plugin validation failed with exit code $LASTEXITCODE"
        }
    } else {
        & (Join-Path $PSScriptRoot "validate-plugin.ps1") -PluginRoot $pluginRoot | Out-Null
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
