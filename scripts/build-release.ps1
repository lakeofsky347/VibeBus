param(
    [string]$Version = "",
    [string]$CargoPath = "cargo",
    [string]$OutputDirectory = "dist",
    [switch]$SkipCargoBuild,
    [switch]$Sign
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$cargoManifestPath = Join-Path $repoRoot "Cargo.toml"
$pluginManifestPath = Join-Path $repoRoot "plugins\vibebus\.codex-plugin\plugin.json"

$cargoText = Get-Content -Raw -LiteralPath $cargoManifestPath
$cargoVersionMatch = [regex]::Match($cargoText, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"')
if (-not $cargoVersionMatch.Success) {
    throw "Could not read the package version from Cargo.toml."
}
$cargoVersion = $cargoVersionMatch.Groups["version"].Value
$pluginVersion = (Get-Content -Raw -LiteralPath $pluginManifestPath | ConvertFrom-Json).version

if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = $cargoVersion
}
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Release version must be semantic X.Y.Z."
}
if ($Version -ne $cargoVersion -or $Version -ne $pluginVersion) {
    throw "Release, Cargo, and plugin versions must match (release=$Version cargo=$cargoVersion plugin=$pluginVersion)."
}

if ([System.IO.Path]::IsPathRooted($OutputDirectory)) {
    $outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
} else {
    $outputRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory))
}
$repoPrefix = $repoRoot.TrimEnd('\') + '\'
if (-not $outputRoot.StartsWith($repoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputDirectory must resolve inside the repository: $outputRoot"
}
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

$stagingParent = Join-Path $outputRoot "staging"
$stagingRoot = Join-Path $stagingParent "VibeBus-$Version"
$wixIntermediate = Join-Path $outputRoot "wixobj"
foreach ($pathToClear in @($stagingRoot, $wixIntermediate)) {
    $fullPath = [System.IO.Path]::GetFullPath($pathToClear)
    if (-not $fullPath.StartsWith($outputRoot.TrimEnd('\') + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clear a path outside the release output directory: $fullPath"
    }
    if (Test-Path -LiteralPath $fullPath) {
        Remove-Item -Recurse -Force -LiteralPath $fullPath
    }
}

if (-not $SkipCargoBuild) {
    & (Join-Path $PSScriptRoot "package-plugin.ps1") -CargoPath $CargoPath
}
& (Join-Path $PSScriptRoot "validate-plugin.ps1") -PluginRoot (Join-Path $repoRoot "plugins\vibebus") | Out-Null

$packagedBinary = Join-Path $repoRoot "plugins\vibebus\bin\vibebus.exe"
if ($Sign) {
    & (Join-Path $PSScriptRoot "sign-windows.ps1") -Path $packagedBinary | Out-Null
}

New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null
Copy-Item -Recurse -Force -LiteralPath (Join-Path $repoRoot ".agents") -Destination (Join-Path $stagingRoot ".agents")
New-Item -ItemType Directory -Force -Path (Join-Path $stagingRoot "plugins") | Out-Null
Copy-Item -Recurse -Force -LiteralPath (Join-Path $repoRoot "plugins\vibebus") -Destination (Join-Path $stagingRoot "plugins\vibebus")
Copy-Item -Force -LiteralPath (Join-Path $repoRoot "LICENSE") -Destination (Join-Path $stagingRoot "LICENSE")
Copy-Item -Force -LiteralPath (Join-Path $repoRoot "README.md") -Destination (Join-Path $stagingRoot "README.md")

Push-Location $repoRoot
try {
    & dotnet tool restore
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet tool restore failed with exit code $LASTEXITCODE."
    }

    $msiPath = Join-Path $outputRoot "VibeBus-$Version-windows-x64.msi"
    & dotnet tool run wix build "installer\Package.wxs" -arch x64 -d "Version=$Version" -d "PayloadRoot=$stagingRoot" -intermediateFolder $wixIntermediate -pdbtype none -out $msiPath
    if ($LASTEXITCODE -ne 0) {
        throw "WiX build failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}

if ($Sign) {
    & (Join-Path $PSScriptRoot "sign-windows.ps1") -Path $msiPath | Out-Null
}

$portableZip = Join-Path $outputRoot "VibeBus-$Version-windows-x64.zip"
$pluginZip = Join-Path $outputRoot "VibeBus-Codex-plugin-$Version.zip"
foreach ($archive in @($portableZip, $pluginZip)) {
    if (Test-Path -LiteralPath $archive) {
        Remove-Item -Force -LiteralPath $archive
    }
}
Compress-Archive -CompressionLevel Optimal -LiteralPath @(
    (Join-Path $stagingRoot ".agents"),
    (Join-Path $stagingRoot "plugins"),
    (Join-Path $stagingRoot "LICENSE"),
    (Join-Path $stagingRoot "README.md")
) -DestinationPath $portableZip
Compress-Archive -CompressionLevel Optimal -LiteralPath (Join-Path $stagingRoot "plugins\vibebus") -DestinationPath $pluginZip

$releaseFiles = @($msiPath, $portableZip, $pluginZip)
$hashLines = foreach ($file in $releaseFiles) {
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
    "$hash  $([System.IO.Path]::GetFileName($file))"
}
$checksumsPath = Join-Path $outputRoot "SHA256SUMS.txt"
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllLines($checksumsPath, [string[]]$hashLines, $utf8NoBom)

$manifestPath = Join-Path $outputRoot "release-manifest.json"
$manifest = [ordered]@{
    version = $Version
    platform = "windows-x64"
    signed = [bool]$Sign
    generatedAt = [DateTimeOffset]::UtcNow.ToString("O")
    artifacts = foreach ($file in $releaseFiles) {
        $item = Get-Item -LiteralPath $file
        [ordered]@{
            name = $item.Name
            bytes = $item.Length
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
        }
    }
    checksums = [System.IO.Path]::GetFileName($checksumsPath)
}
$manifestJson = $manifest | ConvertTo-Json -Depth 5
[System.IO.File]::WriteAllText($manifestPath, $manifestJson + [Environment]::NewLine, $utf8NoBom)
$manifestJson
