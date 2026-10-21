#Requires -Version 5.1
# Fluxo real: executar apenas em Windows descartável, como o runner do CI.
param([Parameter(Mandatory=$true)][string]$PayloadDirectory)
$ErrorActionPreference = 'Stop'
$windows = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
Write-Host "Windows: $($windows.ProductName), build $($windows.CurrentBuild).$($windows.UBR)"
function Assert($condition, $message) { if (-not $condition) { throw $message } }
$layout = Join-Path $env:TEMP ('Tako FSE ' + [guid]::NewGuid())
$gaming = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\GamingConfiguration'
$dev = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock'
$hadGaming = Test-Path $gaming
$oldHome = (Get-ItemProperty $gaming -ErrorAction SilentlyContinue).GamingHomeApp
$oldDev = (Get-ItemProperty $dev -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
$trust = @(Get-ChildItem Cert:\LocalMachine\TrustedPeople | ForEach-Object Thumbprint)
$script = Join-Path $PSScriptRoot '../src/fse.ps1'
New-Item -ItemType Directory $layout | Out-Null
Copy-Item "$PayloadDirectory\Tako-GamingHome.msix", "$PayloadDirectory\Tako-GamingHome.cer" $layout
# O runner não recebe o rollout de GamingConfiguration; criar somente a chave de detecção.
if (-not $hadGaming) { New-Item -Path $gaming -Force | Out-Null }
# ExternalLocation deve conter o exe fixo; não é necessário iniciar a UI.
Copy-Item "$env:WINDIR\System32\whoami.exe" "$layout\Tako.exe"
try {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script -Action enable -Layout $layout -Executable "$layout\Tako.exe"
    Assert ($LASTEXITCODE -eq 0) 'Enable real falhou; consulte o erro AppX e a versão do Windows do runner.'
    $package = Get-AppxPackage -Name Tako.GamingHome
    Assert ($null -ne $package) 'Identidade não registrada.'
    $manifest = Get-AppxPackageManifest -Package $package.PackageFullName
    Assert ($manifest.OuterXml.Contains('windows.gamingApp')) 'Manifesto instalado sem extensão gamingApp.'
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $catalog = [Windows.ApplicationModel.AppExtensions.AppExtensionCatalog,Windows.ApplicationModel.AppExtensions,ContentType=WindowsRuntime]::Open('windows.gamingApp')
    [Windows.ApplicationModel.AppExtensions.AppExtension,Windows.ApplicationModel.AppExtensions,ContentType=WindowsRuntime] | Out-Null
    $operation = $catalog.FindAllAsync()
    $resultType = [Collections.Generic.IReadOnlyList[Windows.ApplicationModel.AppExtensions.AppExtension]]
    $asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetGenericArguments().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
    $task = $asTask.MakeGenericMethod($resultType).Invoke($null, @($operation))
    $task.Wait()
    Assert (@($task.Result | Where-Object { $_.Package.Id.FamilyName -eq $package.PackageFamilyName }).Count -eq 1) 'Catálogo WinRT não publicou o Tako.'
} finally {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script -Action disable -Layout $layout -Executable "$layout\Tako.exe"
    Assert ($LASTEXITCODE -eq 0) 'Disable real falhou.'
    Assert (-not (Get-AppxPackage -Name Tako.GamingHome)) 'Pacote permaneceu registrado.'
    Assert ((Get-ItemProperty $gaming -ErrorAction SilentlyContinue).GamingHomeApp -eq $oldHome) 'GamingHomeApp não restaurado.'
    Assert ((Get-ItemProperty $dev -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense -eq $oldDev) 'Dev Mode não restaurado.'
    $afterTrust = @(Get-ChildItem Cert:\LocalMachine\TrustedPeople | ForEach-Object Thumbprint)
    Assert ((($trust | Sort-Object) -join ',') -eq (($afterTrust | Sort-Object) -join ',')) 'Confiança não restaurada.'
    if (-not $hadGaming) { Remove-Item $gaming }
    Remove-Item $layout -Recurse -Force
}
Write-Host 'FSE: enable, identidade, catálogo WinRT, disable e restauração verificados.'
