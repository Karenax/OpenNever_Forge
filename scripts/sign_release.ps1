[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Fa-f0-9]{40}$')]
    [string]$CertificateThumbprint,

    [Parameter(Mandatory = $true)]
    [string]$TimestampUrl,

    [ValidateSet('Binaries', 'Installer', 'All')]
    [string]$Phase = 'All',

    [string]$SignToolPath
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$thumbprint = $CertificateThumbprint.ToUpperInvariant()
$timestamp = $null
if (-not [Uri]::TryCreate($TimestampUrl, [UriKind]::Absolute, [ref]$timestamp) -or
    $timestamp.Scheme -notin @('http', 'https')) {
    throw 'TimestampUrl doit être une URL HTTP(S) absolue.'
}
$certificate = $null
$machineStore = $false
foreach ($candidate in @(
    "Cert:\CurrentUser\My\$thumbprint",
    "Cert:\LocalMachine\My\$thumbprint"
)) {
    if (Test-Path -LiteralPath $candidate) {
        $certificate = Get-Item -LiteralPath $candidate
        $machineStore = $candidate.StartsWith('Cert:\LocalMachine', [StringComparison]::OrdinalIgnoreCase)
        break
    }
}
if ($null -eq $certificate -or -not $certificate.HasPrivateKey) {
    throw "Certificat de signature avec clé privée introuvable : $thumbprint"
}
if ($certificate.NotBefore -gt (Get-Date) -or $certificate.NotAfter -le (Get-Date)) {
    throw "Certificat hors période de validité : $thumbprint"
}
if ($certificate.EnhancedKeyUsageList.ObjectId -notcontains '1.3.6.1.5.5.7.3.3') {
    throw "Le certificat ne permet pas la signature de code : $thumbprint"
}

if ([string]::IsNullOrWhiteSpace($SignToolPath)) {
    $signTool = Get-ChildItem -LiteralPath 'C:\Program Files (x86)\Windows Kits\10\bin' `
        -Filter signtool.exe -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Directory.Name -eq 'x64' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if ($null -eq $signTool) {
        throw 'signtool.exe x64 est introuvable dans Windows Kits.'
    }
    $SignToolPath = $signTool.FullName
}
elseif (-not (Test-Path -LiteralPath $SignToolPath -PathType Leaf)) {
    throw "signtool.exe introuvable : $SignToolPath"
}

$binaryPaths = @(
    (Join-Path $repositoryRoot 'target/release/opennever-forge-desktop.exe'),
    (Join-Path $repositoryRoot 'target/release/opennever-mcp.exe')
)
$installer = Get-ChildItem -LiteralPath (Join-Path $repositoryRoot 'target/release/bundle/nsis') `
    -Filter '*.exe' -File -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -First 1
$paths = switch ($Phase) {
    'Binaries' { $binaryPaths }
    'Installer' {
        if ($null -eq $installer) { throw 'Installateur NSIS absent pour la phase Installer.' }
        @($installer.FullName)
    }
    'All' {
        if ($null -eq $installer) { throw 'Installateur NSIS absent pour la phase All.' }
        @($binaryPaths + $installer.FullName)
    }
}

$results = foreach ($path in $paths) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Artefact à signer absent : $path"
    }
    $arguments = @('sign')
    if ($machineStore) { $arguments += '/sm' }
    $arguments += @(
        '/sha1', $thumbprint,
        '/fd', 'SHA256',
        '/tr', $timestamp.AbsoluteUri,
        '/td', 'SHA256',
        '/d', 'OpenNever Forge',
        '/v',
        $path
    )
    & $SignToolPath @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Signature Authenticode échouée pour $path ($LASTEXITCODE)."
    }
    & $SignToolPath verify /pa /all /v $path
    if ($LASTEXITCODE -ne 0) {
        throw "Vérification SignTool échouée pour $path ($LASTEXITCODE)."
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
        $signature.SignerCertificate.Thumbprint -ne $thumbprint) {
        throw "Signature Authenticode inattendue pour $path : $($signature.Status)"
    }
    [pscustomobject]@{
        Path = $path
        Status = $signature.Status.ToString()
        Signer = $signature.SignerCertificate.Subject
        Thumbprint = $signature.SignerCertificate.Thumbprint
        TimeStamper = $signature.TimeStamperCertificate.Subject
        Sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    }
}

[pscustomobject]@{
    Phase = $Phase
    SignTool = $SignToolPath
    Certificate = $certificate.Subject
    Artifacts = @($results)
}
