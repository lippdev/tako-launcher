#Requires -Version 5.1
# Package-Web.ps1 - Monta a pasta e gera o instalador de prévia da versão web do Tako (Rust + Chromium/CEF)
# Uso:
#   .\build\rust\Package-Web.ps1
#   .\build\rust\Package-Web.ps1 -Version 0.2.0-preview

[CmdletBinding()]
param(
    [ValidateSet("x64")]
    [string]$Arch = "x64",
    [string]$Version = "",
    [string]$OutputDir = "",
    [string]$StageDir = "",
    [switch]$SkipBuild,
    [switch]$SkipTest,
    [switch]$CreatePortableZip
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

# 1. Determina a versão se não fornecida
if ([string]::IsNullOrWhiteSpace($Version)) {
    $cargoToml = Get-Content -LiteralPath (Join-Path $root "Cargo.toml") -Raw
    if ($cargoToml -match '(?m)^version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        $Version = "0.2.0-preview"
    }
}
$Version = $Version.TrimStart("v")

Write-Host "=== Empacotando Tako Web (prévia) v$Version ($Arch) ==="

$dist = if ($OutputDir) { $OutputDir } else { Join-Path $root "dist" }
$stage = if ($StageDir) { $StageDir } else { Join-Path $root "artifacts\rust\web-preview-$Arch" }

New-Item -ItemType Directory -Force -Path $dist, $stage | Out-Null

if (-not $SkipBuild) {
    # 2. Compilar front-end web (web/dist)
    Write-Host "-> Construindo interface web (web/)..."
    Push-Location (Join-Path $root "web")
    try {
        & npm ci
        if ($LASTEXITCODE -ne 0) { throw "npm ci falhou em web/" }
        & npm run build
        if ($LASTEXITCODE -ne 0) { throw "npm run build falhou em web/" }
    } finally {
        Pop-Location
    }

    # 3. Preparar CEF e compilar tako em release com feature web
    Write-Host "-> Preparando runtime CEF e compilando Tako (--release --features web)..."
    . (Join-Path $root "scripts\cef-setup.ps1")
    & cargo build -p tako --release --features web --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build falhou" }
} else {
    . (Join-Path $root "scripts\cef-setup.ps1")
}

# 4. Montar a pasta de saída (estágio)
Write-Host "-> Montando pasta de saída em: $stage"
Get-ChildItem -LiteralPath $stage -Force | Remove-Item -Recurse -Force

# 4.1. Executável tako.exe
$releaseDir = Join-Path $root "target\release"
$exeSource = Join-Path $releaseDir "tako.exe"
if (-not (Test-Path -LiteralPath $exeSource)) {
    throw "Executável tako.exe não encontrado em $exeSource"
}
Copy-Item -LiteralPath $exeSource -Destination (Join-Path $stage "tako.exe") -Force

# 4.2. Runtime do CEF
# O tako-web procura libcef.dll e os recursos na mesma pasta do executável em release
$cefRuntime = $env:CEF_PATH
if (-not $cefRuntime -or -not (Test-Path -LiteralPath $cefRuntime)) {
    throw "Caminho CEF_PATH não definido ou inválido ($cefRuntime)"
}

Write-Host "-> Copiando arquivos do runtime CEF a partir de $cefRuntime e $releaseDir..."
# Copiar DLLs, PAKs, DATs, BINs e JSONs do CEF
Get-ChildItem -LiteralPath $cefRuntime -File | Where-Object {
    $_.Extension -in @(".dll", ".pak", ".dat", ".bin", ".json") -and $_.Name -ne "archive.json"
} | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $stage -Force
}

# Copiar pasta locales/
$localesSource = Join-Path $cefRuntime "locales"
if (Test-Path -LiteralPath $localesSource) {
    Copy-Item -LiteralPath $localesSource -Destination (Join-Path $stage "locales") -Recurse -Force
} else {
    Write-Warning "Pasta locales não encontrada em $localesSource"
}

# Também conferir se cef-dll-sys copiou algo em target/release que não esteja no estágio
Get-ChildItem -LiteralPath $releaseDir -File | Where-Object {
    $_.Extension -in @(".dll", ".pak", ".dat", ".bin")
} | ForEach-Object {
    $destFile = Join-Path $stage $_.Name
    if (-not (Test-Path -LiteralPath $destFile)) {
        Copy-Item -LiteralPath $_.FullName -Destination $destFile -Force
    }
}
$releaseLocales = Join-Path $releaseDir "locales"
if ((Test-Path -LiteralPath $releaseLocales) -and -not (Test-Path -LiteralPath (Join-Path $stage "locales"))) {
    Copy-Item -LiteralPath $releaseLocales -Destination (Join-Path $stage "locales") -Recurse -Force
}

# 4.3. Interface web/dist
# target.rs em release procura std::env::current_exe().parent().join("web/dist")
$webDistSource = Join-Path $root "web\dist"
if (-not (Test-Path -LiteralPath (Join-Path $webDistSource "index.html"))) {
    throw "web/dist/index.html não encontrado em $webDistSource"
}
$webDistDest = Join-Path $stage "web\dist"
New-Item -ItemType Directory -Force -Path $webDistDest | Out-Null
Copy-Item -Path (Join-Path $webDistSource "*") -Destination $webDistDest -Recurse -Force

# Relatório da pasta de saída montada
$stageFiles = Get-ChildItem -LiteralPath $stage -Recurse -File
$stageBytes = ($stageFiles | Measure-Object -Property Length -Sum).Sum
$stageMb = [Math]::Round($stageBytes / 1MB, 2)
Write-Host "-> Pasta de saída montada com sucesso ($stageMb MB, $($stageFiles.Count) arquivos)."

