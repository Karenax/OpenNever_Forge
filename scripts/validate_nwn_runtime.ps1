[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$NwnRoot,

    [string]$UserDirectory = (Join-Path $PSScriptRoot '..\.tmp\nwn-runtime-validation'),

    [string]$SourceModulePath,

    [string]$DependencyUserDirectory,

    [string]$WokResRef = 'tin01_o20_01',

    [string]$PwkResRef = 'plc_t06',

    [string]$DwkResRef = 't_door01',

    [ValidateRange(1024, 65534)]
    [int]$Port = 5139,

    [ValidateRange(2, 60)]
    [int]$TimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$validationRoot = [System.IO.Path]::GetFullPath($UserDirectory)
$expectedPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if (-not $validationRoot.StartsWith($expectedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Le dossier de validation doit rester sous le dépôt : $repositoryRoot"
}

$serverPath = Join-Path $NwnRoot 'bin\win32\nwserver.exe'
if (-not (Test-Path -LiteralPath $serverPath -PathType Leaf)) {
    throw "nwserver.exe introuvable : $serverPath"
}
$serverWorkingDirectory = Split-Path -Parent $serverPath
$serverFile = Get-Item -LiteralPath $serverPath
$serverVersion = $serverFile.VersionInfo.FileVersion
$serverSha256 = (Get-FileHash -LiteralPath $serverPath -Algorithm SHA256).Hash

$modulesDirectory = Join-Path $validationRoot 'modules'
$developmentDirectory = Join-Path $validationRoot 'development'
$generatedModulePath = Join-Path $modulesDirectory 'onfvalid.mod'
New-Item -ItemType Directory -Force -Path $modulesDirectory | Out-Null
New-Item -ItemType Directory -Force -Path $developmentDirectory | Out-Null
foreach ($name in @("$WokResRef.wok", "$PwkResRef.pwk", "$DwkResRef.dwk")) {
    $candidate = Join-Path $developmentDirectory $name
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        Remove-Item -LiteralPath $candidate -Force
    }
}
if (Test-Path -LiteralPath $generatedModulePath -PathType Leaf) {
    Remove-Item -LiteralPath $generatedModulePath -Force
}

$sourceHashBefore = $null
$resolvedModule = $null
Push-Location $repositoryRoot
try {
    if ([string]::IsNullOrWhiteSpace($SourceModulePath)) {
        & cargo run --quiet -p aurora-edit --example build_validation_module -- $generatedModulePath
        if ($LASTEXITCODE -ne 0) {
            throw "La construction du module de validation a échoué ($LASTEXITCODE)."
        }
    }
    else {
        $resolvedModule = (Resolve-Path -LiteralPath $SourceModulePath).Path
        $sourceHashBefore = (Get-FileHash -LiteralPath $resolvedModule -Algorithm SHA256).Hash
        Copy-Item -LiteralPath $resolvedModule -Destination $generatedModulePath -Force
    }
}
finally {
    Pop-Location
}

function Invoke-NwnServerProbe {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Label,
        [Parameter(Mandatory = $true)]
        [int]$ProbePort
    )
    $arguments = @(
        '-userdirectory', $validationRoot,
        '-module', 'onfvalid',
        '-publicserver', '0',
        '-port', $ProbePort.ToString([System.Globalization.CultureInfo]::InvariantCulture),
        '-servername', "OpenNeverValidation-$Label"
    )
    $startedAt = Get-Date
    $server = Start-Process -FilePath $serverPath -WorkingDirectory $serverWorkingDirectory `
        -ArgumentList $arguments -WindowStyle Hidden -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    try {
        while ([DateTime]::UtcNow -lt $deadline) {
            Start-Sleep -Milliseconds 250
            $server.Refresh()
            if ($server.HasExited) {
                $unsignedExitCode = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int32]$server.ExitCode), 0)
                $crashEvent = try {
                    Get-WinEvent -FilterHashtable @{ LogName = 'Application'; StartTime = $startedAt.AddSeconds(-2) } -MaxEvents 50 -ErrorAction Stop |
                        Where-Object { $_.Message -match 'nwserver\.exe' } |
                        Select-Object -First 1 |
                        ForEach-Object {
                            [pscustomobject]@{
                                Provider = $_.ProviderName
                                EventId = $_.Id
                                TimeCreated = $_.TimeCreated
                                Message = $_.Message
                            }
                        }
                }
                catch {
                    $null
                }
                return [pscustomobject]@{
                    Label = $Label
                    Passed = $false
                    Detail = "NWServer arrêté avant écoute (code $($server.ExitCode))."
                    ExitCode = $server.ExitCode
                    ExitCodeHex = ('0x{0:X8}' -f $unsignedExitCode)
                    CrashEvent = $crashEvent
                }
            }
            $endpoint = Get-NetUDPEndpoint -LocalPort $ProbePort -ErrorAction SilentlyContinue |
                Where-Object { $_.OwningProcess -eq $server.Id } |
                Select-Object -First 1
            if ($null -ne $endpoint) {
                return [pscustomobject]@{
                    Label = $Label
                    Passed = $true
                    Detail = "Écoute UDP active sur $ProbePort."
                }
            }
        }
        return [pscustomobject]@{
            Label = $Label
            Passed = $false
            Detail = "Aucune écoute UDP après $TimeoutSeconds secondes."
        }
    }
    finally {
        $server.Refresh()
        if (-not $server.HasExited) {
            Stop-Process -Id $server.Id
            $server.WaitForExit(5000) | Out-Null
        }
    }
}

$baseline = Invoke-NwnServerProbe -Label 'baseline' -ProbePort $Port
$overlayManifest = @()
$overlay = [pscustomobject]@{
    Label = 'walkmesh-overlay'
    Passed = $false
    Detail = "Contrôle non exécuté : le témoin doit atteindre l’écoute en premier."
}
if ($baseline.Passed) {
    if ($null -ne $resolvedModule) {
        $dependencyRoot = if ([string]::IsNullOrWhiteSpace($DependencyUserDirectory)) {
            $validationRoot
        }
        else {
            (Resolve-Path -LiteralPath $DependencyUserDirectory).Path
        }
        Push-Location $repositoryRoot
        try {
            $manifestJson = & cargo run --quiet -p aurora-project --example build_walkmesh_runtime_overlay -- `
                $resolvedModule $NwnRoot $dependencyRoot $developmentDirectory $WokResRef $PwkResRef $DwkResRef
            if ($LASTEXITCODE -ne 0) {
                throw "La construction de l'overlay WOK/PWK/DWK a échoué ($LASTEXITCODE)."
            }
            $overlayManifest = $manifestJson | ConvertFrom-Json
        }
        finally {
            Pop-Location
        }
    }
    $overlay = Invoke-NwnServerProbe -Label 'walkmesh-overlay' -ProbePort ($Port + 1)
}

