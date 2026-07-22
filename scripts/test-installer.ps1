param(
    [Parameter(Mandatory = $true)]
    [string]$MsiPath,

    [string]$ExpectedVersion = "",

    [string]$PreviousMsiPath = "",

    [switch]$ExerciseLifecycle
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path

function Remove-TemporaryPath {
    param(
        [string]$Path,
        [switch]$Recurse
    )
    for ($attempt = 1; $attempt -le 10; $attempt++) {
        if (-not (Test-Path -LiteralPath $Path)) {
            return
        }
        try {
            if ($Recurse) {
                Remove-Item -Recurse -Force -LiteralPath $Path
            } else {
                Remove-Item -Force -LiteralPath $Path
            }
            return
        } catch {
            if ($attempt -eq 10) {
                Write-Warning "Could not remove temporary path after $attempt attempts: $Path"
                return
            }
            Start-Sleep -Milliseconds 250
        }
    }
}

function Invoke-MsiOperation {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("install", "uninstall")]
        [string]$Operation,
        [Parameter(Mandatory = $true)]
        [string]$PackagePath,
        [Parameter(Mandatory = $true)]
        [string]$LogPath
    )

    $verb = if ($Operation -eq "install") { "/i" } else { "/x" }
    $arguments = @(
        $verb,
        ('"' + $PackagePath + '"'),
        "/qn",
        "/norestart",
        "/L*v",
        ('"' + $LogPath + '"')
    )
    $process = Start-Process -FilePath msiexec.exe -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "MSI $Operation failed with exit code $($process.ExitCode). See $LogPath"
    }
}

