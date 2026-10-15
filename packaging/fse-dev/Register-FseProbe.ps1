<#
.SYNOPSIS
  Registra (ou remove) a FseProbe como gaming home app de desenvolvimento do Xbox mode. Spike MVP-007.

.DESCRIPTION
  Monta packaging/fse-dev/layout e o registra com Add-AppxPackage -Register (sem MSIX, sem certificado).
  No kit da release, a probe já vem compilada na pasta probe\ ao lado do script; no repositório,
  o script publica tools/Launcher.FseProbe (exige o SDK do .NET). Exige Modo de Desenvolvedor ligado,
  porque o SCCD usa o catálogo de desenvolvimento (FFFF).

.EXAMPLE
  .\Register-FseProbe.ps1
  .\Register-FseProbe.ps1 -Unregister
#>
param([switch]$Unregister)

$ErrorActionPreference = 'Stop'
$packageName = 'Tako.FseProbe'
$here = $PSScriptRoot
$layout = Join-Path $here 'layout'
$prebuilt = Join-Path $here 'probe'

if ($Unregister) {
    Get-AppxPackage -Name $packageName | Remove-AppxPackage
    Write-Host "Pacote $packageName removido."
    return
}

$devMode = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock' -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
if ($devMode -ne 1) {
    throw 'Ligue o Modo de Desenvolvedor (Configurações > Sistema > Avançado) e rode de novo.'
}

if (Test-Path $layout) {
    Remove-Item $layout -Recurse -Force
}

if (Test-Path (Join-Path $prebuilt 'Launcher.FseProbe.exe')) {
    Copy-Item $prebuilt $layout -Recurse
}
else {
    $repo = Resolve-Path (Join-Path $here '..\..')
    dotnet publish (Join-Path $repo 'tools\Launcher.FseProbe') -c Release -o $layout
    if ($LASTEXITCODE -ne 0) {
        throw 'dotnet publish falhou.'
    }
}

Copy-Item (Join-Path $here 'AppxManifest.xml'), (Join-Path $here 'CustomCapability.SCCD') $layout
Copy-Item (Join-Path $here 'Assets') $layout -Recurse
New-Item -ItemType Directory -Path (Join-Path $layout 'Public') -Force | Out-Null

Add-AppxPackage -Register (Join-Path $layout 'AppxManifest.xml') -ForceUpdateFromAnyVersion
Get-AppxPackage -Name $packageName | Select-Object PackageFamilyName, SignatureKind, IsDevelopmentMode | Format-List

Write-Host 'Registrado. Agora: Configurações > Jogos > Xbox mode > Escolher app inicial > Tako FseProbe.'