$sourceIntact = $true
$sourceHashAfter = $sourceHashBefore
if ($null -ne $resolvedModule) {
    $sourceHashAfter = (Get-FileHash -LiteralPath $resolvedModule -Algorithm SHA256).Hash
    $sourceIntact = $sourceHashBefore -eq $sourceHashAfter
    if (-not $sourceIntact) {
        throw "Le module source a changé pendant la validation."
    }
}

$status = if ($baseline.Passed -and $overlay.Passed) {
    'PASS'
}
elseif (-not $baseline.Passed) {
    'INCONCLUSIVE_ENVIRONMENT'
}
else {
    'FAIL_OVERLAY'
}

[pscustomobject]@{
    Status = $status
    Module = $generatedModulePath
    SourceModule = $resolvedModule
    SourceSha256Before = $sourceHashBefore
    SourceSha256After = $sourceHashAfter
    NwnServer = $serverPath
    NwnServerVersion = $serverVersion
    NwnServerSha256 = $serverSha256
    NwnServerWorkingDirectory = $serverWorkingDirectory
    ValidationUserDirectory = $validationRoot
    Ports = [pscustomobject]@{
        Baseline = $Port
        Overlay = $Port + 1
    }
    Baseline = $baseline
    Overlay = $overlay
    Walkmeshes = $overlayManifest
    SourceIntact = $sourceIntact
}
