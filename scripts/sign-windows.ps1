param(
    [Parameter(Mandatory = $true)]
    [string[]]$Path,

    [string]$TimestampUrl = ""
)

$ErrorActionPreference = "Stop"
$certificateBase64 = $env:VIBEBUS_SIGNING_CERTIFICATE_BASE64
$certificatePassword = $env:VIBEBUS_SIGNING_CERTIFICATE_PASSWORD

if ([string]::IsNullOrWhiteSpace($certificateBase64)) {
    throw "VIBEBUS_SIGNING_CERTIFICATE_BASE64 is required for signing."
}
if ([string]::IsNullOrWhiteSpace($certificatePassword)) {
    throw "VIBEBUS_SIGNING_CERTIFICATE_PASSWORD is required for signing."
}
if ([string]::IsNullOrWhiteSpace($TimestampUrl)) {
    $TimestampUrl = $env:VIBEBUS_TIMESTAMP_URL
}
if ([string]::IsNullOrWhiteSpace($TimestampUrl)) {
    $TimestampUrl = "http://timestamp.digicert.com"
}

$signTool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending |
    Select-Object -First 1
if ($null -eq $signTool) {
    throw "SignTool was not found in the Windows 10 SDK."
}

$resolvedPaths = foreach ($item in $Path) {
    (Resolve-Path -LiteralPath $item).Path
}
$pfxPath = Join-Path ([System.IO.Path]::GetTempPath()) ("vibebus-signing-{0}.pfx" -f [guid]::NewGuid().ToString("N"))

try {
    [System.IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($certificateBase64))
    foreach ($item in $resolvedPaths) {
        & $signTool.FullName sign /f $pfxPath /p $certificatePassword /fd SHA256 /tr $TimestampUrl /td SHA256 /d "VibeBus" $item
        if ($LASTEXITCODE -ne 0) {
            throw "SignTool signing failed for '$item' with exit code $LASTEXITCODE."
        }
        & $signTool.FullName verify /pa /tw /v $item
        if ($LASTEXITCODE -ne 0) {
            throw "SignTool verification failed for '$item' with exit code $LASTEXITCODE."
        }
    }
} finally {
    if (Test-Path -LiteralPath $pfxPath) {
        Remove-Item -Force -LiteralPath $pfxPath
    }
}

[pscustomobject]@{
    signed = $true
    timestampUrl = $TimestampUrl
    files = $resolvedPaths
} | ConvertTo-Json
