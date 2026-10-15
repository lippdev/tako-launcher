#Requires -Version 5.1
# Publica o Tako (WPF, self-contained), no mesmo formato do Console Mode:
#   dist\Tako-Portable-<arch>.exe  um único .exe portátil (dados em Tako_Data ao lado do exe)
#   dist\Tako-Setup-<arch>.exe     instalador por usuário (Inno Setup; dados em %LOCALAPPDATA%\Tako)
#   dist\Tako-FseKit-<arch>.zip    kit do experimento do Xbox mode (MVP-007), com -Target All ou FseKit
# O PresentMon da Intel (MIT), usado pelo contador de FPS (ADR-0017), é baixado da release oficial e conferido pelo
# SHA-256 abaixo; não fica no repositório. Só existe x64: no ARM64 o contador aparece como indisponível.

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

# PresentMon 2.6.0 (GameTechDev/PresentMon, MIT). Para trocar de versão: nova URL, novo SHA-256 (o "digest" da release
# no GitHub) e PresentMonArguments.Version em ConsoleMode.Runtime.
$presentMonVersion = "2.6.0"
$presentMonUrl = "https://github.com/GameTechDev/PresentMon/releases/download/v$presentMonVersion/PresentMon-$presentMonVersion-x64.exe"
$presentMonSha256 = "b2a706bc6ad475749e3b7e3409263aa1e6906d45bdcf993f6dbc0f660188f1af"

function Get-PresentMon {
    if ($arch -ne "x64") {
        Write-Warning "PresentMon só tem build x64; o contador de FPS fica indisponível em $arch."
        return $null
    }
    $dir = Join-Path $root "artifacts\presentmon\$presentMonVersion"
    $file = Join-Path $dir "PresentMon.exe"
    if (Test-Path -LiteralPath $file) {
        if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -eq $presentMonSha256) { return $file }
        Remove-Item -LiteralPath $file -Force
    }
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    $download = "$file.download"
    Write-Host "Baixando PresentMon $presentMonVersion..."
    Invoke-WebRequest -Uri $presentMonUrl -OutFile $download -UseBasicParsing
    $hash = (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash
    if ($hash -ne $presentMonSha256) {
        Remove-Item -LiteralPath $download -Force
        throw "SHA-256 do PresentMon não confere (esperado $presentMonSha256, veio $hash). Nada foi publicado."
    }
    Move-Item -LiteralPath $download -Destination $file -Force
    return $file
}

$presentMon = $null
if ($Target -in "All", "Portable", "Installer") {
    $presentMon = Get-PresentMon
}

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
    if ($presentMon -and $ProjectPath -eq $project) {
        $pubArgs += "-p:PresentMonExe=$presentMon"
    }
    # Temas e fontes vão dentro do .exe e são extraídos na primeira execução.
    # A compressão reduz o portátil (~141 MB) pela metade, como no Console Mode.
    if ($SingleFile) {
        $pubArgs += "-p:IncludeAllContentForSelfExtract=true"
        $pubArgs += "-p:EnableCompressionInSingleFile=true"
    }
    & dotnet @pubArgs
    if ($LASTEXITCODE -ne 0) { throw "dotnet publish falhou ($OutDir)" }
    if (-not (Test-Path -LiteralPath (Join-Path $OutDir $Exe))) {
        throw "Publish não gerou $Exe em $OutDir"
    }
    if ($presentMon -and $ProjectPath -eq $project -and -not $SingleFile -and -not (Test-Path -LiteralPath (Join-Path $OutDir "PresentMon\PresentMon.exe"))) {
        throw "Publish não incluiu o PresentMon em $OutDir"
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
