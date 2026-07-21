param(
    [string]$ImageTag = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-DockerChecked {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & docker @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($exitCode -ne 0) {
        throw "docker $($Arguments -join ' ') failed: $($output -join [Environment]::NewLine)"
    }
    return @($output)
}

function Grant-ContainerTestAccess {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ParentPath,
        [Parameter(Mandatory = $true)]
        [string[]]$WritablePaths
    )

    if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
        return
    }

    # Linux bind mounts preserve host permissions. Grant only the disposable
    # acceptance directories to the image's fixed non-root UID.
    if (Get-Command setfacl -ErrorAction SilentlyContinue) {
        & setfacl -m "u:10001:--x" -- $ParentPath
        if ($LASTEXITCODE -ne 0) {
            throw "failed to grant container traversal access to $ParentPath"
        }
        foreach ($path in $WritablePaths) {
            & setfacl -m "u:10001:rwx" -- $path
            if ($LASTEXITCODE -ne 0) {
                throw "failed to grant container write access to $path"
            }
        }
        return
    }

    & chmod "0701" -- $ParentPath
    if ($LASTEXITCODE -ne 0) {
        throw "failed to grant container traversal access to $ParentPath"
    }
    & chmod "0707" -- $WritablePaths
    if ($LASTEXITCODE -ne 0) {
        throw "failed to grant container write access to disposable acceptance directories"
    }
}

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw "docker CLI was not found"
}

