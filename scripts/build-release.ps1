param(
    [string]$Version = "",
    [string]$CargoPath = "cargo",
    [string]$OutputDirectory = "dist",
    [string]$SourceRef = "HEAD",
    [switch]$Sign
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$helper = Join-Path $PSScriptRoot "plugin_package.py"

function Invoke-PythonHelper {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)
    $output = & python $helper @Arguments
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "plugin_package.py $($Arguments -join ' ') failed with exit code $exitCode."
    }
    return ($output | Out-String).Trim()
}

$sourceRevision = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceRevision -notmatch '^[0-9a-f]{40}$') {
    throw "Git source revision could not be resolved."
}
$sourceRefRevision = (& git -C $repoRoot rev-parse "$SourceRef^{commit}").Trim()
if ($LASTEXITCODE -ne 0 -or $sourceRefRevision -notmatch '^[0-9a-f]{40}$') {
    throw "SourceRef must resolve to a Git commit."
}
if ($sourceRefRevision -ne $sourceRevision) {
    throw "SourceRef must resolve to the checked-out source revision."
}
$workingTree = @(& git -C $repoRoot status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0) {
    throw "Git working tree status could not be resolved."
}
if ($workingTree.Count -ne 0) {
    throw "Windows release assets require a clean working tree so binaries, source evidence, and sourceRevision agree."
}

$outputRoot = Invoke-PythonHelper @("output-root", "--repo-root", $repoRoot, "--output-root", $OutputDirectory)
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$cargoVersion = Invoke-PythonHelper @("version", "--repo-root", $repoRoot, "--cargo", $CargoPath)
if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = $cargoVersion
}
if ($Version -notmatch '^\d+\.\d+\.\d+$' -or $Version -ne $cargoVersion) {
    throw "Release and Cargo versions must match (release=$Version cargo=$cargoVersion)."
}
$expectedStagingName = "VibeBus-$Version-windows-x64"
$expectedStagingRoot = [IO.Path]::GetFullPath((Join-Path (Join-Path $outputRoot "staging") $expectedStagingName))

$packageArguments = @(
    "-CargoPath", $CargoPath,
    "-OutputRoot", $outputRoot,
    "-ExpectedVersion", $Version
)
$packageOutput = & (Join-Path $PSScriptRoot "package-plugin.ps1") @packageArguments
$packageExitCode = $LASTEXITCODE
if ($packageExitCode -ne 0) {
    throw "package-plugin.ps1 failed with exit code $packageExitCode."
}
$package = (($packageOutput | Out-String).Trim() | ConvertFrom-Json -ErrorAction Stop)
$StagingRoot = [IO.Path]::GetFullPath([string]$package.StagingRoot)
if (-not $StagingRoot.Equals($expectedStagingRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "StagingRoot must be the controlled package output $expectedStagingRoot."
}

$pluginRoot = Join-Path $StagingRoot "plugins\vibebus"
$marketplacePath = Join-Path $StagingRoot ".agents\plugins\marketplace.json"
$pluginManifestPath = Join-Path $pluginRoot ".codex-plugin\plugin.json"
$pluginManifest = Get-Content -Raw -LiteralPath $pluginManifestPath | ConvertFrom-Json -ErrorAction Stop
$stagedVersion = [string]$pluginManifest.version
if ($Version -notmatch '^\d+\.\d+\.\d+$' -or $Version -ne $cargoVersion -or $Version -ne $stagedVersion) {
    throw "Release, Cargo, and staged plugin versions must match (release=$Version cargo=$cargoVersion plugin=$stagedVersion)."
}
if ((Split-Path -Leaf $StagingRoot) -ne $expectedStagingName) {
    throw "Windows staging root must be named $expectedStagingName."
}

& (Join-Path $PSScriptRoot "validate-plugin.ps1") `
    -StagingRoot $StagingRoot -CargoPath $CargoPath | Out-Null
$packagedBinary = Join-Path $pluginRoot "bin\vibebus.exe"
if ($Sign) {
    & (Join-Path $PSScriptRoot "sign-windows.ps1") -Path $packagedBinary | Out-Null
    & (Join-Path $PSScriptRoot "validate-plugin.ps1") `
        -StagingRoot $StagingRoot -CargoPath $CargoPath -RequireAuthenticodeSignature | Out-Null
}