function Test-UserPathEntry {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathEntry
    )

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) {
        return $false
    }
    $normalizedEntry = $PathEntry.TrimEnd('\')
    return @($userPath -split ';' | Where-Object {
        $_.Trim().TrimEnd('\') -ieq $normalizedEntry
    }).Count -gt 0
}

if ([string]::IsNullOrWhiteSpace($ExpectedVersion)) {
    $cargoText = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "Cargo.toml")
    $ExpectedVersion = [regex]::Match($cargoText, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"').Groups["version"].Value
}

Push-Location $repoRoot
try {
    & dotnet tool restore | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet tool restore failed with exit code $LASTEXITCODE."
    }
    # ICE91 warns about per-machine deployment of files in a user directory. This
    # package is intentionally Scope="perUser"; all other stock ICEs still run.
    & dotnet tool run wix msi validate -sice ICE91 $MsiPath
    if ($LASTEXITCODE -ne 0) {
        throw "WiX MSI validation failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}

$extractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("vibebus-msi-{0}" -f [guid]::NewGuid().ToString("N"))
$logPath = "$extractRoot.log"
try {
    New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null
    $msiArguments = @(
        "/a",
        ('"' + $MsiPath + '"'),
        "/qn",
        ('TARGETDIR="' + $extractRoot + '"'),
        "/L*v",
        ('"' + $logPath + '"')
    )
    $msiProcess = Start-Process -FilePath msiexec.exe -ArgumentList $msiArguments -Wait -PassThru
    if ($msiProcess.ExitCode -ne 0) {
        throw "Administrative MSI extraction failed with exit code $($msiProcess.ExitCode). See $logPath"
    }

    $expectedRelativePaths = @(
        ".agents\plugins\marketplace.json",
        "plugins\vibebus\.codex-plugin\plugin.json",
        "plugins\vibebus\.mcp.json",
        "plugins\vibebus\bin\vibebus.exe",
        "plugins\vibebus\hooks\hooks.json",
        "plugins\vibebus\scripts\hook-common.ps1",
        "plugins\vibebus\scripts\post-tool-facts.ps1",
        "plugins\vibebus\scripts\session-start.ps1",
        "plugins\vibebus\scripts\stop-handoff.ps1",
        "plugins\vibebus\skills\vibebus-coordination\SKILL.md"
    )
    $extractedBinary = Get-ChildItem -LiteralPath $extractRoot -Recurse -Filter vibebus.exe |
        Where-Object { $_.FullName -match 'plugins\\vibebus\\bin\\vibebus\.exe$' } |
        Select-Object -First 1
    if ($null -eq $extractedBinary) {
        throw "The extracted MSI does not contain plugins\vibebus\bin\vibebus.exe."
    }
    $installRoot = $extractedBinary.FullName
    for ($level = 0; $level -lt 4; $level++) {
        $installRoot = Split-Path -Parent $installRoot
    }
    foreach ($relativePath in $expectedRelativePaths) {
        if (-not (Test-Path -LiteralPath (Join-Path $installRoot $relativePath))) {
            throw "The extracted MSI is missing '$relativePath'."
        }
    }

    $binaryPath = Join-Path $installRoot "plugins\vibebus\bin\vibebus.exe"
    $versionOutput = (& $binaryPath --version 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $versionOutput -ne "vibebus $ExpectedVersion") {
        throw "Extracted binary version '$versionOutput' does not match '$ExpectedVersion'."
    }

    $lifecycleExercised = $false
    $upgradeExercised = $false
    if ($ExerciseLifecycle) {
        $lifecycleRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("vibebus-msi-lifecycle-{0}" -f [guid]::NewGuid().ToString("N"))
        $lifecycleInstallRoot = Join-Path $env:LOCALAPPDATA "Programs\VibeBus"
        $lifecyclePluginBin = Join-Path $lifecycleInstallRoot "plugins\vibebus\bin"
        $lifecycleBinary = Join-Path $lifecyclePluginBin "vibebus.exe"
        $lifecycleMarketplace = Join-Path $lifecycleInstallRoot ".agents\plugins\marketplace.json"
        $installedPackage = ""
        try {
            New-Item -ItemType Directory -Force -Path $lifecycleRoot | Out-Null
            if (-not [string]::IsNullOrWhiteSpace($PreviousMsiPath)) {
                $PreviousMsiPath = (Resolve-Path -LiteralPath $PreviousMsiPath).Path
                Invoke-MsiOperation -Operation install -PackagePath $PreviousMsiPath -LogPath (Join-Path $lifecycleRoot "previous-install.log")
                $installedPackage = $PreviousMsiPath
                $upgradeExercised = $true
            }

            Invoke-MsiOperation -Operation install -PackagePath $MsiPath -LogPath (Join-Path $lifecycleRoot "install-or-upgrade.log")
            $installedPackage = $MsiPath
            foreach ($requiredPath in @($lifecycleBinary, $lifecycleMarketplace)) {
                if (-not (Test-Path -LiteralPath $requiredPath)) {
                    throw "Installed MSI is missing '$requiredPath'."
                }
            }
            if (-not (Test-UserPathEntry -PathEntry $lifecyclePluginBin)) {
                throw "Installed MSI did not add '$lifecyclePluginBin' to the user PATH."
            }
            $installedVersion = (& $lifecycleBinary --version 2>&1 | Out-String).Trim()
            if ($LASTEXITCODE -ne 0 -or $installedVersion -ne "vibebus $ExpectedVersion") {
                throw "Installed binary version '$installedVersion' does not match '$ExpectedVersion'."
            }

            Invoke-MsiOperation -Operation uninstall -PackagePath $installedPackage -LogPath (Join-Path $lifecycleRoot "uninstall.log")
            $installedPackage = ""
            if ((Test-Path -LiteralPath $lifecycleInstallRoot) -or (Test-Path -LiteralPath $lifecycleMarketplace)) {
                throw "Uninstall left VibeBus marketplace files behind."
            }
            if (Test-UserPathEntry -PathEntry $lifecyclePluginBin) {
                throw "Uninstall left the VibeBus plugin directory on the user PATH."
            }
            $lifecycleExercised = $true
        } finally {
            if (-not [string]::IsNullOrWhiteSpace($installedPackage)) {
                try {
                    Invoke-MsiOperation -Operation uninstall -PackagePath $installedPackage -LogPath (Join-Path $lifecycleRoot "cleanup-uninstall.log")
                } catch {
                    Write-Warning "Lifecycle cleanup uninstall failed: $($_.Exception.Message)"
                }
            }
            Remove-TemporaryPath -Path $lifecycleRoot -Recurse
        }
    }

    [pscustomobject]@{
        ok = $true
        msi = $MsiPath
        version = $ExpectedVersion
        extractedFilesVerified = $expectedRelativePaths.Count
        signatureStatus = [string](Get-AuthenticodeSignature -LiteralPath $MsiPath).Status
        lifecycleExercised = $lifecycleExercised
        upgradeExercised = $upgradeExercised
    } | ConvertTo-Json
} finally {
    Remove-TemporaryPath -Path $extractRoot -Recurse
    Remove-TemporaryPath -Path $logPath
}