$serverOs = (& docker info --format '{{.OSType}}' 2>$null).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Docker Engine is not ready"
}
if ($serverOs -ne "linux") {
    throw "VibeBus container acceptance requires a Linux Docker Engine"
}

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$sourceRevision = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceRevision -notmatch '^[0-9a-f]{40}$') {
    throw "Git source revision could not be resolved"
}
$cargoText = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "Cargo.toml")
$expectedVersion = [regex]::Match($cargoText, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"').Groups["version"].Value
if ([string]::IsNullOrWhiteSpace($expectedVersion)) {
    throw "Cargo package version could not be resolved"
}
if ([string]::IsNullOrWhiteSpace($ImageTag)) {
    $ImageTag = "vibebus:$expectedVersion-local"
}
if (-not $SkipBuild) {
    Write-Host "[container] building $ImageTag"
    Invoke-DockerChecked -Arguments @(
        "build",
        "--pull",
        "--platform", "linux/amd64",
        "--build-arg", "VIBEBUS_VERSION=$expectedVersion",
        "--build-arg", "VIBEBUS_SOURCE_REVISION=$sourceRevision",
        "--tag", $ImageTag,
        $repoRoot
    ) | Out-Null
}

Write-Host "[container] checking version"
$version = (Invoke-DockerChecked -Arguments @("run", "--rm", $ImageTag, "--version")) -join "`n"
if ($version.Trim() -ne "vibebus $expectedVersion") {
    throw "unexpected container version: $version"
}

$acceptanceRoot = Join-Path ([IO.Path]::GetTempPath()) ("vibebus-container-" + [guid]::NewGuid().ToString("N"))
$projectRoot = Join-Path $acceptanceRoot "project"
$dataRoot = Join-Path $acceptanceRoot "data"
[IO.Directory]::CreateDirectory($projectRoot) | Out-Null
[IO.Directory]::CreateDirectory($dataRoot) | Out-Null
Grant-ContainerTestAccess -ParentPath $acceptanceRoot -WritablePaths @($projectRoot, $dataRoot)
$mountProject = "type=bind,source=$projectRoot,target=/workspace"
$mountData = "type=bind,source=$dataRoot,target=/data"
$containerMayOwnFiles = $false

try {
    $common = @("run", "--rm", "--mount", $mountProject, "--mount", $mountData, $ImageTag)

    Write-Host "[container] initializing disposable project"
    $containerMayOwnFiles = $true
    $initRaw = (Invoke-DockerChecked -Arguments ($common + @(
        "init", "--root", "/workspace", "--name", "Container Acceptance"
    ))) -join "`n"
    $initialized = $initRaw | ConvertFrom-Json
    if (-not $initialized.ok -or $initialized.journalMode -ne "WAL") {
        throw "container project initialization did not report ok/WAL"
    }

    Write-Host "[container] running doctor"
    $doctorRaw = (Invoke-DockerChecked -Arguments ($common + @(
        "doctor", "--root", "/workspace"
    ))) -join "`n"
    $doctor = $doctorRaw | ConvertFrom-Json
    if (-not $doctor.ok -or -not $doctor.result.ok -or $doctor.result.journalMode -ne "wal" -or -not $doctor.result.foreignKeysEnabled) {
        throw "container doctor did not report a healthy WAL database"
    }

    Write-Host "[container] registering disposable Agent"
    $registrationRaw = (Invoke-DockerChecked -Arguments ($common + @(
        "register", "--root", "/workspace", "--name", "container-agent", "--role", "test"
    ))) -join "`n"
    $registration = $registrationRaw | ConvertFrom-Json
    if ([string]::IsNullOrWhiteSpace($registration.result.token) -or [string]::IsNullOrWhiteSpace($registration.result.recoveryKey)) {
        throw "container registration did not return an explicit token/recovery pair"
    }

    Write-Host "[container] checking authenticated Inbox"
    $previousAgentToken = $env:VIBEBUS_AGENT_TOKEN
    $env:VIBEBUS_AGENT_TOKEN = $registration.result.token
    try {
        $inboxRaw = (Invoke-DockerChecked -Arguments (@(
            "run", "--rm",
            "--env", "VIBEBUS_AGENT_TOKEN",
            "--mount", $mountProject,
            "--mount", $mountData,
            $ImageTag,
            "inbox", "--root", "/workspace", "--agent", "container-agent"
        ))) -join "`n"
    }
    finally {
        if ($null -eq $previousAgentToken) {
            Remove-Item Env:VIBEBUS_AGENT_TOKEN -ErrorAction SilentlyContinue
        }
        else {
            $env:VIBEBUS_AGENT_TOKEN = $previousAgentToken
        }
    }
    $inbox = $inboxRaw | ConvertFrom-Json
    if (-not $inbox.ok -or @($inbox.result).Count -ne 0) {
        throw "new container Agent inbox was expected to be empty"
    }

    Write-Host "[container] negotiating stdio MCP"
    $mcpInput = @(
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"container-acceptance","version":"0.1.0"}}}',
        '{"jsonrpc":"2.0","method":"notifications/initialized"}',
        '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
    ) -join "`n"
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $mcpOutput = $mcpInput | & docker run --rm -i --mount $mountProject --mount $mountData $ImageTag mcp --root /workspace 2>&1
        $mcpExitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($mcpExitCode -ne 0) {
        throw "container stdio MCP negotiation failed"
    }
    $responses = @($mcpOutput | ForEach-Object { $_ | ConvertFrom-Json })
    $initializedResponse = $responses | Where-Object { $_.id -eq 1 } | Select-Object -First 1
    $toolsResponse = $responses | Where-Object { $_.id -eq 2 } | Select-Object -First 1
    if ($initializedResponse.result.serverInfo.name -ne "vibebus") {
        throw "container MCP initialize returned an unexpected server"
    }
    $toolCount = @($toolsResponse.result.tools).Count
    if ($toolCount -lt 40) {
        throw "container MCP tools/list returned only $toolCount tools"
    }

    Write-Host "[container] inspecting accepted image"
    $imageInspectRaw = (& docker image inspect $ImageTag --format '{{json .}}') -join "`n"
    if ($LASTEXITCODE -ne 0) {
        throw "unable to read the built image ID"
    }
    $imageInspect = $imageInspectRaw | ConvertFrom-Json
    $imageId = [string]$imageInspect.Id
    if (-not $imageId.StartsWith("sha256:")) {
        throw "unable to read the built image ID"
    }
    if ($imageInspect.Os -ne "linux" -or $imageInspect.Architecture -ne "amd64") {
        throw "accepted image must target linux/amd64"
    }
    if ($imageInspect.Config.User -ne "10001:10001") {
        throw "accepted image must run as non-root user 10001:10001"
    }
    if ($imageInspect.Config.Labels."org.opencontainers.image.revision" -ne $sourceRevision) {
        throw "accepted image source revision does not match the checked-out source"
    }

    [pscustomobject]@{
        ok = $true
        image = $ImageTag
        imageId = $imageId
        platform = "linux/amd64"
        imageSizeBytes = [long]$imageInspect.Size
        runtimeUser = [string]$imageInspect.Config.User
        sourceRevision = $sourceRevision
        version = $version.Trim()
        journalMode = $doctor.result.journalMode
        foreignKeysEnabled = [bool]$doctor.result.foreignKeysEnabled
        mcpToolCount = $toolCount
        credentialMode = "explicit token or VIBEBUS_AGENT_TOKEN"
        secretsPrinted = $false
    } | ConvertTo-Json
}
finally {
    Remove-Variable registration, registrationRaw, inboxRaw, mcpInput -ErrorAction SilentlyContinue
    if ($containerMayOwnFiles -and [Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        # Files created by UID 10001 can contain owner-only directories. Remove
        # their contents inside the same mount namespace before host cleanup.
        Invoke-DockerChecked -Arguments @(
            "run", "--rm",
            "--user", "0",
            "--entrypoint", "/bin/sh",
            "--mount", $mountProject,
            "--mount", $mountData,
            $ImageTag,
            "-c", "rm -rf /workspace/* /workspace/.[!.]* /workspace/..?* /data/* /data/.[!.]* /data/..?*"
        ) | Out-Null
    }
    if ([IO.Directory]::Exists($acceptanceRoot)) {
        [IO.Directory]::Delete($acceptanceRoot, $true)
    }
}
