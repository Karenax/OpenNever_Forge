[CmdletBinding()]
param(
    [string]$ManifestPath = 'target/release/release-manifest.json',
    [string]$ArtifactRoot,
    [string]$ExpectedVersion,
    [switch]$RequireClean,
    [switch]$RequireSigned
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$resolvedManifest = if ([string]::IsNullOrWhiteSpace($ArtifactRoot)) {
    [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $ManifestPath))
}
else {
    $root = [System.IO.Path]::GetFullPath($ArtifactRoot)
    $matches = @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter 'release-manifest.json')
    if ($matches.Count -ne 1) { throw "Un manifeste attendu sous $root, trouvé : $($matches.Count)" }
    $matches[0].FullName
}
if (-not (Test-Path -LiteralPath $resolvedManifest -PathType Leaf)) {
    throw "Manifeste de distribution absent : $resolvedManifest"
}
$manifest = Get-Content -LiteralPath $resolvedManifest -Raw -Encoding utf8 | ConvertFrom-Json
if ($manifest.schemaVersion -ne 2) {
    throw "Schéma de manifeste non supporté : $($manifest.schemaVersion)"
}
if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion) -and $manifest.version -ne $ExpectedVersion) {
    throw "Version inattendue : $($manifest.version), attendu $ExpectedVersion"
}
if ($RequireClean -and $manifest.dirty) {
    throw "Le manifeste provient d’un arbre Git sale."
}
if ($RequireSigned -and -not $manifest.signed) {
    throw 'Le manifeste ne décrit pas une distribution signée.'
}
function Resolve-DistributionFile {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    if ($RelativePath.Contains('..') -or [System.IO.Path]::IsPathRooted($RelativePath)) {
        throw "Chemin de distribution non sûr : $RelativePath"
    }
    if ([string]::IsNullOrWhiteSpace($ArtifactRoot)) {
        $candidate = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $RelativePath))
        $prefix = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
        if (-not $candidate.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Chemin de distribution hors dépôt : $RelativePath"
        }
        return $candidate
    }
    $leaf = Split-Path -Leaf $RelativePath
    $matches = @(Get-ChildItem -LiteralPath ([System.IO.Path]::GetFullPath($ArtifactRoot)) -Recurse -File |
        Where-Object { $_.Name -eq $leaf })
    if ($matches.Count -ne 1) {
        throw "Un fichier $leaf attendu dans la distribution, trouvé : $($matches.Count)"
    }
    return $matches[0].FullName
}

$checksumPath = Resolve-DistributionFile -RelativePath ([string]$manifest.checksums.path)
$checksumHash = (Get-FileHash -LiteralPath $checksumPath -Algorithm SHA256).Hash
if ($checksumHash -ne $manifest.checksums.sha256) {
    throw 'Le fichier SHA256SUMS ne correspond pas au manifeste.'
}
$checksumEntries = @{}
foreach ($line in Get-Content -LiteralPath $checksumPath -Encoding ascii) {
    if ($line -notmatch '^([A-Fa-f0-9]{64})  (.+)$') {
        throw "Ligne SHA256SUMS invalide : $line"
    }
    $checksumEntries[$Matches[2].Replace('\', '/')] = $Matches[1].ToUpperInvariant()
}

$records = @($manifest.artifacts) + @($manifest.sboms)
$results = foreach ($record in $records) {
    $path = Resolve-DistributionFile -RelativePath ([string]$record.path)
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Fichier de distribution absent : $path"
    }
    $item = Get-Item -LiteralPath $path
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    if ($item.Length -ne $record.sizeBytes -or $hash -ne $record.sha256) {
        throw "Taille ou SHA-256 divergent pour $($record.path)"
    }
    $checksumKey = ([string]$record.path).Replace('\', '/')
    if (-not $checksumEntries.ContainsKey($checksumKey) -or $checksumEntries[$checksumKey] -ne $hash) {
        throw "Entrée SHA256SUMS absente ou divergente : $checksumKey"
    }
    if ($record.PSObject.Properties.Name -contains 'signature') {
        $signature = Get-AuthenticodeSignature -LiteralPath $path
        if ($manifest.signed -or $RequireSigned) {
            if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
                $signature.SignerCertificate.Thumbprint -ne $record.signature.thumbprint) {
                throw "Signature invalide ou inattendue : $($record.path)"
            }
        }
    }
    else {
        $raw = Get-Content -LiteralPath $path -Raw -Encoding utf8
        $sbom = $raw | ConvertFrom-Json
        if ($sbom.bomFormat -ne 'CycloneDX') {
            throw "SBOM CycloneDX invalide : $($record.path)"
        }
        foreach ($variant in @($repositoryRoot, $repositoryRoot.Replace('\', '/'), $repositoryRoot.Replace('\', '\\'))) {
            if ($raw.IndexOf($variant, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
                throw "Chemin local détecté dans la SBOM : $($record.path)"
            }
        }
    }
    [pscustomobject]@{ Path = $path; SizeBytes = $item.Length; Sha256 = $hash }
}

if ([string]::IsNullOrWhiteSpace($ArtifactRoot)) {
    $head = (& git -C $repositoryRoot rev-parse HEAD).Trim()
    if ($head -ne $manifest.commit) { throw 'Le manifeste ne correspond pas au commit courant.' }
    if ($RequireClean -and [bool]((& git -C $repositoryRoot status --porcelain).Count)) {
        throw "L’arbre Git courant n’est pas propre."
    }
}

[pscustomobject]@{
    Status = 'DISTRIBUTION_VERIFICATION_PASS'
    Version = $manifest.version
    Commit = $manifest.commit
    Dirty = [bool]$manifest.dirty
    Signed = [bool]$manifest.signed
    Files = @($results)
}