# 5. Testar a pasta de saída fora do repositório
if (-not $SkipTest) {
    Write-Host "-> Testando execução da pasta montada fora do repositório..."
    $testDataDir = Join-Path ([System.IO.Path]::GetTempPath()) ("tako-test-web-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $testDataDir | Out-Null

    try {
        $testExe = Join-Path $stage "tako.exe"
        $proc = Start-Process -FilePath $testExe -ArgumentList @("--web", "--data-dir", $testDataDir) -WorkingDirectory $stage -PassThru
        $testPid = $proc.Id
        Write-Host "Processo principal de teste iniciado (PID: $testPid)"

        $deadline = [DateTime]::UtcNow.AddSeconds(20)
        $uiProcessStarted = $false
        $childPid = $null

        while ([DateTime]::UtcNow -lt $deadline) {
            if ($proc.HasExited) {
                throw "O processo de teste (PID $testPid) saiu prematuramente com código $($proc.ExitCode)."
            }

            # Procura subprocessos do tako.exe
            $children = Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq $testPid -and $_.Name -like "tako*" }
            if ($children) {
                $childPid = $children[0].ProcessId
                $uiProcessStarted = $true
                break
            }

            $logDir = Join-Path $testDataDir "logs"
            $logFile = Get-ChildItem -Path $logDir -Filter "tako-*.log" -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($logFile) {
                $content = Get-Content -LiteralPath $logFile.FullName -Raw -ErrorAction SilentlyContinue
                if ($content -match "CEF inicializado" -or $content -match "web-ui") {
                    $uiProcessStarted = $true
                    break
                }
            }
            Start-Sleep -Milliseconds 500
        }

        if (-not $uiProcessStarted) {
            throw "O processo da interface web não subiu dentro do tempo limite (20s)."
        }

        Write-Host "OK: Processo da interface subiu com sucesso (PID filho: $childPid)."

        # Aguardar para que a inicialização do CEF e o carregamento do pacote ocorram
        Start-Sleep -Seconds 2

        # Conferir que libcef.dll e os recursos estão sendo carregados a partir da pasta de saída
        # e não do repositório
        $logFile = Get-ChildItem -Path (Join-Path $testDataDir "logs") -Filter "tako-*.log" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($logFile) {
            $logContent = Get-Content -LiteralPath $logFile.FullName -Raw -ErrorAction SilentlyContinue
            if ($logContent -match "runtime CEF ausente" -or $logContent -match "pacote web ausente") {
                throw "Erro detectado no log da interface: $logContent"
            }
            Write-Host "OK: Log da sessão de teste verificado:"
            $logContent -split "`r?`n" | Where-Object { $_ -match "iniciando|aberta|CEF" } | ForEach-Object { Write-Host "   $_" }
        }

        Write-Host "Encerrando processo de teste (apenas PID $testPid)..."
        Stop-Process -Id $testPid -Force
        $proc.WaitForExit(5000)
        # Breve pausa para os subprocessos CEF encerrarem após fechamento do pipe
        Start-Sleep -Seconds 2
        Write-Host "OK: Teste da pasta concluído com sucesso."
    } finally {
        Remove-Item -LiteralPath $testDataDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# 6. Compilar o instalador com Inno Setup
function Get-InnoSetupCompiler {
    $cmd = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $knownPaths = @(
        (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe"),
        (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"),
        (Join-Path $env:ProgramFiles "Inno Setup 6\ISCC.exe")
    )
    foreach ($p in $knownPaths) {
        if ($p -and (Test-Path -LiteralPath $p)) { return $p }
    }

    Write-Host "ISCC.exe não encontrado. Tentando instalar via winget..."
    try {
        & winget install --id JRSoftware.InnoSetup -e --scope user --silent
    } catch {
        Write-Warning "winget falhou: $_"
    }

    foreach ($p in $knownPaths) {
        if ($p -and (Test-Path -LiteralPath $p)) { return $p }
    }
    $cmd = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    return $null
}

$iscc = Get-InnoSetupCompiler
$issScript = Join-Path $PSScriptRoot "tako-web.iss"

if ($iscc -and (Test-Path -LiteralPath $iscc)) {
    Write-Host "-> Gerando instalador com Inno Setup ($iscc)..."
    & $iscc "/DAppVersion=$Version" "/DSourceDir=$stage" "/DOutputDir=$dist" "/DArch=$Arch" $issScript
    if ($LASTEXITCODE -ne 0) { throw "ISCC falhou ao gerar o instalador" }

    $setupExe = Join-Path $dist "Tako-Web-Preview-Setup-$Arch.exe"
    if (Test-Path -LiteralPath $setupExe) {
        $sizeMb = [Math]::Round((Get-Item -LiteralPath $setupExe).Length / 1MB, 2)
        Write-Host "OK: Instalador gerado em $setupExe ($sizeMb MB)"
    }
} else {
    Write-Warning "Inno Setup não disponível. Para instalar: winget install --id JRSoftware.InnoSetup -e --scope user"
    Write-Host "Gerando pacote portátil compactado..."
    $CreatePortableZip = $true
}

if ($CreatePortableZip) {
    $zipPath = Join-Path $dist "Tako-Web-Preview-Portable-$Arch.zip"
    Write-Host "-> Criando arquivo portátil: $zipPath..."
    Compress-Archive -Path "$stage\*" -DestinationPath $zipPath -Force
    $zipSizeMb = [Math]::Round((Get-Item -LiteralPath $zipPath).Length / 1MB, 2)
    Write-Host "OK: Pacote portátil criado em $zipPath ($zipSizeMb MB)"
}

Write-Host "=== Concluído com sucesso! ==="
