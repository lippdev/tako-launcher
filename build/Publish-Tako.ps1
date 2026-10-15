#Requires -Version 5.1
# Publica o Tako (WPF, self-contained), no mesmo formato do Console Mode:
#   dist\Tako-Portable-<arch>.exe  um único .exe portátil (dados em Tako_Data ao lado do exe)
#   dist\Tako-Setup-<arch>.exe     instalador por usuário (Inno Setup; dados em %LOCALAPPDATA%\Tako)
#   dist\Tako-FseKit-<arch>.zip    kit do experimento do Xbox mode (MVP-007), com -Target All ou FseKit

param(
    [ValidateSet("x64", "ARM64")]
    [string]$Runtime = "x64",
    [ValidateSet("All", "Portable", "Installer", "FseKit")]
    [string]$Target = "All",
    # Ex.: 0.1.0 ou 0.1.0-alpha.5 (sem o "v"). Vazio = versão do .csproj.
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$project = Join-Path $root "src\Launcher.UI.Wpf\Launcher.UI.Wpf.csproj"
$arch = $Runtime.ToLowerInvariant()
$rid = "win-$arch"
$singleDir = Join-Path $root "artifacts\publish\$rid-single"
$appDir = Join-Path $root "artifacts\publish\$rid-app"
$distDir = Join-Path $root "dist"

if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
    throw "dotnet SDK não encontrado. Instale o .NET 10 SDK no Windows."
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = ([xml](Get-Content -LiteralPath $project -Raw)).Project.PropertyGroup.Version | Where-Object { $_ } | Select-Object -First 1
}
$Version = $Version.TrimStart("v")
if ($Version -notmatch '^(\d+)\.(\d+)\.(\d+)(-[0-9A-Za-z.\-]+)?$') {
    throw "Versão inválida: '$Version' (use 1.2.3 ou 1.2.3-beta.1)"
}
$numericVersion = "$($Matches[1]).$($Matches[2]).$($Matches[3]).0"
Write-Host "Versão: $Version"

New-Item -ItemType Directory -Path $distDir -Force | Out-Null

function Invoke-Publish([string]$ProjectPath, [string]$OutDir, [bool]$SingleFile, [string]$Exe) {
    if (Test-Path -LiteralPath $OutDir) { Remove-Item -LiteralPath $OutDir -Recurse -Force }
    $pubArgs = @(
        "publish", $ProjectPath,
        "-c", "Release",
        "-r", $rid,
        "--self-contained", "true",
        "-p:PublishSingleFile=$($SingleFile.ToString().ToLowerInvariant())",
        "-p:PublishTrimmed=false",
        "-p:Version=$Version",
        "-p:InformationalVersion=$Version",
        "-p:AssemblyVersion=$numericVersion",
        "-p:FileVersion=$numericVersion",
        "-o", $OutDir
    )
    # Temas e fontes vão dentro do .exe e são extraídos na primeira execução.
    if ($SingleFile) { $pubArgs += "-p:IncludeAllContentForSelfExtract=true" }
    & dotnet @pubArgs
    if ($LASTEXITCODE -ne 0) { throw "dotnet publish falhou ($OutDir)" }
    if (-not (Test-Path -LiteralPath (Join-Path $OutDir $Exe))) {
        throw "Publish não gerou $Exe em $OutDir"
    }
}

if ($Target -in "All", "Portable") {
    Write-Host "Publicando versão portátil (um único .exe)..."
    Invoke-Publish $project $singleDir $true "Tako.exe"
    $portable = Join-Path $distDir "Tako-Portable-$arch.exe"
    Copy-Item -LiteralPath (Join-Path $singleDir "Tako.exe") -Destination $portable -Force
    Write-Host "OK: $portable"
}

if ($Target -in "All", "Installer") {
    $iscc = @(
        (Get-Command iscc -ErrorAction SilentlyContinue).Source,
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1

    if (-not $iscc) {
        $msg = "Inno Setup 6 (ISCC.exe) não encontrado; instalador não gerado. Instale com: winget install JRSoftware.InnoSetup"
        if ($Target -eq "Installer") { throw $msg }
        Write-Warning $msg
    }
    else {
        Write-Host "Publicando versão para o instalador (pasta)..."
        Invoke-Publish $project $appDir $false "Tako.exe"
        & $iscc "/Q" "/DAppVersion=$Version" "/DSourceDir=$appDir" "/DOutputDir=$distDir" "/DArch=$arch" (Join-Path $PSScriptRoot "Tako.iss")
        if ($LASTEXITCODE -ne 0) { throw "ISCC falhou" }
        Write-Host "OK: $(Join-Path $distDir "Tako-Setup-$arch.exe")"
    }
}

if ($Target -in "All", "FseKit") {
    Write-Host "Montando o kit do experimento FSE..."
    $kitDir = Join-Path $root "artifacts\publish\fsekit-$arch"
    Invoke-Publish (Join-Path $root "tools\Launcher.FseProbe\Launcher.FseProbe.csproj") (Join-Path $kitDir "probe") $false "Launcher.FseProbe.exe"
    $fse = Join-Path $root "packaging\fse-dev"
    Copy-Item (Join-Path $fse "AppxManifest.xml"), (Join-Path $fse "CustomCapability.SCCD"), (Join-Path $fse "Register-FseProbe.ps1"), (Join-Path $fse "LEIA-ME.txt") $kitDir
    Copy-Item (Join-Path $fse "Assets") $kitDir -Recurse -Force
    $kit = Join-Path $distDir "Tako-FseKit-$arch.zip"
    Compress-Archive (Join-Path $kitDir "*") $kit -Force
    Write-Host "OK: $kit"
}