$wixIntermediate = Join-Path $outputRoot "wixobj"
if (Test-Path -LiteralPath $wixIntermediate) {
    $wixItem = Get-Item -Force -LiteralPath $wixIntermediate
    if (($wixItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "WiX intermediate directory must not be a reparse point: $wixIntermediate"
    }
    [IO.Directory]::Delete($wixIntermediate, $true)
}
Push-Location $repoRoot
try {
    & dotnet tool restore
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet tool restore failed with exit code $LASTEXITCODE."
    }
    $msiPath = Join-Path $outputRoot "VibeBus-$Version-windows-x64.msi"
    & dotnet tool run wix build "installer\Package.wxs" -arch x64 `
        -d "Version=$Version" -d "PayloadRoot=$StagingRoot" `
        -intermediateFolder $wixIntermediate -pdbtype none -out $msiPath
    if ($LASTEXITCODE -ne 0) {
        throw "WiX build failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
if ($Sign) {
    & (Join-Path $PSScriptRoot "sign-windows.ps1") -Path $msiPath | Out-Null
}

$portableZip = Join-Path $outputRoot "VibeBus-$Version-windows-x64.zip"
$pluginZip = Join-Path $outputRoot "VibeBus-Codex-plugin-$Version-windows-x64.zip"
$sourceZip = Join-Path $outputRoot "VibeBus-$Version-source.zip"
Invoke-PythonHelper @(
    "archive",
    "--repo-root", $repoRoot,
    "--output-root", $outputRoot,
    "--staging-root", $StagingRoot,
    "--portable", $portableZip,
    "--plugin", $pluginZip,
    "--portable-format", "zip"
) | Out-Null
Invoke-PythonHelper @(
    "validate-archive",
    "--repo-root", $repoRoot,
    "--archive", $portableZip,
    "--kind", "portable",
    "--platform", "windows",
    "--arch", "x64",
    "--cargo", $CargoPath
) | Out-Null
Invoke-PythonHelper @(
    "validate-archive",
    "--repo-root", $repoRoot,
    "--archive", $pluginZip,
    "--kind", "plugin",
    "--platform", "windows",
    "--arch", "x64",
    "--cargo", $CargoPath
) | Out-Null

if (Test-Path -LiteralPath $sourceZip) {
    [IO.File]::Delete($sourceZip)
}
& git -C $repoRoot archive --format=zip "--prefix=VibeBus-$Version/" "--output=$sourceZip" $SourceRef
if ($LASTEXITCODE -ne 0) {
    throw "git archive failed for source ref '$SourceRef' with exit code $LASTEXITCODE."
}

$releaseFiles = @($msiPath, $portableZip, $pluginZip, $sourceZip)
$hashLines = foreach ($file in $releaseFiles) {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
    "$hash  $([IO.Path]::GetFileName($file))"
}
$checksumsPath = Join-Path $outputRoot "SHA256SUMS.txt"
$utf8NoBom = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllLines($checksumsPath, [string[]]$hashLines, $utf8NoBom)

$manifestPath = Join-Path $outputRoot "release-manifest.json"
$manifest = [ordered]@{
    version = $Version
    platform = "windows-x64"
    signed = [bool]$Sign
    sourceRevision = $sourceRevision
    sourceRef = $SourceRef
    staging = $expectedStagingName
    binaryMachine = "AMD64"
    binarySha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $packagedBinary).Hash.ToLowerInvariant()
    generatedAt = [DateTimeOffset]::UtcNow.ToString("O")
    artifacts = foreach ($file in $releaseFiles) {
        $item = Get-Item -LiteralPath $file
        [ordered]@{
            name = $item.Name
            bytes = $item.Length
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
        }
    }
    checksums = [IO.Path]::GetFileName($checksumsPath)
}
$manifestJson = $manifest | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText($manifestPath, $manifestJson + [Environment]::NewLine, $utf8NoBom)
$manifestJson
