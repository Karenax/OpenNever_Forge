[CmdletBinding()]
param(
    [string]$OutputPath = 'target/release/release-manifest.json',
    [string]$SbomDirectory = 'target/release/sbom',
    [string]$ChecksumsPath = 'target/release/SHA256SUMS',
    [string]$ExpectedVersion,
    [switch]$RequireClean,
    [switch]$RequireSigned
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$resolvedOutput = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $OutputPath))
$resolvedSbomDirectory = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $SbomDirectory))
$resolvedChecksums = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $ChecksumsPath))
$repositoryPrefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
foreach ($path in @($resolvedOutput, $resolvedSbomDirectory, $resolvedChecksums)) {
    if (-not $path.StartsWith($repositoryPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Sortie de distribution hors dépôt refusée : $path"
    }
}

function Get-RelativeRepositoryPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if (-not $fullPath.StartsWith($repositoryPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Fichier hors dépôt refusé : $fullPath"
    }
    $fullPath.Substring($repositoryPrefix.Length).Replace('\', '/')
}

function Get-VersionOutput {
    param([Parameter(Mandatory = $true)][scriptblock]$Command)
    $output = @(& $Command 2>$null)
    if ($LASTEXITCODE -ne 0 -or $output.Count -eq 0) { return $null }
    ($output | Select-Object -First 1).ToString().Trim()
}

$configuration = Get-Content -LiteralPath (Join-Path $repositoryRoot 'apps/desktop/src-tauri/tauri.conf.json') -Raw -Encoding utf8 | ConvertFrom-Json
$package = Get-Content -LiteralPath (Join-Path $repositoryRoot 'package.json') -Raw -Encoding utf8 | ConvertFrom-Json
$cargoMetadataRaw = (& cargo metadata --manifest-path (Join-Path $repositoryRoot 'Cargo.toml') --no-deps --format-version 1 | Out-String)
if ($LASTEXITCODE -ne 0) { throw 'Impossible de lire cargo metadata.' }
$cargoMetadata = $cargoMetadataRaw | ConvertFrom-Json
$cargoVersions = @($cargoMetadata.packages.version | Sort-Object -Unique)
if ($cargoVersions.Count -ne 1) {
    throw "Versions Cargo incohérentes : $($cargoVersions -join ', ')"
}
$version = [string]$configuration.version
if ($package.version -ne $version -or $cargoVersions[0] -ne $version) {
    throw "Versions incohérentes : Tauri=$version, npm=$($package.version), Cargo=$($cargoVersions[0])"
}
if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion) -and $version -ne $ExpectedVersion) {
    throw "Version inattendue : $version, attendu $ExpectedVersion"
}

$artifactPaths = @(
    (Join-Path $repositoryRoot 'target/release/opennever-forge-desktop.exe'),
    (Join-Path $repositoryRoot 'target/release/opennever-mcp.exe')
)
$installer = Get-ChildItem -LiteralPath (Join-Path $repositoryRoot 'target/release/bundle/nsis') -Filter '*.exe' -File |
    Sort-Object Name | Select-Object -First 1
if ($null -eq $installer) { throw 'Installateur NSIS absent sous target/release/bundle/nsis.' }
$artifactPaths += $installer.FullName

$artifacts = foreach ($path in $artifactPaths) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Artefact de release absent : $path" }
    $item = Get-Item -LiteralPath $path
    $signature = Get-AuthenticodeSignature -LiteralPath $item.FullName
    [ordered]@{
        path = Get-RelativeRepositoryPath -Path $item.FullName
        sizeBytes = $item.Length
        sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash
        signature = [ordered]@{
            status = $signature.Status.ToString()
            subject = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
            thumbprint = if ($signature.SignerCertificate) { $signature.SignerCertificate.Thumbprint } else { $null }
            timeStamper = if ($signature.TimeStamperCertificate) { $signature.TimeStamperCertificate.Subject } else { $null }
        }
    }
}
$signed = @($artifacts | Where-Object { $_.signature.status -ne 'Valid' }).Count -eq 0
if ($RequireSigned -and -not $signed) { throw 'Tous les artefacts doivent porter une signature Authenticode valide.' }

$sbomFiles = @(Get-ChildItem -LiteralPath $resolvedSbomDirectory -Filter '*.cdx.json' -File -ErrorAction SilentlyContinue | Sort-Object Name)
if ($sbomFiles.Count -lt 2) { throw "SBOM incomplètes sous $resolvedSbomDirectory" }
$sboms = foreach ($file in $sbomFiles) {
    $document = Get-Content -LiteralPath $file.FullName -Raw -Encoding utf8 | ConvertFrom-Json
    if ($document.bomFormat -ne 'CycloneDX') { throw "SBOM CycloneDX invalide : $($file.FullName)" }
    [ordered]@{
        path = Get-RelativeRepositoryPath -Path $file.FullName
        sizeBytes = $file.Length
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
        specVersion = [string]$document.specVersion
        component = [string]$document.metadata.component.name
    }
}

$commit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Impossible de lire le commit Git.' }
$branch = (& git -C $repositoryRoot rev-parse --abbrev-ref HEAD).Trim()
$tag = @(& git -C $repositoryRoot tag --points-at HEAD | Select-Object -First 1)
$tagValue = if ($tag.Count -eq 0) { $null } else { $tag[0].Trim() }
$dirty = [bool]((& git -C $repositoryRoot status --porcelain).Count)
if ($RequireClean -and $dirty) { throw 'Un arbre Git propre est requis pour le manifeste final.' }
$sourceDateEpoch = (& git -C $repositoryRoot show -s --format=%ct HEAD).Trim()

$checksumInputs = @($artifacts) + @($sboms)
$checksumLines = @($checksumInputs | Sort-Object path | ForEach-Object { "$($_.sha256)  $($_.path)" })
$checksumParent = Split-Path -Parent $resolvedChecksums
New-Item -ItemType Directory -Force -Path $checksumParent | Out-Null
$checksumLines | Set-Content -LiteralPath $resolvedChecksums -Encoding ascii

$manifest = [ordered]@{
    schemaVersion = 2
    product = $configuration.productName
    version = $version
    createdUtc = [DateTime]::UtcNow.ToString('o')
    sourceDateEpoch = [long]$sourceDateEpoch
    commit = $commit
    branch = $branch
    tag = $tagValue
    dirty = $dirty
    signed = $signed
    platform = [ordered]@{
        os = [Environment]::OSVersion.VersionString
        architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    }
    toolchains = [ordered]@{
        node = Get-VersionOutput { node --version }
        pnpm = Get-VersionOutput { pnpm --version }
        rustc = Get-VersionOutput { rustc --version }
        cargo = Get-VersionOutput { cargo --version }
        python = Get-VersionOutput { python --version }
        tauri = Get-VersionOutput { pnpm --filter '@opennever/desktop' exec tauri --version }
    }
    artifacts = @($artifacts)
    sboms = @($sboms)
    checksums = [ordered]@{
        path = Get-RelativeRepositoryPath -Path $resolvedChecksums
        sizeBytes = (Get-Item -LiteralPath $resolvedChecksums).Length
        sha256 = (Get-FileHash -LiteralPath $resolvedChecksums -Algorithm SHA256).Hash
        entries = $checksumLines.Count
    }
}
$parent = Split-Path -Parent $resolvedOutput
New-Item -ItemType Directory -Force -Path $parent | Out-Null
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $resolvedOutput -Encoding utf8
$manifest
