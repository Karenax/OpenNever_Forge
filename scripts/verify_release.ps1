[CmdletBinding()]
param(
    [string]$ManifestPath = 'target/release/release-manifest.json',
    [string]$ExpectedVersion,
    [switch]$RequireClean,
    [switch]$RequireSigned,
    [string]$SigningCertificateThumbprint,
    [string]$TimestampUrl
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))

function Invoke-Gate {
    param([string]$Name, [scriptblock]$Command)
    Write-Host "`n== $Name ==" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Name a échoué avec le code $LASTEXITCODE." }
}
if ($RequireClean -and [bool]((& git -C $repositoryRoot status --porcelain).Count)) {
    throw 'Le mode final exige un arbre Git propre avant de lancer les builds.'
}
if ($RequireSigned -and ([string]::IsNullOrWhiteSpace($SigningCertificateThumbprint) -or
    [string]::IsNullOrWhiteSpace($TimestampUrl))) {
    throw 'Le mode signé exige SigningCertificateThumbprint et TimestampUrl.'
}

Push-Location $repositoryRoot
try {
    Invoke-Gate 'Installation déterministe' { pnpm install --frozen-lockfile }
    Invoke-Gate 'Audit frontend de production' { pnpm audit --prod --audit-level=low }
    Invoke-Gate 'TypeScript' { pnpm lint }
    Invoke-Gate 'Tests frontend' { pnpm test:run }
    Invoke-Gate 'Build frontend' { pnpm build }
    Invoke-Gate 'Budgets bundle et sources' { pnpm check:bundle }
    Invoke-Gate 'Format Rust' { cargo fmt --all -- --check }
    Invoke-Gate 'Clippy strict' { cargo clippy --workspace --all-targets -- -D warnings }
    Invoke-Gate 'Tests Rust' { cargo test --workspace }
    Invoke-Gate 'Tests Python et fixtures' { python -m unittest discover -s tests -v }
    Invoke-Gate 'Graphe d architecture' { python scripts/architecture_graph.py check }

    $cargoDeny = Get-Command cargo-deny -ErrorAction SilentlyContinue
    if ($null -eq $cargoDeny) {
        $localCargoDeny = Join-Path $repositoryRoot '.tmp/cargo-tools/bin/cargo-deny.exe'
        if (Test-Path -LiteralPath $localCargoDeny -PathType Leaf) { $cargoDenyPath = $localCargoDeny }
        else { throw 'cargo-deny est requis. Installez-le avec cargo install --locked cargo-deny.' }
    }
    else { $cargoDenyPath = $cargoDeny.Source }
    Invoke-Gate 'Advisories, licences et sources Rust' { & $cargoDenyPath --log-level error check }

    if ($RequireSigned) {
        Invoke-Gate 'Binaire desktop sans bundle' { pnpm tauri build --no-bundle --ci --no-sign }
        Invoke-Gate 'Compagnon MCP' { cargo build --release -p opennever-mcp }
        Invoke-Gate 'Signature des binaires' {
            & (Join-Path $PSScriptRoot 'sign_release.ps1') -Phase Binaries `
                -CertificateThumbprint $SigningCertificateThumbprint -TimestampUrl $TimestampUrl | Format-List
        }
        Invoke-Gate 'Construction NSIS depuis le binaire signé' { pnpm tauri bundle --bundles nsis --ci --no-sign }
        Invoke-Gate 'Signature de l installateur' {
            & (Join-Path $PSScriptRoot 'sign_release.ps1') -Phase Installer `
                -CertificateThumbprint $SigningCertificateThumbprint -TimestampUrl $TimestampUrl | Format-List
        }
    }
    else {
        Invoke-Gate 'Candidate Windows NSIS' { pnpm tauri build --ci --no-sign }
        Invoke-Gate 'Compagnon MCP' { cargo build --release -p opennever-mcp }
    }

    Invoke-Gate 'SBOM CycloneDX' { & (Join-Path $PSScriptRoot 'create_sbom.ps1') | Format-List }
    Invoke-Gate 'Manifeste et checksums' {
        & (Join-Path $PSScriptRoot 'create_release_manifest.ps1') -OutputPath $ManifestPath `
            -ExpectedVersion $ExpectedVersion -RequireClean:$RequireClean -RequireSigned:$RequireSigned | Format-List
    }
    Invoke-Gate 'Vérification de distribution' {
        & (Join-Path $PSScriptRoot 'verify_distribution.ps1') -ManifestPath $ManifestPath `
            -ExpectedVersion $ExpectedVersion -RequireClean:$RequireClean -RequireSigned:$RequireSigned | Format-List
    }
    Write-Host "`nRELEASE_VERIFICATION_PASS" -ForegroundColor Green
}
finally {
    Pop-Location
}
