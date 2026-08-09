param(
    [string]$CargoPath = "cargo",
    [string]$OutputRoot = "dist",
    [string]$ExpectedVersion = "",
    [string]$PluginValidator = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$helper = Join-Path $PSScriptRoot "plugin_package.py"
$sourceBin = Join-Path $repoRoot "plugins\vibebus\bin"

function Invoke-PythonHelper {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)
    $output = & python $helper @Arguments
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "plugin_package.py $($Arguments -join ' ') failed with exit code $exitCode."
    }
    return ($output | Out-String).Trim()
}

if ($CargoPath -eq "cargo" -and $null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $localCargo = Join-Path $repoRoot ".tools\cargo\bin\cargo.exe"
    if (-not (Test-Path -LiteralPath $localCargo -PathType Leaf)) {
        throw "Cargo was not found on PATH or at '$localCargo'."
    }
    $env:CARGO_HOME = Join-Path $repoRoot ".tools\cargo"
    $env:RUSTUP_HOME = Join-Path $repoRoot ".tools\rustup"
    $CargoPath = $localCargo
}
if ($null -eq (Get-Command python -ErrorAction SilentlyContinue)) {
    throw "Python is required for the shared plugin packaging contract."
}

$trackedBinary = & git -C $repoRoot ls-files --error-unmatch -- "plugins/vibebus/bin/vibebus.exe" 2>$null
if ($LASTEXITCODE -eq 0) {
    throw "plugins/vibebus/bin/vibebus.exe must never be tracked."
}
if (Test-Path -LiteralPath $sourceBin) {
    $sourceEntries = @(Get-ChildItem -Force -LiteralPath $sourceBin)
    if ($sourceEntries.Count -ne 0) {
        throw "Source plugin bin must be empty; packaging writes binaries only to staging."
    }
}

$version = Invoke-PythonHelper @("version", "--repo-root", $repoRoot, "--cargo", $CargoPath)
if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion) -and $ExpectedVersion -ne $version) {
    throw "Expected version $ExpectedVersion does not match Cargo metadata version $version."
}
$resolvedOutput = Invoke-PythonHelper @("output-root", "--repo-root", $repoRoot, "--output-root", $OutputRoot)
$targetDirectory = Invoke-PythonHelper @("target-directory", "--repo-root", $repoRoot, "--cargo", $CargoPath)
$rustTarget = "x86_64-pc-windows-msvc"
$releaseBinary = Join-Path $targetDirectory "$rustTarget\release\vibebus.exe"

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        & $CargoPath build --release --locked --package vibebus --bin vibebus --target $rustTarget
        $exitCode = $LASTEXITCODE
        if ($exitCode -ne 0) {
            throw "cargo build for $rustTarget failed with exit code $exitCode."
        }
    }
    if (-not (Test-Path -LiteralPath $releaseBinary -PathType Leaf)) {
        throw "Expected Windows build output does not exist: $releaseBinary"
    }

    $stageJson = Invoke-PythonHelper @(
        "stage",
        "--repo-root", $repoRoot,
        "--output-root", $resolvedOutput,
        "--platform", "windows",
        "--arch", "x64",
        "--binary", $releaseBinary,
        "--cargo", $CargoPath
    )
    $stage = $stageJson | ConvertFrom-Json -ErrorAction Stop

    $validationJson = Invoke-PythonHelper @(
        "validate",
        "--repo-root", $repoRoot,
        "--plugin-root", $stage.plugin,
        "--marketplace-path", $stage.marketplace,
        "--platform", "windows",
        "--arch", "x64",
        "--cargo", $CargoPath
    )
    $validation = $validationJson | ConvertFrom-Json -ErrorAction Stop
    if (-not [string]::IsNullOrWhiteSpace($PluginValidator)) {
        & python $PluginValidator $stage.plugin
        $exitCode = $LASTEXITCODE
        if ($exitCode -ne 0) {
            throw "Additional plugin validator failed with exit code $exitCode."
        }
    }

    if (Test-Path -LiteralPath $sourceBin) {
        $sourceEntries = @(Get-ChildItem -Force -LiteralPath $sourceBin)
        if ($sourceEntries.Count -ne 0) {
            throw "Packaging contaminated the source plugin bin directory."
        }
    }
    [pscustomobject]@{
        ok = $true
        Version = [string]$stage.version
        Platform = "windows-x64"
        StagingRoot = [string]$stage.staging
        PluginRoot = [string]$stage.plugin
        Marketplace = [string]$stage.marketplace
        Binary = [string]$stage.binary
        BinarySha256 = [string]$validation.binarySha256
        McpTools = [int]$validation.tools
    } | ConvertTo-Json -Compress
}
finally {
    Pop-Location
}
