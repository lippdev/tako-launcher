<#
.SYNOPSIS
  Gera os pacotes do Tako: instalador (Inno Setup), portable (zip) e kit do experimento FSE.

.EXAMPLE
  ./packaging/build-packages.ps1 -Version 0.1.0-alpha.5 -OutDir artifacts
#>
param(
    [Parameter(Mandatory)] [string] $Version,
    [Parameter(Mandatory)] [string] $OutDir
)

$ErrorActionPreference = 'Stop'
$repo = Resolve-Path (Join-Path $PSScriptRoot '..')
$OutDir = [IO.Path]::GetFullPath($OutDir)
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Invoke-Checked([scriptblock] $Command, [string] $What) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What falhou ($LASTEXITCODE)" }
}

# Self-contained: roda sem instalar o .NET.
$app = Join-Path $OutDir 'Tako'
Invoke-Checked { dotnet publish (Join-Path $repo 'src/Launcher.UI.Wpf') -c Release -r win-x64 --self-contained -p:Version=$Version -o $app } 'dotnet publish (Tako)'

# Portable: mesma pasta + portable.txt (dados em Data\ ao lado do Tako.exe).
$portable = Join-Path $OutDir 'Tako-Portable'
Copy-Item $app $portable -Recurse -Force
Copy-Item (Join-Path $repo 'packaging/release/portable.txt') (Join-Path $portable 'portable.txt')
Copy-Item (Join-Path $repo 'packaging/release/LEIA-ME-portable.txt') (Join-Path $portable 'LEIA-ME.txt')
Copy-Item (Join-Path $repo 'packaging/release/LEIA-ME-launcher.txt') (Join-Path $portable 'COMO-TESTAR.txt')
Compress-Archive (Join-Path $portable '*') (Join-Path $OutDir "Tako-$Version-Portable-x64.zip") -Force

# Instalador (Inno Setup 6): usa o instalado ou instala pelo Chocolatey (runners do GitHub).
$iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
if (-not (Test-Path $iscc)) {
    Invoke-Checked { choco install innosetup -y --no-progress } 'choco install innosetup'
}
$numeric = ($Version -split '-')[0] + '.0'
Invoke-Checked { & $iscc "/DAppVersion=$Version" "/DNumericVersion=$numeric" "/DSourceDir=$app" "/DOutputDir=$OutDir" (Join-Path $repo 'packaging/installer/Tako.iss') } 'Inno Setup'

# Kit do experimento FSE (MVP-007).
$kit = Join-Path $OutDir 'FseKit'
Invoke-Checked { dotnet publish (Join-Path $repo 'tools/Launcher.FseProbe') -c Release -r win-x64 --self-contained -p:Version=$Version -o (Join-Path $kit 'probe') } 'dotnet publish (FseProbe)'
$fse = Join-Path $repo 'packaging/fse-dev'
Copy-Item (Join-Path $fse 'AppxManifest.xml'), (Join-Path $fse 'CustomCapability.SCCD'), (Join-Path $fse 'Register-FseProbe.ps1') $kit
Copy-Item (Join-Path $fse 'Assets') $kit -Recurse -Force
Copy-Item (Join-Path $repo 'packaging/release/LEIA-ME-fse.txt') (Join-Path $kit 'LEIA-ME.txt')
Compress-Archive (Join-Path $kit '*') (Join-Path $OutDir "Tako-FseKit-$Version-win-x64.zip") -Force

Get-ChildItem $OutDir -File | Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table | Out-String | Write-Host
