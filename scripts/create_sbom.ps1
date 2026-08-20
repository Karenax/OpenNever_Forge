[CmdletBinding()]
param(
    [string]$OutputDirectory = 'target/release/sbom'
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$releaseRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot 'target/release'))
$resolvedOutput = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $OutputDirectory))
$releasePrefix = $releaseRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if (-not $resolvedOutput.StartsWith($releasePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Le dossier SBOM doit rester sous target/release : $resolvedOutput"
}
New-Item -ItemType Directory -Force -Path $resolvedOutput | Out-Null

Get-ChildItem -LiteralPath $resolvedOutput -Filter '*.cdx.json' -File -ErrorAction SilentlyContinue |
    ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force }

function Invoke-External {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )
    $output = @(& $Executable @Arguments)
    if ($LASTEXITCODE -ne 0) {
        throw "$Name a échoué avec le code $LASTEXITCODE."
    }
    if ($output.Count -ne 0) {
        Write-Verbose ($output -join [Environment]::NewLine)
    }
}

$frontendPath = Join-Path $resolvedOutput 'opennever-forge-frontend.cdx.json'
Invoke-External -Name 'SBOM frontend pnpm' -Executable 'pnpm' -Arguments @(
    '--filter', '@opennever/desktop', 'sbom',
    '--sbom-format', 'cyclonedx',
    '--sbom-spec-version', '1.7',
    '--prod',
    '--lockfile-only',
    '--sbom-type', 'application',
    '--sbom-supplier', 'OpenNever Forge',
    '--out', $frontendPath
)

$cargoCycloneDx = Get-Command cargo-cyclonedx -ErrorAction SilentlyContinue
if ($null -eq $cargoCycloneDx) {
    $localCargoCycloneDx = Join-Path $repositoryRoot '.tmp/cargo-tools/bin/cargo-cyclonedx.exe'
    if (Test-Path -LiteralPath $localCargoCycloneDx -PathType Leaf) {
        $cargoCycloneDxPath = $localCargoCycloneDx
    }
    else {
        throw 'cargo-cyclonedx 0.5.9 est requis. Installez-le avec cargo install --locked cargo-cyclonedx --version 0.5.9.'
    }
}
else {
    $cargoCycloneDxPath = $cargoCycloneDx.Source
}

$generatedBaseName = 'opennever-forge-rust-workspace-sbom'
$generatedFileName = "$generatedBaseName.json"
$sourceRoots = @(
    (Join-Path $repositoryRoot 'apps'),
    (Join-Path $repositoryRoot 'crates')
)
$preexisting = @($sourceRoots | ForEach-Object {
    Get-ChildItem -LiteralPath $_ -Recurse -File -Filter $generatedFileName -ErrorAction SilentlyContinue
})
if ($preexisting.Count -ne 0) {
    throw "Une SBOM temporaire préexistante empêche une collecte sûre : $($preexisting[0].FullName)"
}

$previousSourceDateEpoch = $env:SOURCE_DATE_EPOCH
$sourceDateEpoch = (& git -C $repositoryRoot show -s --format=%ct HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceDateEpoch -notmatch '^\d+$') {
    throw 'Impossible de déterminer SOURCE_DATE_EPOCH depuis Git.'
}
$env:SOURCE_DATE_EPOCH = $sourceDateEpoch
$generated = @()
try {
    Invoke-External -Name 'SBOM workspace Rust' -Executable $cargoCycloneDxPath -Arguments @(
        'cyclonedx',
        '--manifest-path', (Join-Path $repositoryRoot 'Cargo.toml'),
        '--format', 'json',
        '--spec-version', '1.5',
        '--override-filename', $generatedBaseName
    )
    $generated = @($sourceRoots | ForEach-Object {
        Get-ChildItem -LiteralPath $_ -Recurse -File -Filter $generatedFileName -ErrorAction SilentlyContinue
    })
    if ($generated.Count -eq 0) {
        throw "cargo-cyclonedx n’a produit aucune SBOM Rust."
    }
    foreach ($item in $generated) {
        $document = Get-Content -LiteralPath $item.FullName -Raw -Encoding utf8 | ConvertFrom-Json
        $componentName = [string]$document.metadata.component.name
        if ([string]::IsNullOrWhiteSpace($componentName)) {
            throw "Composant principal absent de la SBOM Rust : $($item.FullName)"
        }
        $safeName = ($componentName.ToLowerInvariant() -replace '[^a-z0-9._-]', '-')
        $destination = Join-Path $resolvedOutput "rust-$safeName.cdx.json"
        if (Test-Path -LiteralPath $destination) {
            throw "Nom de composant Rust dupliqué dans les SBOM : $componentName"
        }
        Copy-Item -LiteralPath $item.FullName -Destination $destination
        $raw = Get-Content -LiteralPath $destination -Raw -Encoding utf8
        $forwardRoot = $repositoryRoot.Replace('\', '/')
        $sanitized = $raw.Replace("file:///$forwardRoot", 'file:///workspace')
        $sanitized = $sanitized.Replace($forwardRoot, '/workspace')
        $sanitized = $sanitized.Replace($repositoryRoot.Replace('\', '\\'), '\\workspace')
        $sanitized | Set-Content -LiteralPath $destination -Encoding utf8
    }
}
finally {
    foreach ($item in $generated) {
        if (Test-Path -LiteralPath $item.FullName -PathType Leaf) {
            Remove-Item -LiteralPath $item.FullName -Force
        }
    }
    if ($null -eq $previousSourceDateEpoch) {
        Remove-Item Env:SOURCE_DATE_EPOCH -ErrorAction SilentlyContinue
    }
    else {
        $env:SOURCE_DATE_EPOCH = $previousSourceDateEpoch
    }
}

$repositoryVariants = @(
    $repositoryRoot,
    $repositoryRoot.Replace('\', '/'),
    $repositoryRoot.Replace('\', '\\')
)
$results = foreach ($file in Get-ChildItem -LiteralPath $resolvedOutput -Filter '*.cdx.json' -File | Sort-Object Name) {
    $raw = Get-Content -LiteralPath $file.FullName -Raw -Encoding utf8
    $document = $raw | ConvertFrom-Json
    if ($document.bomFormat -ne 'CycloneDX' -or [string]::IsNullOrWhiteSpace([string]$document.specVersion)) {
        throw "SBOM CycloneDX invalide : $($file.FullName)"
    }
    foreach ($variant in $repositoryVariants) {
        if ($raw.IndexOf($variant, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
            throw "Chemin local absolu détecté dans la SBOM : $($file.FullName)"
        }
    }
    [pscustomobject]@{
        Path = $file.FullName
        SpecVersion = [string]$document.specVersion
        Component = [string]$document.metadata.component.name
        Components = @($document.components).Count
        Sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    }
}

[pscustomobject]@{
    OutputDirectory = $resolvedOutput
    SourceDateEpoch = $sourceDateEpoch
    Documents = @($results)
}
