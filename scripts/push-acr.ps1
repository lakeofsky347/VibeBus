param(
    [Parameter(Mandatory = $true)]
    [string]$Repository,
    [string]$SourceImage = "",
    [string]$Tag = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$cargoText = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "Cargo.toml")
$version = [regex]::Match($cargoText, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"').Groups["version"].Value
if ([string]::IsNullOrWhiteSpace($version)) {
    throw "Cargo package version could not be resolved"
}
if ([string]::IsNullOrWhiteSpace($Tag)) {
    $Tag = $version
}
if ([string]::IsNullOrWhiteSpace($SourceImage)) {
    $SourceImage = "vibebus:$version-local"
}

if ($Repository -notmatch '^[a-z0-9][a-z0-9.-]*\.aliyuncs\.com/[a-z0-9._/-]+$') {
    throw "Repository must be a complete Alibaba Cloud ACR path without a tag"
}
if ($Tag -notmatch '^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$') {
    throw "Tag is not a valid container tag"
}

$registry = $Repository.Split('/')[0]
$configPath = Join-Path $env:USERPROFILE ".docker\config.json"
if (-not (Test-Path -LiteralPath $configPath)) {
    throw "Docker credentials are not configured; run docker login $registry in your own terminal"
}
$config = Get-Content -Raw -LiteralPath $configPath | ConvertFrom-Json
$knownRegistries = @($config.auths.PSObject.Properties.Name)
if ($knownRegistries -notcontains $registry -and $knownRegistries -notcontains "https://$registry") {
    throw "Docker is not logged in to $registry; run docker login $registry in your own terminal"
}

& docker image inspect $SourceImage *> $null
if ($LASTEXITCODE -ne 0) {
    throw "source image '$SourceImage' does not exist"
}

$remoteImage = "${Repository}:${Tag}"
& docker tag $SourceImage $remoteImage
if ($LASTEXITCODE -ne 0) {
    throw "failed to tag $remoteImage"
}

$previousErrorActionPreference = $ErrorActionPreference
try {
    $ErrorActionPreference = "Continue"
    $pushOutput = & docker push $remoteImage 2>&1
    $pushExitCode = $LASTEXITCODE
}
finally {
    $ErrorActionPreference = $previousErrorActionPreference
}
if ($pushExitCode -ne 0) {
    throw "docker push failed: $($pushOutput -join [Environment]::NewLine)"
}
$digestMatch = [regex]::Match(($pushOutput -join "`n"), 'digest:\s*(sha256:[0-9a-f]{64})')
if (-not $digestMatch.Success) {
    throw "push completed but no manifest digest was found in Docker output"
}

[pscustomobject]@{
    ok = $true
    image = $remoteImage
    digest = $digestMatch.Groups[1].Value
    pushedAt = [DateTimeOffset]::UtcNow.ToString("o")
} | ConvertTo-Json
