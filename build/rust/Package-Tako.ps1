#Requires -Version 5.1
# Pacotes do Tako (Rust) para Windows (PRD D-07, MIG-040), com os mesmos nomes das alphas C#, para o atualizador
# delas encontrar a versão Rust:
#   dist\Tako-Portable-<arch>.exe  o próprio executável (dados em Tako_Data ao lado dele)
#   dist\Tako-Setup-<arch>.exe     instalador por usuário (Inno Setup, mesmo AppId; dados em %LOCALAPPDATA%\Tako)
# Credenciais do ScreenScraper (ADR-0022): TAKO_SCREENSCRAPER_DEV_ID/_DEV_PASSWORD/_SOFT_NAME no ambiente.

param(
    [ValidateSet("x64", "arm64")]
    [string]$Arch = "x64",
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$Version = $Version.TrimStart("v")
if ($Version -notmatch '^(\d+)\.(\d+)\.(\d+)(-[0-9A-Za-z.\-]+)?$') { throw "Versão inválida: '$Version'" }
$triple = if ($Arch -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
$dist = Join-Path $root "dist"
$stage = Join-Path $root "artifacts\rust\windows-$Arch"

New-Item -ItemType Directory -Force -Path $dist, $stage | Out-Null
rustup target add $triple | Out-Null
$env:TAKO_VERSION = $Version
cargo build --release --locked -p tako --target $triple
if ($LASTEXITCODE -ne 0) { throw "cargo build falhou" }

# FSE-001: manifesto, SCCD e logo são include_str!/include_bytes! no tako-platform.
# Instalador e portable carregam os mesmos recursos; extração local só ao ativar.
$exe = Join-Path $root "target\$triple\release\tako.exe"
# Tako.exe (maiúsculo), como nas alphas: atalhos e a chave Run continuam valendo.
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage "Tako.exe") -Force
Copy-Item -LiteralPath $exe -Destination (Join-Path $dist "Tako-Portable-$Arch.exe") -Force

$iscc = Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"
if (-not (Test-Path -LiteralPath $iscc)) { throw "Inno Setup 6 não encontrado ($iscc)" }
& $iscc "/DAppVersion=$Version" "/DSourceDir=$stage" "/DOutputDir=$dist" "/DArch=$Arch" (Join-Path $PSScriptRoot "tako.iss")
if ($LASTEXITCODE -ne 0) { throw "ISCC falhou" }

Get-ChildItem -LiteralPath $dist -Filter "Tako-*-$Arch.exe" | ForEach-Object { Write-Host "OK: $($_.Name)" }
