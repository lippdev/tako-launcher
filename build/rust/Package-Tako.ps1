#Requires -Version 5.1
# Instalador do Tako (Rust) para Windows (PRD D-07, D-11, MIG-040, INST-01). Desde a 0.2.0-alpha.23 a interface
# padrão é a web (React sobre o Chromium embutido, CEF); o Slint continua no executável (`Tako.exe --slint`).
#   dist\Tako-Setup-<arch>.exe   instalador por usuário (Inno Setup, mesmo AppId das alphas; dados em
#                                %LOCALAPPDATA%\Tako). Leva Tako.exe, o runtime do CEF e o web\dist.
# Sem portátil: com o CEF o Tako é uma pasta de ~450 MB, não um .exe só.
# Credenciais do ScreenScraper (ADR-0022): TAKO_SCREENSCRAPER_DEV_ID/_DEV_PASSWORD/_SOFT_NAME no ambiente.
# Precisa de Node 24 (npm), do Rust e do Inno Setup 6; o CEF vem do scripts\cef-setup.ps1 (cache por versão).

param(
    [ValidateSet("x64")]
    [string]$Arch = "x64",
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$Version = $Version.TrimStart("v")
if ($Version -notmatch '^(\d+)\.(\d+)\.(\d+)(-[0-9A-Za-z.\-]+)?$') { throw "Versão inválida: '$Version'" }
$dist = Join-Path $root "dist"
$stage = Join-Path $root "artifacts\rust\windows-$Arch"
# O que o instalador copia para {app}; o payload do FSE fica ao lado, fora do instalador.
$app = Join-Path $stage "app"

New-Item -ItemType Directory -Force -Path $dist, $stage | Out-Null
$env:TAKO_VERSION = $Version

# 1. Interface web (web\dist). Com node_modules presente (um Vite aberto trava os binários), usa o que já está lá.
Push-Location (Join-Path $root "web")
try {
    if (-not (Test-Path "node_modules")) {
        & npm ci
        if ($LASTEXITCODE -ne 0) { throw "npm ci falhou em web\" }
    }
    & npm run build
    if ($LASTEXITCODE -ne 0) { throw "npm run build falhou em web\" }
} finally {
    Pop-Location
}

# 2. Runtime do CEF (CEF_PATH) e o executável com a interface web.
. (Join-Path $root "scripts\cef-setup.ps1")
$cef = $env:CEF_PATH
if (-not $cef -or -not (Test-Path -LiteralPath (Join-Path $cef "libcef.dll"))) { throw "Runtime do CEF não encontrado ($cef)" }
# FSE-002: MSIX e certificado público são embutidos no tako-platform.
$env:TAKO_FSE_PAYLOAD_DIR = Join-Path $stage 'fse'
& (Join-Path $PSScriptRoot 'Package-Fse.ps1') -OutputDirectory $env:TAKO_FSE_PAYLOAD_DIR
# Sem `--target`: o host já é x64, e o triplo a mais no caminho passa dos 260 caracteres nos .obj do wrapper do CEF.
cargo build --release --locked -p tako --features web
if ($LASTEXITCODE -ne 0) { throw "cargo build falhou" }

# 3. Pasta do app, montada do zero: nada de uma versão anterior do CEF ou da página sobra nela.
if (Test-Path -LiteralPath $app) { Remove-Item -LiteralPath $app -Recurse -Force }
New-Item -ItemType Directory -Force -Path $app | Out-Null
# Tako.exe (maiúsculo), como nas alphas: atalhos, a chave Run e o tako:// continuam valendo.
Copy-Item -LiteralPath (Join-Path $root "target\release\tako.exe") -Destination (Join-Path $app "Tako.exe")
# O tako-web procura a libcef.dll e os recursos ao lado do executável. archive.json é só do cef-setup.
Get-ChildItem -LiteralPath $cef -File |
    Where-Object { $_.Extension -in @(".dll", ".pak", ".dat", ".bin", ".json") -and $_.Name -ne "archive.json" } |
    Copy-Item -Destination $app
Copy-Item -LiteralPath (Join-Path $cef "locales") -Destination (Join-Path $app "locales") -Recurse
# Licenças do Chromium e do CEF (BSD e afins), que acompanham o runtime redistribuído.
Copy-Item -LiteralPath (Join-Path $cef "CREDITS.html") -Destination (Join-Path $app "CEF-CREDITS.html")
$webDist = Join-Path $root "web\dist"
if (-not (Test-Path -LiteralPath (Join-Path $webDist "index.html"))) { throw "web\dist\index.html não encontrado" }
New-Item -ItemType Directory -Force -Path (Join-Path $app "web") | Out-Null
Copy-Item -LiteralPath $webDist -Destination (Join-Path $app "web\dist") -Recurse
foreach ($required in @("Tako.exe", "libcef.dll", "chrome_elf.dll", "icudtl.dat", "resources.pak", "v8_context_snapshot.bin", "locales\en-US.pak", "web\dist\index.html")) {
    if (-not (Test-Path -LiteralPath (Join-Path $app $required))) { throw "Pacote incompleto: falta $required" }
}
$files = Get-ChildItem -LiteralPath $app -Recurse -File
$mb = [Math]::Round((($files | Measure-Object -Property Length -Sum).Sum) / 1MB, 1)
Write-Host "Pasta do app: $app ($mb MB, $($files.Count) arquivos)"

# 4. Instalador. O ISCC instalado por usuário (winget) fica em %LOCALAPPDATA%\Programs.
$iscc = @(
    (Get-Command "ISCC.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1),
    (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"),
    (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe")
) | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
if (-not $iscc) { throw "Inno Setup 6 não encontrado (winget install --id JRSoftware.InnoSetup -e --scope user)" }
& $iscc "/DAppVersion=$Version" "/DSourceDir=$app" "/DOutputDir=$dist" "/DArch=$Arch" (Join-Path $PSScriptRoot "tako.iss")
if ($LASTEXITCODE -ne 0) { throw "ISCC falhou" }

Get-ChildItem -LiteralPath $dist -Filter "Tako-Setup-$Arch.exe" | ForEach-Object {
    Write-Host "OK: $($_.Name) ($([Math]::Round($_.Length / 1MB, 1)) MB)"
}
