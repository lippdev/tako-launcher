[CmdletBinding()]
param()

$OutputEncoding = [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'Stop'

function ConvertTo-NormalizedVersion {
    param([string]$versionString)
    if ([string]::IsNullOrWhiteSpace($versionString)) { return $null }
    $clean = $versionString -replace '\+.*$', '' -replace '-.*$', ''
    $parts = $clean.Split('.')
    while ($parts.Count -lt 2) {
        $clean += ".0"
        $parts = $clean.Split('.')
    }
    try {
        return [version]$clean
    } catch {
        return $null
    }
}

# 1. Leitura das versões locais do projeto
$cargoTomlPath = Join-Path $PSScriptRoot '../Cargo.toml'
$cargoLockPath = Join-Path $PSScriptRoot '../Cargo.lock'
$cefSetupPath  = Join-Path $PSScriptRoot 'cef-setup.ps1'

if (-not (Test-Path -LiteralPath $cargoTomlPath)) {
    throw "Arquivo Cargo.toml não encontrado em: $cargoTomlPath"
}
if (-not (Test-Path -LiteralPath $cargoLockPath)) {
    throw "Arquivo Cargo.lock não encontrado em: $cargoLockPath"
}
if (-not (Test-Path -LiteralPath $cefSetupPath)) {
    throw "Arquivo cef-setup.ps1 não encontrado em: $cefSetupPath"
}

# Extrai do Cargo.toml
$cargoTomlContent = Get-Content -LiteralPath $cargoTomlPath -Raw
$tomlMatch = [regex]::Match($cargoTomlContent, '(?m)^\s*cef\s*=\s*\{[^}]*version\s*=\s*"=?([^"]+)"')
if (-not $tomlMatch.Success) {
    throw 'Dependência cef não encontrada no Cargo.toml.'
}
$cargoTomlCrateVersion = $tomlMatch.Groups[1].Value

# Extrai do Cargo.lock
$cargoLockContent = Get-Content -LiteralPath $cargoLockPath -Raw
$lockMatch = [regex]::Match($cargoLockContent, '(?m)^name = "cef"\r?\nversion = "([^"+]+)\+([^"+]+)"')
if (-not $lockMatch.Success) {
    throw 'Versão da crate cef ausente ou inválida no Cargo.lock.'
}
$cargoLockBindingVersion = $lockMatch.Groups[1].Value
$cargoLockRuntimeVersion = $lockMatch.Groups[2].Value
$cargoLockFullVersion    = "$cargoLockBindingVersion+$cargoLockRuntimeVersion"

# Avalia a leitura do scripts/cef-setup.ps1
# O cef-setup.ps1 obtém a versão via regex idêntica no Cargo.lock
$cefSetupContent = Get-Content -LiteralPath $cefSetupPath -Raw
$setupUsesLockRegex = $cefSetupContent -match 'name = "cef"\\r\?\\nversion = "\[\^"\+\]\+\\+\[\^"\+\]\+"'

Write-Host "=== Versões locais do CEF no projeto ==="
Write-Host "Cargo.toml:     cef = $cargoTomlCrateVersion"
Write-Host "Cargo.lock:     $cargoLockFullVersion (binding: $cargoLockBindingVersion, runtime: $cargoLockRuntimeVersion)"
Write-Host "cef-setup.ps1:  resolvido via Cargo.lock (runtime: $cargoLockRuntimeVersion)"

$versionsMatch = ($cargoTomlCrateVersion -eq $cargoLockBindingVersion)
if ($versionsMatch) {
    Write-Host "Consistência interna: Cargo.toml e Cargo.lock/cef-setup.ps1 batem na versão base da crate ($cargoTomlCrateVersion)."
} else {
    Write-Warning "Divergência interna: Cargo.toml ($cargoTomlCrateVersion) difere de Cargo.lock/cef-setup.ps1 ($cargoLockBindingVersion)."
}

Write-Host ""
Write-Host "=== Consulta a índices remotos ==="

# 2. Consulta à versão estável no índice público do Spotify CDN para Windows 64-bit
$spotifyUrl = 'https://cef-builds.spotifycdn.com/index.json'
Write-Host "Consultando índice CEF Spotify CDN ($spotifyUrl)..."
$spotifyIndex = Invoke-RestMethod -Uri $spotifyUrl -Headers @{ 'User-Agent' = 'tako-cef-check' }

if (-not $spotifyIndex.windows64 -or -not $spotifyIndex.windows64.versions) {
    throw 'Índice do Spotify CDN não contém a seção windows64 esperada.'
}

$latestStableWin64 = $spotifyIndex.windows64.versions | Where-Object { $_.channel -eq 'stable' } | Select-Object -First 1
if (-not $latestStableWin64) {
    throw 'Nenhuma versão com canal "stable" encontrada para windows64 no índice do Spotify CDN.'
}

$remoteCefFull = $latestStableWin64.cef_version
$remoteChromium = $latestStableWin64.chromium_version
if ($remoteCefFull -match '^([^+]+)') {
    $remoteCefClean = $matches[1]
} else {
    $remoteCefClean = $remoteCefFull
}

# 3. Consulta à versão mais recente do crate cef no crates.io
$cratesUrl = 'https://crates.io/api/v1/crates/cef'
Write-Host "Consultando crates.io ($cratesUrl)..."
$cratesData = Invoke-RestMethod -Uri $cratesUrl -Headers @{ 'User-Agent' = 'tako-cef-check' }
$remoteCrateVersion = $cratesData.crate.max_stable_version
if ([string]::IsNullOrWhiteSpace($remoteCrateVersion)) {
    $remoteCrateVersion = $cratesData.crate.max_version
}

Write-Host "CEF estável mais recente (Spotify CDN): $remoteCefClean (Chromium $remoteChromium)"
Write-Host "Crate cef mais recente (crates.io):    $remoteCrateVersion"
Write-Host ""

# 4. Comparação
$currentRuntimeNorm = ConvertTo-NormalizedVersion $cargoLockRuntimeVersion
$remoteCefNorm      = ConvertTo-NormalizedVersion $remoteCefClean

$hasUpdate = $false
if ($null -ne $currentRuntimeNorm -and $null -ne $remoteCefNorm) {
    if ($currentRuntimeNorm -lt $remoteCefNorm) {
        $hasUpdate = $true
    }
} elseif ($cargoLockRuntimeVersion -ne $remoteCefClean) {
    $hasUpdate = $true
}

# Também confere se a crate no crates.io tem versão mais nova
if ($remoteCrateVersion -match '^([^+]+)\+([^+]+)') {
    $remoteCrateBinding = $matches[1]
    $remoteCrateRuntime = $matches[2]
    $remoteCrateBindingNorm = ConvertTo-NormalizedVersion $remoteCrateBinding
    $localCrateBindingNorm  = ConvertTo-NormalizedVersion $cargoLockBindingVersion
    if ($null -ne $localCrateBindingNorm -and $null -ne $remoteCrateBindingNorm -and $localCrateBindingNorm -lt $remoteCrateBindingNorm) {
        $hasUpdate = $true
    }
    $remoteCrateRuntimeNorm = ConvertTo-NormalizedVersion $remoteCrateRuntime
    if ($null -ne $currentRuntimeNorm -and $null -ne $remoteCrateRuntimeNorm -and $currentRuntimeNorm -lt $remoteCrateRuntimeNorm) {
        $hasUpdate = $true
    }
}

# 5. Saída e código de retorno
Write-Host "=== Resultado da verificação ==="
if ($hasUpdate) {
    Write-Host "há versão nova: $remoteCefClean (Chromium $remoteChromium)"
    Write-Host ""
    Write-Host "Versão atual do projeto: $cargoLockRuntimeVersion (crate $cargoLockFullVersion)"
    Write-Host "Versão remota estável:   $remoteCefClean (Chromium $remoteChromium, crate $remoteCrateVersion)"
    Write-Host ""
    Write-Host "Notas de versão e referências:"
    Write-Host "  - Notas de versão e builds do CEF: https://cef-builds.spotifycdn.com/index.html"
    Write-Host "  - Atualizações de segurança do Chromium: https://chromereleases.googleblog.com/search/label/Stable%20Updates"
    Write-Host "  - Releases da crate cef: https://github.com/tauri-apps/cef-rs/releases"
    Write-Host "  - Procedimento de atualização: docs/web/atualizar-cef.md"
    exit 1
} else {
    Write-Host "em dia"
    Write-Host ""
    Write-Host "O runtime do CEF usado pelo projeto ($cargoLockRuntimeVersion, Chromium $remoteChromium) está na versão estável mais recente."
    Write-Host "Crate cef no crates.io: $remoteCrateVersion"
    Write-Host ""
    Write-Host "Notas de versão e referências:"
    Write-Host "  - Notas de versão e builds do CEF: https://cef-builds.spotifycdn.com/index.html"
    Write-Host "  - Atualizações de segurança do Chromium: https://chromereleases.googleblog.com/search/label/Stable%20Updates"
    Write-Host "  - Releases da crate cef: https://github.com/tauri-apps/cef-rs/releases"
    exit 0
}
