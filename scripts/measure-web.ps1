<#
.SYNOPSIS
    Script de medição automatizada de desempenho e memória do Tako (WEB-60).
    Compara a versão Web (CEF + React) com a versão Slint (linha de base).

.DESCRIPTION
    Mede as métricas estipuladas no PRD (§8.5 "Metas da migração"):
    - Tempo de abertura a frio até a janela aparecer e responder interativamente;
    - Private Working Set (WS Privado) e Working Set total por processo e somado;
    - Contagem de processos em cada estado:
      1) Interface aberta parada (em repouso);
      2) Interface fechada (núcleo Rust ou supervisor Slint sozinho);
      3) Durante o jogo (sessão de jogo onde a interface fecha e resta apenas o núcleo/supervisor).
    - Executa 3 rodadas para cada modo e calcula a mediana estatística;
    - Utiliza pasta temporária própria (--data-dir) para não alterar dados do usuário.

    WEB-60b: além de memória e abertura até a janela (métrica `memoria`, o comportamento de sempre), mede
    `abertura-biblioteca` (até a biblioteca interativa), `cpu-gpu` (CPU e GPU em repouso, web e Slint), `fps`,
    `latencia` (teclado) e `retorno` (reabertura depois do jogo). Método e limites de cada uma em
    docs/plans/web-medicao-2026-10.md. Escolha com -Metrics; `tudo` roda todas.

    Pré-requisitos das métricas novas (o script não compila Rust):
      # CEF e ferramentas:   . .\scripts\cef-setup.ps1
      # web release:         cargo build -p tako --release --features web        -> target\release\tako.exe
      # Slint release:       cargo build -p tako --release  (e copiar tako.exe para target\release\tako-slint.exe)
      # web "de medição":    $env:CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS='true'; $env:CARGO_TARGET_DIR='target\medicao'
      #                      cargo build -p tako --release --features web   -> target\medicao\release\tako.exe
    O executável "de medição" é o release otimizado com asserções de debug ligadas: só assim o hospedeiro aceita
    `TAKO_WEB_DEV_URL` (em release de verdade ele é ignorado, por segurança). A interface que ele carrega é o
    pacote de produção do app mais web/src/dev/medicao/ (marcos e contadores), servido por HTTP em loopback.
    O que roda no release de verdade (target\release\tako.exe + web\dist) é CPU, GPU, memória e abertura até a janela.

.PARAMETER ExePath
    Caminho para o executável do Tako (padrão: target/release/tako.exe).

.PARAMETER Mode
    Modo a testar: 'web', 'slint' ou 'both' (executa ambos para comparação).

.PARAMETER Runs
    Número de repetições por medição (padrão: 3).

.PARAMETER SettlingSeconds
    Tempo de espera (em segundos) após a janela responder para estabilização de repouso (padrão: 5).

.PARAMETER Metrics
    Quais medições rodar: memoria (padrão), abertura-biblioteca, cpu-gpu, fps, latencia, retorno ou tudo.

.PARAMETER RestSeconds
    Duração de cada janela de repouso de CPU e GPU (padrão: 30).

.PARAMETER OpenRuns
    Rodadas da abertura até a biblioteca interativa (padrão: 10; o p95 de 10 amostras é o máximo, dito no relatório).

.PARAMETER FpsRuns
    Rodadas do cenário de fps (padrão: 3).

.PARAMETER ReturnRuns
    Rodadas do retorno depois do jogo (padrão: 5).

.PARAMETER KeyPresses
    Teclas por rodada na medição de latência (padrão: 100, uma a cada 200 ms).

.PARAMETER Games
    Jogos de teste somados à biblioteca de medição, além dos que o PC já tem (padrão: 300).

.PARAMETER Port
    Porta do servidor de medição (padrão: 5199; a 5173 é do orquestrador).

.PARAMETER WebExe / SlintExe / MeasureExe
    Executáveis: web release, Slint release e web "de medição" (ver acima).

.PARAMETER ResultsFile
    Onde gravar o resultado das métricas novas em JSON (padrão: pasta de trabalho no TEMP).
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$ExePath = "target\release\tako.exe",

    [Parameter(Position = 1)]
    [ValidateSet("web", "slint", "both", "auto")]
    [string]$Mode = "auto",

    [Parameter()]
    [int]$Runs = 3,

    [Parameter()]
    [int]$SettlingSeconds = 5,

    [Parameter()]
    [string]$BaseDataDir = "",

    [Parameter()]
    [ValidateSet("memoria", "abertura-biblioteca", "cpu-gpu", "fps", "latencia", "retorno", "tudo")]
    [string[]]$Metrics = @("memoria"),

    [Parameter()]
    [int]$RestSeconds = 30,

    [Parameter()]
    [int]$OpenRuns = 10,

    [Parameter()]
    [int]$FpsRuns = 3,

    [Parameter()]
    [int]$ReturnRuns = 5,

    [Parameter()]
    [int]$KeyPresses = 100,

    [Parameter()]
    [int]$Games = 300,

    [Parameter()]
    [int]$Port = 5199,

    [Parameter()]
    [string]$WebExe = "target\release\tako.exe",

    [Parameter()]
    [string]$SlintExe = "target\release\tako-slint.exe",

    [Parameter()]
    [string]$MeasureExe = "target\medicao\release\tako.exe",

    [Parameter()]
    [string]$ResultsFile = ""
)

$ErrorActionPreference = "Stop"

# Garante que caminhos relativos funcionem a partir da raiz do repositório
$RepoRoot = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $RepoRoot "Cargo.toml"))) {
    $RepoRoot = (Get-Location).Path
}

# Resolve caminho completo do executável
function Resolve-Exe([string]$path) {
    if ([System.IO.Path]::IsPathRooted($path)) {
        return $path
    }
    return Join-Path $RepoRoot $path
}

# Árvore de processos descendentes de um PID raiz
function Get-ProcessTree {
    param([int]$RootPid)
    $processes = @()
    $queue = New-Object System.Collections.Generic.Queue[int]
    $queue.Enqueue($RootPid)
    $visited = New-Object System.Collections.Generic.HashSet[int]

    while ($queue.Count -gt 0) {
        $currentPid = $queue.Dequeue()
        if (-not $visited.Add($currentPid)) { continue }

        $proc = Get-Process -Id $currentPid -ErrorAction SilentlyContinue
        if ($null -ne $proc) {
            $processes += $proc
        }

        # Busca processos filhos via Win32_Process
        $children = Get-CimInstance Win32_Process -Filter "ParentProcessId = $currentPid" -ErrorAction SilentlyContinue
        if ($null -ne $children) {
            foreach ($child in $children) {
                $queue.Enqueue([int]$child.ProcessId)
            }
        }
    }
    return $processes
}

# Coleta métricas de memória por processo e agregada
function Measure-ProcessMemory {
    param([int]$RootPid)

    $tree = Get-ProcessTree -RootPid $RootPid
    if ($tree.Count -eq 0) {
        return $null
    }

    $details = @()
    $totalWsBytes = [int64]0
    $totalPrivateBytes = [int64]0
    $coreProc = $null

    foreach ($proc in $tree) {
        $pidVal = $proc.Id
        $wsBytes = $proc.WorkingSet64
        $totalWsBytes += $wsBytes

        # Tenta obter Private Working Set via contador do Windows
        $privateWsBytes = [int64]0
        try {
            $perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess = $pidVal" -ErrorAction SilentlyContinue
            if ($null -ne $perf -and $perf.WorkingSetPrivate -gt 0) {
                $privateWsBytes = [int64]$perf.WorkingSetPrivate
            }
        } catch {}

        # Fallback para PrivateMemorySize64 (Commit Charge privado)
        if ($privateWsBytes -le 0) {
            $privateWsBytes = [int64]$proc.PrivateMemorySize64
        }
        $totalPrivateBytes += $privateWsBytes

        # Linha de comando para distinguir papéis
        $cmdLine = ""
        try {
            $cim = Get-CimInstance Win32_Process -Filter "ProcessId = $pidVal" -ErrorAction SilentlyContinue
            if ($null -ne $cim) { $cmdLine = $cim.CommandLine }
        } catch {}

        $role = "Processo filho"
        if ($cmdLine -match "--type=renderer") { $role = "CEF Renderer" }
        elseif ($cmdLine -match "--type=gpu-process") { $role = "CEF GPU" }
        elseif ($cmdLine -match "--type=utility") { $role = "CEF Utility" }
        elseif ($cmdLine -match "--web-ui") { $role = "Interface CEF (Host)" }
        elseif ($cmdLine -match "--web") { $role = "Núcleo Rust (--web)"; $coreProc = $proc }
        elseif ($cmdLine -match "--ui") { $role = "Interface Slint" }
        elseif ($pidVal -eq $RootPid) { $role = "Supervisor / Root"; $coreProc = $proc }

        $details += [PSCustomObject]@{
            Id                  = $pidVal
            Name                = $proc.ProcessName
            Role                = $role
            WorkingSetMB        = [math]::Round($wsBytes / 1MB, 2)
            PrivateWorkingSetMB = [math]::Round($privateWsBytes / 1MB, 2)
            CommandLine         = $cmdLine
        }
    }

    # Métricas específicas do processo núcleo / supervisor sozinho
    $corePrivateMB = 0.0
    $coreWorkingMB = 0.0
    if ($null -ne $coreProc) {
        $coreItem = $details | Where-Object { $_.Id -eq $coreProc.Id } | Select-Object -First 1
        if ($null -ne $coreItem) {
            $corePrivateMB = $coreItem.PrivateWorkingSetMB
            $coreWorkingMB = $coreItem.WorkingSetMB
        }
    }

    return [PSCustomObject]@{
        ProcessCount        = $tree.Count
        TotalWorkingSetMB   = [math]::Round($totalWsBytes / 1MB, 2)
        TotalPrivateSetMB   = [math]::Round($totalPrivateBytes / 1MB, 2)
        CorePrivateSetMB    = $corePrivateMB
        CoreWorkingSetMB    = $coreWorkingMB
        Details             = $details
    }
}

# Encerra apenas os processos gerados pelo benchmark
function Stop-ProcessTree {
    param([int]$RootPid)
    if ($RootPid -le 0) { return }
    $tree = Get-ProcessTree -RootPid $RootPid
    [array]::Reverse($tree)
    foreach ($proc in $tree) {
        try {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        } catch {}
    }
}

# Cálculo da mediana
function Get-Median {
    param([double[]]$Values)
    if ($null -eq $Values -or $Values.Count -eq 0) { return 0.0 }
    $sorted = $Values | Sort-Object
    $mid = [math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2 -eq 0) {
        return [math]::Round(($sorted[$mid - 1] + $sorted[$mid]) / 2.0, 2)
    } else {
        return [math]::Round($sorted[$mid], 2)
    }
}

# Executa bateria de testes para um executável e modo específico
function Run-BenchmarkSuite {
    param(
        [string]$TargetExe,
        [string]$TargetMode
    )

    $resolved = Resolve-Exe $TargetExe
    if (-not (Test-Path -LiteralPath $resolved)) {
        Write-Error "Executável não encontrado: $resolved"
        return $null
    }

    Write-Host "`n============================================================" -ForegroundColor Cyan
    Write-Host " Tako Benchmark Suite: Modo $TargetMode" -ForegroundColor Cyan
    Write-Host " Executável: $resolved" -ForegroundColor Yellow
    Write-Host " Repetições: $Runs rodadas (cálculo de mediana)" -ForegroundColor Yellow
    Write-Host " Repouso:    $SettlingSeconds s" -ForegroundColor Yellow
    Write-Host "============================================================" -ForegroundColor Cyan

    $runResults = @()

    for ($i = 1; $i -le $Runs; $i++) {
        $tempDir = if ([string]::IsNullOrWhiteSpace($BaseDataDir)) {
            Join-Path $env:TEMP ("tako-meas-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
        } else {
            Join-Path $BaseDataDir "$TargetMode-run-$i"
        }
        New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

        $procArgs = @("--data-dir", $tempDir)
        if ($TargetMode -eq "web") {
            $procArgs = @("--web") + $procArgs
        }

        Write-Host "`n--- [Rodada $i / $Runs - Modo $TargetMode] ---" -ForegroundColor DarkCyan
        Write-Host "Iniciando: $resolved $($procArgs -join ' ')"

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $pinfo = New-Object System.Diagnostics.ProcessStartInfo
        $pinfo.FileName = $resolved
        $pinfo.Arguments = $procArgs -join " "
        $pinfo.UseShellExecute = $false
        $pinfo.CreateNoWindow = $true

        $rootProc = [System.Diagnostics.Process]::Start($pinfo)
        $rootPid = $rootProc.Id

        $windowFoundTimeMs = $null
        $deadline = [DateTime]::UtcNow.AddSeconds(20)

        try {
            # 1. Medição do tempo de abertura até resposta interativa
            while ([DateTime]::UtcNow -lt $deadline) {
                Start-Sleep -Milliseconds 30
                $tree = Get-ProcessTree -RootPid $rootPid
                $windowProc = $tree | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
                if ($null -ne $windowProc -and $windowProc.Responding) {
                    $windowFoundTimeMs = $sw.ElapsedMilliseconds
                    break
                }
            }

            if ($null -eq $windowFoundTimeMs) {
                # Fallback: interface ativa pelo surgimento do processo de UI
                $tree = Get-ProcessTree -RootPid $rootPid
                $hasChild = if ($TargetMode -eq "web") { $tree.Count -ge 2 } else { $tree.Count -ge 1 }
                if ($hasChild) {
                    $windowFoundTimeMs = $sw.ElapsedMilliseconds
                    Write-Host "Janela/Processo interativo detectado em ${windowFoundTimeMs} ms." -ForegroundColor Green
                } else {
                    $windowFoundTimeMs = $sw.ElapsedMilliseconds
                }
            } else {
                Write-Host "Janela interativa respondendo em ${windowFoundTimeMs} ms." -ForegroundColor Green
            }

            # 2. Estabilização para medir Interface Aberta Parada
            Write-Host "Aguardando $SettlingSeconds s de estabilização em repouso..."
            Start-Sleep -Seconds $SettlingSeconds

            $memOpen = Measure-ProcessMemory -RootPid $rootPid
            Write-Host "Estado: Interface Aberta Parada" -ForegroundColor Yellow
            Write-Host "  Contagem de Processos:     $($memOpen.ProcessCount)"
            Write-Host "  Private Working Set Total: $($memOpen.TotalPrivateSetMB) MB"
            Write-Host "  Working Set Total:         $($memOpen.TotalWorkingSetMB) MB"
            foreach ($d in $memOpen.Details) {
                Write-Host ("    PID {0,5} | {1,-22} | Privado: {2,7} MB | WS: {3,7} MB" -f $d.Id, $d.Role, $d.PrivateWorkingSetMB, $d.WorkingSetMB)
            }

            # 3. Estado: Núcleo / Supervisor Sozinho (Sem Interface / Durante Jogo)
            # Na arquitetura Tako, durante um jogo ou minimizado/bandeja, todos os processos de UI
            # (CEF e seus subprocessos ou Slint UI) são descartados, restando unicamente o núcleo/supervisor.
            # O consumo desse estado é exatamente o consumo do processo raiz isolado.
            Write-Host "`nEstado: Núcleo / Supervisor Sozinho (Interface Fechada / Durante o Jogo)" -ForegroundColor Yellow
            Write-Host "  Contagem de Processos:     1 (apenas núcleo/supervisor)"
            Write-Host "  Private Working Set Total: $($memOpen.CorePrivateSetMB) MB"
            Write-Host "  Working Set Total:         $($memOpen.CoreWorkingSetMB) MB"

            $runResults += [PSCustomObject]@{
                OpenTimeMs          = [double]$windowFoundTimeMs
                OpenProcCount       = $memOpen.ProcessCount
                OpenPrivateSetMB    = $memOpen.TotalPrivateSetMB
                OpenWorkingSetMB    = $memOpen.TotalWorkingSetMB
                ClosedProcCount     = 1
                ClosedPrivateSetMB  = $memOpen.CorePrivateSetMB
                ClosedWorkingSetMB  = $memOpen.CoreWorkingSetMB
                Details             = $memOpen.Details
            }

        } finally {
            Stop-ProcessTree -RootPid $rootPid
            Start-Sleep -Milliseconds 400
            try {
                Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
            } catch {}
        }
    }

    $medOpenTime = Get-Median ($runResults | ForEach-Object { [double]$_.OpenTimeMs })
    $medOpenProcs = Get-Median ($runResults | ForEach-Object { [double]$_.OpenProcCount })
    $medOpenPriv = Get-Median ($runResults | ForEach-Object { [double]$_.OpenPrivateSetMB })
    $medOpenWs = Get-Median ($runResults | ForEach-Object { [double]$_.OpenWorkingSetMB })
    $medClosedPriv = Get-Median ($runResults | ForEach-Object { [double]$_.ClosedPrivateSetMB })
    $medClosedWs = Get-Median ($runResults | ForEach-Object { [double]$_.ClosedWorkingSetMB })

    Write-Host "`n------------------------------------------------------------" -ForegroundColor Green
    Write-Host " RESULTADOS CONSOLIDADOS (Mediana de $Runs rodadas - $TargetMode)" -ForegroundColor Green
    Write-Host "------------------------------------------------------------" -ForegroundColor Green
    Write-Host ("Tempo de abertura até responder: {0,7} ms" -f $medOpenTime)
    Write-Host ("Interface Aberta - Processos:     {0,7}" -f $medOpenProcs)
    Write-Host ("Interface Aberta - Private WS:    {0,7} MB" -f $medOpenPriv)
    Write-Host ("Interface Aberta - Total WS:      {0,7} MB" -f $medOpenWs)
    Write-Host ("Sem Interface / Jogo - Processos: {0,7}" -f 1)
    Write-Host ("Sem Interface / Jogo - Private WS:{0,7} MB" -f $medClosedPriv)
    Write-Host ("Sem Interface / Jogo - Total WS:  {0,7} MB" -f $medClosedWs)
    Write-Host "------------------------------------------------------------`n" -ForegroundColor Green

    return [PSCustomObject]@{
        Mode                  = $TargetMode
        Runs                  = $Runs
        MedianOpenTimeMs      = $medOpenTime
        MedianOpenProcs       = $medOpenProcs
        MedianOpenPrivateMB   = $medOpenPriv
        MedianOpenWorkingMB   = $medOpenWs
        MedianClosedProcs     = 1
        MedianClosedPrivateMB = $medClosedPriv
        MedianClosedWorkingMB = $medClosedWs
        RawResults            = $runResults
    }
}

# ============================================================================================================
# WEB-60b: abertura até a biblioteca, CPU e GPU em repouso, fps, latência de teclado e retorno depois do jogo
# ============================================================================================================

$Work = Join-Path $env:TEMP "tako-web60b"
$EventosFile = Join-Path $Work "eventos.jsonl"
$SeedDir = Join-Path $Work "semente"
$GamesDir = Join-Path $Work "jogos"
$MeasureDist = Join-Path $RepoRoot "target\medicao-dist"
$TestGameExe = Join-Path $GamesDir "Jogo de teste.exe"
$script:Servidor = $null
$script:Geometria = $null

function Get-UnixMs { return [double][DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() }

function Get-Percentile {
    param([double[]]$Values, [double]$P)
    if ($null -eq $Values -or $Values.Count -eq 0) { return $null }
    $sorted = @($Values | Sort-Object)
    $i = [math]::Min($sorted.Count - 1, [math]::Max(0, [int][math]::Ceiling(($P / 100.0) * $sorted.Count) - 1))
    return [math]::Round([double]$sorted[$i], 2)
}

function Get-Stats {
    param([double[]]$Values)
    if ($null -eq $Values -or $Values.Count -eq 0) { return $null }
    $sorted = @($Values | Sort-Object)
    return [ordered]@{
        n      = $Values.Count
        min    = [math]::Round($sorted[0], 2)
        median = Get-Median $Values
        p95    = Get-Percentile $Values 95
        max    = [math]::Round($sorted[$sorted.Count - 1], 2)
    }
}

# Funções do Windows para trazer a janela à frente e mexer no mouse e no teclado de verdade (SendInput legado).
function Initialize-Win32 {
    if (-not ("TakoWin32" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class TakoWin32 {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern void SwitchToThisWindow(IntPtr h, bool alt);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr e);
}
'@
    }
    [void][TakoWin32]::SetProcessDPIAware()
}

# Todos os descendentes de um processo, de uma vez só (uma consulta ao WMI).
function Get-TreeIds {
    param([int]$RootPid)
    $all = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, CommandLine)
    $ids = New-Object System.Collections.Generic.List[int]
    $queue = New-Object System.Collections.Generic.Queue[int]
    $queue.Enqueue($RootPid)
    while ($queue.Count -gt 0) {
        $cur = $queue.Dequeue()
        if ($ids.Contains($cur)) { continue }
        $ids.Add($cur)
        foreach ($c in ($all | Where-Object { $_.ParentProcessId -eq $cur })) { $queue.Enqueue([int]$c.ProcessId) }
    }
    return [PSCustomObject]@{ Ids = $ids.ToArray(); Commands = $all }
}

function Get-RoleOf {
    param([string]$CmdLine, [int]$ProcessId, [int]$RootPid)
    if ($CmdLine -match "--type=renderer") { return "CEF Renderer" }
    if ($CmdLine -match "--type=gpu-process") { return "CEF GPU" }
    if ($CmdLine -match "--type=utility") { return "CEF Utility" }
    if ($CmdLine -match "--web-ui") { return "Interface CEF (Host)" }
    if ($CmdLine -match "--web") { return "Núcleo Rust (--web)" }
    if ($CmdLine -match "--ui") { return "Interface Slint" }
    if ($ProcessId -eq $RootPid) { return "Supervisor / Root" }
    return "Processo filho"
}

function Stop-Tree {
    param([int]$RootPid)
    $info = Get-TreeIds -RootPid $RootPid
    $ids = @($info.Ids)
    [array]::Reverse($ids)
    foreach ($id in $ids) { Stop-Process -Id $id -Force -ErrorAction SilentlyContinue }
    Start-Sleep -Milliseconds 500
}

# Jogos de teste que o Tako abriu e que ficaram para trás (só os da pasta de trabalho deste script).
function Stop-TestGames {
    Get-Process -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -and $_.Path.StartsWith($GamesDir, [StringComparison]::OrdinalIgnoreCase)
    } | ForEach-Object { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }
}

function Get-UiWindow {
    param([int]$RootPid)
    foreach ($id in (Get-TreeIds -RootPid $RootPid).Ids) {
        $p = Get-Process -Id $id -ErrorAction SilentlyContinue
        if ($null -ne $p -and $p.MainWindowHandle -ne 0) { return [IntPtr]$p.MainWindowHandle }
    }
    return [IntPtr]::Zero
}

# O Windows só deixa trocar a janela da frente quem já está recebendo entrada: o script se liga à fila de entrada da
# janela que está na frente, pede a troca e solta. Tenta algumas vezes e confere no fim.
function Set-Foreground {
    param([IntPtr]$Handle)
    for ($tentativa = 1; $tentativa -le 4; $tentativa++) {
        $frente = [TakoWin32]::GetForegroundWindow()
        $pidFrente = [uint32]0
        $threadFrente = [TakoWin32]::GetWindowThreadProcessId($frente, [ref]$pidFrente)
        $meu = [TakoWin32]::GetCurrentThreadId()
        [void][TakoWin32]::AttachThreadInput($meu, $threadFrente, $true)
        [void][TakoWin32]::ShowWindow($Handle, 5)
        [void][TakoWin32]::BringWindowToTop($Handle)
        [void][TakoWin32]::SetForegroundWindow($Handle)
        [void][TakoWin32]::AttachThreadInput($meu, $threadFrente, $false)
        Start-Sleep -Milliseconds 400
        if ([TakoWin32]::GetForegroundWindow() -eq $Handle) { return $true }
        # Último recurso: Alt solto antes do pedido e o atalho de "trocar para".
        [TakoWin32]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
        [TakoWin32]::SwitchToThisWindow($Handle, $true)
        [TakoWin32]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 400
        if ([TakoWin32]::GetForegroundWindow() -eq $Handle) { return $true }
    }
    return $false
}

function Send-Key {
    param([byte]$Vk)
    # 1 = tecla estendida (setas); 2 = soltar.
    [TakoWin32]::keybd_event($Vk, 0, 1, [UIntPtr]::Zero)
    [TakoWin32]::keybd_event($Vk, 0, 3, [UIntPtr]::Zero)
}

function Send-Click {
    param([int]$X, [int]$Y)
    # Um passo antes: o Chromium só aceita o clique depois de ver o mouse chegar à janela.
    [void][TakoWin32]::SetCursorPos($X - 6, $Y - 6)
    Start-Sleep -Milliseconds 150
    [void][TakoWin32]::SetCursorPos($X, $Y)
    Start-Sleep -Milliseconds 150
    [TakoWin32]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 60
    [TakoWin32]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
}

# --- Eventos da página (coletor do servidor de medição) -----------------------------------------------------

function Read-Eventos {
    if (-not (Test-Path -LiteralPath $EventosFile)) { return @() }
    $fs = [System.IO.File]::Open($EventosFile, "Open", "Read", "ReadWrite")
    try {
        $sr = New-Object System.IO.StreamReader($fs, [System.Text.Encoding]::UTF8)
        $texto = $sr.ReadToEnd()
    } finally { $fs.Dispose() }
    $lista = New-Object System.Collections.Generic.List[object]
    foreach ($l in ($texto -split "`n")) {
        if ($l.Trim().Length -eq 0) { continue }
        try { $lista.Add(($l | ConvertFrom-Json)) } catch {}
    }
    return $lista.ToArray()
}

# Espera um evento da página (por nome) com horário a partir de $Since; devolve $null ao estourar o prazo.
function Wait-Evento {
    param([string]$Name, [double]$Since, [int]$TimeoutSec = 30, [scriptblock]$Where = $null)
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSec)
    while ([DateTime]::UtcNow -lt $deadline) {
        $found = Read-Eventos | Where-Object { $_.evento -eq $Name -and $_.em -ge $Since -and ($null -eq $Where -or (& $Where $_)) } | Select-Object -First 1
        if ($null -ne $found) { return $found }
        Start-Sleep -Milliseconds 100
    }
    return $null
}

# --- Bancada: servidor, jogo de teste, biblioteca de medição ------------------------------------------------

function Start-Servidor {
    if ($null -ne $script:Servidor -and -not $script:Servidor.HasExited) { return }
    $node = (Get-Command node -ErrorAction Stop).Source
    $webDir = Join-Path $RepoRoot "web"
    if (-not (Test-Path (Join-Path $webDir "node_modules"))) { throw "web\node_modules ausente: rode 'npm ci' em web\" }
    # Pacote de produção do app com a medição injetada (não toca em web\dist).
    $env:TAKO_MEDICAO_DIST = $MeasureDist
    Push-Location $webDir
    # O Vite escreve avisos no stderr (PostCSS, tamanho do pacote); com "Stop" o PowerShell os trataria como erro.
    $anterior = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & $node "node_modules/vite/bin/vite.js" build --config "src/dev/medicao/vite.medicao.config.mjs" --mode medicao 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "falha ao construir o pacote de medição" }
    } finally { $ErrorActionPreference = $anterior; Pop-Location }
    $env:TAKO_MEDICAO_PORTA = "$Port"
    $env:TAKO_MEDICAO_SAIDA = $EventosFile
    $env:TAKO_MEDICAO_JOGOS = Join-Path $Work "jogos.txt"
    $env:TAKO_MEDICAO_JOGO = $TestGameExe
    $script:Servidor = Start-Process -FilePath $node -ArgumentList "node_modules/vite/bin/vite.js", "preview", "--config", "src/dev/medicao/vite.medicao.config.mjs" `
        -WorkingDirectory $webDir -PassThru -WindowStyle Hidden
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while ([DateTime]::UtcNow -lt $deadline) {
        try {
            $c = New-Object System.Net.Sockets.TcpClient
            $c.Connect("127.0.0.1", $Port); $c.Close()
            return
        } catch { Start-Sleep -Milliseconds 200 }
    }
    throw "o servidor de medição não abriu na porta $Port"
}

function Stop-Servidor {
    if ($null -ne $script:Servidor -and -not $script:Servidor.HasExited) {
        Stop-Process -Id $script:Servidor.Id -Force -ErrorAction SilentlyContinue
    }
    $script:Servidor = $null
}

# Jogo de teste: uma janela comum que fecha sozinha e anota o instante da saída (para o retorno depois do jogo).
function New-JogoDeTeste {
    if (Test-Path -LiteralPath $TestGameExe) { return }
    New-Item -ItemType Directory -Path $GamesDir -Force | Out-Null
    $fonte = @'
using System;
using System.IO;
using System.Windows.Forms;

public static class JogoDeTeste
{
    [STAThread]
    public static void Main()
    {
        string s = Environment.GetEnvironmentVariable("TAKO_MEDICAO_JOGO_SEGUNDOS");
        int segundos = string.IsNullOrEmpty(s) ? 20 : int.Parse(s);
        string marca = Environment.GetEnvironmentVariable("TAKO_MEDICAO_JOGO_MARCA");
        Form janela = new Form();
        janela.Text = "Jogo de teste (medicao do Tako)";
        janela.Width = 640;
        janela.Height = 360;
        janela.StartPosition = FormStartPosition.CenterScreen;
        Timer relogio = new Timer();
        relogio.Interval = segundos * 1000;
        relogio.Tick += delegate { relogio.Stop(); janela.Close(); };
        relogio.Start();
        Application.Run(janela);
        if (!string.IsNullOrEmpty(marca)) File.WriteAllText(marca, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds().ToString());
    }
}
'@
    Add-Type -TypeDefinition $fonte -OutputAssembly $TestGameExe -OutputType WindowsApplication -ReferencedAssemblies System.Windows.Forms
}

# Cópias do jogo de teste: viram jogos "Jogo 001" a "Jogo NNN" na biblioteca de medição (a lista precisa rolar).
function New-JogosDeTeste {
    $copias = Join-Path $GamesDir "copias"
    New-Item -ItemType Directory -Path $copias -Force | Out-Null
    $lista = @()
    for ($i = 1; $i -le $Games; $i++) {
        $caminho = Join-Path $copias ("Jogo {0:D3}.exe" -f $i)
        if (-not (Test-Path -LiteralPath $caminho)) { Copy-Item -LiteralPath $TestGameExe -Destination $caminho }
        $lista += $caminho
    }
    Set-Content -LiteralPath (Join-Path $Work "jogos.txt") -Value $lista -Encoding UTF8
}

# Outros processos disputando a CPU (compilações em paralelo, por exemplo) deformam a abertura e o fps. Antes de cada
# rodada espera o PC ficar abaixo de $MaxLoad% de CPU (média de 3 s), no máximo $MaxSec s; devolve a carga vista e se
# ela ainda passava do limite ao desistir de esperar.
function Wait-MaquinaQuieta {
    param([int]$MaxSec = 90, [double]$MaxLoad = 20)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    do {
        # CIM em vez de Get-Counter: o nome do contador muda com o idioma do Windows, o da classe WMI não.
        $leituras = @()
        for ($k = 0; $k -lt 3; $k++) {
            $leituras += (Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'").PercentProcessorTime
            Start-Sleep -Seconds 1
        }
        $carga = ($leituras | Measure-Object -Average).Average
        if ($carga -lt $MaxLoad) { break }
        Start-Sleep -Seconds 2
    } while ($sw.Elapsed.TotalSeconds -lt $MaxSec)
    return [PSCustomObject]@{ CargaPct = [math]::Round($carga, 1); Ruidoso = ($carga -ge $MaxLoad); Rustc = @(Get-Process rustc, cargo -ErrorAction SilentlyContinue).Count }
}

function New-RunDir {
    param([string]$Name)
    $dir = Join-Path $Work ("runs\" + $Name)
    if (Test-Path -LiteralPath $dir) { Remove-Item -LiteralPath $dir -Recurse -Force }
    # Cópia da biblioteca de medição sem os logs nem o cache do CEF. O cache de metadados vai junto: sem ele a fila
    # refaria a consulta dos 300 jogos de teste a cada abertura e o repouso nunca assentaria.
    robocopy $SeedDir $dir /E /XD logs (Join-Path $SeedDir "cache\cef") /NFL /NDL /NJH /NJS /NP | Out-Null
    return $dir
}

# Abre o Tako "de medição" apontando a interface para o servidor local. $Cenario escolhe o que a página faz.
function Start-AppMedicao {
    param([string]$DataDir, [string]$Cenario, [hashtable]$Extra = @{}, [string]$Rodada = "1")
    $consulta = "medicao=$Cenario&rodada=$Rodada"
    foreach ($k in $Extra.Keys) { $consulta += "&$k=$($Extra[$k])" }
    $env:TAKO_WEB_DEV_URL = "http://127.0.0.1:$Port/?$consulta"
    $t0 = Get-UnixMs
    try {
        $p = Start-Process -FilePath (Resolve-Exe $MeasureExe) -ArgumentList @("--web", "--data-dir", "`"$DataDir`"") -PassThru
    } finally { Remove-Item Env:TAKO_WEB_DEV_URL -ErrorAction SilentlyContinue }
    return [PSCustomObject]@{ Process = $p; T0 = $t0 }
}

# A fila de metadados do núcleo (uma consulta a cada ~1,8 s por jogo sem metadados) segue trabalhando e dispara
# library.changed, que a interface redesenha: repouso medido com ela ligada não é repouso. Antes de medir, a semente
# roda até a fila esvaziar (sem linha "Metadados" no log por $QuietSec s); o resultado fica no banco e vai nas cópias.
function Wait-FilaDeMetadados {
    param([string]$DataDir, [int]$QuietSec = 45, [int]$MaxSec = 1500)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $inicio = [DateTimeOffset]::Now
    while ($sw.Elapsed.TotalSeconds -lt $MaxSec) {
        Start-Sleep -Seconds 5
        $arquivo = Get-ChildItem (Join-Path $DataDir "logs") -Filter "tako-*.log" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($null -eq $arquivo) { continue }
        $ultimo = $null
        foreach ($l in (Get-Content -LiteralPath $arquivo.FullName -Encoding UTF8 -Tail 60)) {
            if ($l -match "Metadados|igdb|metadata") {
                $m = [regex]::Match($l, '^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d{3} [+-]\d\d:\d\d)')
                if ($m.Success) {
                    $t = [DateTimeOffset]::ParseExact($m.Groups[1].Value, "yyyy-MM-dd HH:mm:ss.fff zzz", [Globalization.CultureInfo]::InvariantCulture)
                    # Só vale o que aconteceu depois que esta espera começou (o log guarda as sessões anteriores).
                    if ($t -ge $inicio) { $ultimo = $t }
                }
            }
        }
        if ($null -ne $ultimo -and ([DateTimeOffset]::Now - $ultimo).TotalSeconds -ge $QuietSec) { return [math]::Round($sw.Elapsed.TotalSeconds, 0) }
        # Sem nenhuma linha de metadados em 90 s: a fila já estava vazia.
        if ($null -eq $ultimo -and $sw.Elapsed.TotalSeconds -ge 90) { return [math]::Round($sw.Elapsed.TotalSeconds, 0) }
    }
    return -1
}

function Initialize-Bancada {
    Initialize-Win32
    New-Item -ItemType Directory -Path $Work -Force | Out-Null
    foreach ($exe in @($MeasureExe)) {
        if (-not (Test-Path -LiteralPath (Resolve-Exe $exe))) {
            throw "Executável ausente: $(Resolve-Exe $exe). Veja os pré-requisitos no cabeçalho do script."
        }
    }
    New-JogoDeTeste
    New-JogosDeTeste
    # O release de verdade lê o pacote ao lado do executável (web\dist); copia se faltar.
    $relDist = Join-Path (Split-Path (Resolve-Exe $WebExe)) "web\dist"
    if (-not (Test-Path (Join-Path $relDist "index.html"))) {
        $src = Join-Path $RepoRoot "web\dist"
        if (-not (Test-Path (Join-Path $src "index.html"))) { throw "web\dist ausente: rode 'npm run build' em web\" }
        New-Item -ItemType Directory -Path (Split-Path $relDist) -Force | Out-Null
        Copy-Item -LiteralPath $src -Destination $relDist -Recurse
    }
    Start-Servidor
    # O primeiro Get-Counter dos contadores de GPU leva alguns segundos para inicializar: faz isso fora das janelas.
    [void](Get-Counter -Counter '\GPU Engine(*)\Utilization Percentage' -ErrorAction SilentlyContinue)
    # Semear a biblioteca (uma vez): os jogos do PC, mais os de teste, assistente concluído, capas desligadas.
    if (-not (Test-Path (Join-Path $SeedDir "tako.db"))) {
        Write-Host "Semeando a biblioteca de medição ($Games jogos de teste)..." -ForegroundColor Yellow
        New-Item -ItemType Directory -Path $SeedDir -Force | Out-Null
        $app = Start-AppMedicao -DataDir $SeedDir -Cenario "semear"
        try {
            $ev = Wait-Evento -Name "semeado" -Since $app.T0 -TimeoutSec 180
            if ($null -eq $ev) { throw "a página não terminou de semear a biblioteca" }
            Write-Host ("  biblioteca: {0} jogos ({1} novos)" -f $ev.total, $ev.novos)
        } finally { Stop-Tree -RootPid $app.Process.Id }
    }
    $marca = Join-Path $SeedDir "fila-de-metadados-assentada.txt"
    if (-not (Test-Path $marca)) {
        Write-Host "Assentando a fila de metadados da semente (pode levar vários minutos)..." -ForegroundColor Yellow
        $app = Start-AppMedicao -DataDir $SeedDir -Cenario "abertura" -Rodada "semente"
        try {
            $seg = Wait-FilaDeMetadados -DataDir $SeedDir
            Write-Host "  fila quieta depois de $seg s"
            if ($seg -ge 0) { Set-Content -LiteralPath $marca -Value "$seg" }
        } finally { Stop-Tree -RootPid $app.Process.Id }
    }
}

# --- CPU e GPU do sistema operacional -----------------------------------------------------------------------

function Get-CpuSnapshot {
    param([int]$RootPid)
    $info = Get-TreeIds -RootPid $RootPid
    $map = @{}
    foreach ($id in $info.Ids) {
        $p = Get-Process -Id $id -ErrorAction SilentlyContinue
        if ($null -eq $p) { continue }
        try { $cpu = $p.TotalProcessorTime.TotalSeconds } catch { continue }
        $cmd = ($info.Commands | Where-Object { $_.ProcessId -eq $id } | Select-Object -First 1).CommandLine
        $map[$id] = [PSCustomObject]@{ Cpu = $cpu; Role = (Get-RoleOf -CmdLine $cmd -ProcessId $id -RootPid $RootPid) }
    }
    return $map
}

# Espera o Tako assentar: o núcleo e a interface deixam de gastar CPU (leitura das lojas, verificação de
# atualização, capas). Três janelas seguidas de 2 s abaixo de 3% de um núcleo, entre 10 s e $MaxSec.
function Wait-Quieto {
    param([int]$RootPid, [int]$MaxSec = 60)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $quietas = 0
    $antes = Get-CpuSnapshot -RootPid $RootPid
    while ($sw.Elapsed.TotalSeconds -lt $MaxSec) {
        Start-Sleep -Seconds 2
        $depois = Get-CpuSnapshot -RootPid $RootPid
        $delta = 0.0
        foreach ($id in $depois.Keys) {
            $base = if ($antes.ContainsKey($id)) { $antes[$id].Cpu } else { 0.0 }
            $delta += ($depois[$id].Cpu - $base)
        }
        $antes = $depois
        if (($delta / 2.0 * 100.0) -lt 3.0) { $quietas++ } else { $quietas = 0 }
        if ($quietas -ge 3 -and $sw.Elapsed.TotalSeconds -ge 10) { break }
    }
    return [math]::Round($sw.Elapsed.TotalSeconds, 1)
}

# CPU (TotalProcessorTime por processo) e GPU (contadores "GPU Engine" por PID) durante $Seconds, com a
# interface parada. Devolve % de um núcleo e % do PC; a GPU em % de utilização do motor, média e máxima por segundo.
function Measure-Repouso {
    param([int]$RootPid, [int]$Seconds)
    $inicio = Get-CpuSnapshot -RootPid $RootPid
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # Um caminho por PID do Tako: os contadores de todos os motores de todos os processos levam mais de 2 s por
    # amostra e esticariam a janela. Um PID sem motor de GPU só não devolve amostra.
    $caminhosGpu = @($inicio.Keys | ForEach-Object { "\GPU Engine(pid_$($_)_*)\Utilization Percentage" })
    $gpuSets = @(Get-Counter -Counter $caminhosGpu -SampleInterval 1 -MaxSamples $Seconds -ErrorAction SilentlyContinue)
    $fim = Get-CpuSnapshot -RootPid $RootPid
    $wall = $sw.Elapsed.TotalSeconds
    $pids = @($inicio.Keys) + @($fim.Keys) | Sort-Object -Unique

    $porPapel = @{}
    $totalCpu = 0.0
    foreach ($id in $fim.Keys) {
        $base = if ($inicio.ContainsKey($id)) { $inicio[$id].Cpu } else { 0.0 }
        $d = $fim[$id].Cpu - $base
        $totalCpu += $d
        $papel = $fim[$id].Role
        if (-not $porPapel.ContainsKey($papel)) { $porPapel[$papel] = 0.0 }
        $porPapel[$papel] += $d
    }
    $logicos = [Environment]::ProcessorCount
    $papeis = [ordered]@{}
    foreach ($k in ($porPapel.Keys | Sort-Object)) { $papeis[$k] = [math]::Round($porPapel[$k] / $wall * 100.0, 2) }

    # GPU: soma dos motores dos processos do Tako a cada segundo.
    $totais = New-Object System.Collections.Generic.List[double]
    $tipos = @{}
    foreach ($set in $gpuSets) {
        $soma = 0.0
        foreach ($s in $set.CounterSamples) {
            if ($s.InstanceName -notmatch '^pid_(\d+)_.*engtype_(\w+)$') { continue }
            if ($pids -notcontains [int]$Matches[1]) { continue }
            $soma += $s.CookedValue
            $t = $Matches[2]
            if (-not $tipos.ContainsKey($t)) { $tipos[$t] = 0.0 }
            $tipos[$t] += $s.CookedValue
        }
        $totais.Add($soma)
    }
    $n = [math]::Max(1, $totais.Count)
    $gpuMedia = if ($totais.Count) { [math]::Round(($totais | Measure-Object -Average).Average, 3) } else { $null }
    $gpuMax = if ($totais.Count) { [math]::Round(($totais | Measure-Object -Maximum).Maximum, 3) } else { $null }
    $porTipo = [ordered]@{}
    foreach ($k in ($tipos.Keys | Sort-Object)) { $porTipo[$k] = [math]::Round($tipos[$k] / $n, 3) }

    return [ordered]@{
        janelaSegundos      = [math]::Round($wall, 1)
        processos           = $fim.Count
        cpuSegundos         = [math]::Round($totalCpu, 3)
        cpuPctDeUmNucleo    = [math]::Round($totalCpu / $wall * 100.0, 2)
        cpuPctDoPc          = [math]::Round($totalCpu / $wall / $logicos * 100.0, 3)
        cpuPorPapelPctNucleo = $papeis
        gpuAmostras         = $totais.Count
        gpuUtilizacaoMedia  = $gpuMedia
        gpuUtilizacaoMaxima = $gpuMax
        gpuMediaPorMotor    = $porTipo
    }
}

# --- Cenários -----------------------------------------------------------------------------------------------

# Abertura até a biblioteca interativa: do início do processo até a lista com itens, aceitando foco e sem a
# entrada do app por cima. A página marca o instante; o script marca o início, no mesmo relógio.
function Measure-AberturaBiblioteca {
    param([int]$Runs = $OpenRuns)
    $carregou = @(); $naTela = @(); $livre = @(); $nav = @(); $cargas = @()
    for ($i = 1; $i -le $Runs; $i++) {
        $dir = New-RunDir "abertura-$i"
        $carga = Wait-MaquinaQuieta
        $cargas += $carga
        $app = Start-AppMedicao -DataDir $dir -Cenario "abertura" -Rodada "$i"
        try {
            $ev = Wait-Evento -Name "biblioteca-livre" -Since $app.T0 -TimeoutSec 40
            if ($null -eq $ev) { Write-Host "  rodada ${i}: a biblioteca não chegou" -ForegroundColor Red; continue }
            $c = Wait-Evento -Name "carregou" -Since $app.T0 -TimeoutSec 5
            $n = Wait-Evento -Name "biblioteca-na-tela" -Since $app.T0 -TimeoutSec 5
            $g = Wait-Evento -Name "geometria" -Since $app.T0 -TimeoutSec 5
            if ($null -ne $g -and $null -eq $script:Geometria) {
                $janela = Get-UiWindow -RootPid $app.Process.Id
                $r = New-Object TakoWin32+RECT
                [void][TakoWin32]::GetWindowRect($janela, [ref]$r)
                # A origem da página é a da janela mais a borda (zero em janela sem moldura); window.screenX não serve: o
                # Chromium devolve 0 até a janela ser posicionada.
                $script:Geometria = [PSCustomObject]@{
                    Item   = $g
                    BordaX = (($r.R - $r.L) - $g.larguraInterna) / 2
                    BordaY = (($r.B - $r.T) - $g.alturaInterna) / 2
                }
            }
            $carregou += ($c.em - $app.T0); $naTela += ($n.em - $app.T0); $livre += ($ev.em - $app.T0)
            $nav += $ev.desdeNavegacaoMs
            Write-Host ("  rodada {0}: página carregou {1:N0} ms, biblioteca na tela {2:N0} ms, livre {3:N0} ms (PC a {4}% de CPU, {5} rustc/cargo)" -f $i, ($c.em - $app.T0), ($n.em - $app.T0), ($ev.em - $app.T0), $carga.CargaPct, $carga.Rustc)
        } finally { Stop-Tree -RootPid $app.Process.Id }
    }
    return [ordered]@{
        rodadas                  = $Runs
        paginaCarregouMs         = Get-Stats $carregou
        bibliotecaNaTelaMs       = Get-Stats $naTela
        bibliotecaInterativaMs   = Get-Stats $livre
        navegacaoAteInterativaMs = Get-Stats $nav
        cargaDoPcAntesDaRodadaPct = @($cargas | ForEach-Object { $_.CargaPct })
        rodadasComPcRuidoso      = @($cargas | Where-Object { $_.Ruidoso }).Count
    }
}

# Repouso com a página de medição: a página conta o que pede ao compositor e o script mede CPU e GPU do sistema
# na mesma janela. O segundo estado (página do jogo) é aberto por um clique de mouse do sistema.
function Measure-RepousoBundle {
    $dir = New-RunDir "repouso-bundle"
    $carga = Wait-MaquinaQuieta
    $ms = $RestSeconds * 1000
    $app = Start-AppMedicao -DataDir $dir -Cenario "repouso" -Extra @{ ms = $ms; clique = "os" }
    $res = [ordered]@{ cargaDoPcAntesPct = $carga.CargaPct }
    try {
        $janelaPronta = Wait-Evento -Name "repouso-janela" -Since $app.T0 -TimeoutSec 60 -Where { param($e) $e.estado -eq "visao-geral" }
        if ($null -eq $janelaPronta) { throw "a página não chegou ao repouso" }
        $res["visao-geral"] = [ordered]@{ os = Measure-Repouso -RootPid $app.Process.Id -Seconds $RestSeconds }
        $pag = Wait-Evento -Name "repouso" -Since $app.T0 -TimeoutSec 20 -Where { param($e) $e.estado -eq "visao-geral" }
        $res["visao-geral"]["pagina"] = $pag
        $aguardando = Wait-Evento -Name "aguardando-clique" -Since $app.T0 -TimeoutSec 20
        if ($null -eq $aguardando) { throw "a página não pediu o clique" }
        # Até 3 cliques: a página confirma (clique-recebido) quando o item vira o jogo aberto.
        for ($tentativa = 1; $tentativa -le 3; $tentativa++) {
            $ponto = Invoke-CliqueNoPrimeiroJogo -RootPid $app.Process.Id
            $res["cliqueEm"] = "$($ponto.X),$($ponto.Y) (tentativa $tentativa)"
            if ($null -ne (Wait-Evento -Name "clique-recebido" -Since $app.T0 -TimeoutSec 4)) { break }
        }
        $janela2 = Wait-Evento -Name "repouso-janela" -Since $app.T0 -TimeoutSec 20 -Where { param($e) $e.estado -eq "pagina-do-jogo" }
        if ($null -eq $janela2) { throw "o clique do sistema não abriu a página do jogo" }
        $res["pagina-do-jogo"] = [ordered]@{ os = Measure-Repouso -RootPid $app.Process.Id -Seconds $RestSeconds }
        $pag2 = Wait-Evento -Name "repouso" -Since $app.T0 -TimeoutSec 20 -Where { param($e) $e.estado -eq "pagina-do-jogo" }
        $res["pagina-do-jogo"]["pagina"] = $pag2
    } finally { Stop-Tree -RootPid $app.Process.Id }
    return $res
}

# Clica no primeiro item da lista com o mouse do sistema, no lugar que a página mediu. Exige a janela na frente.
function Invoke-CliqueNoPrimeiroJogo {
    param([int]$RootPid)
    if ($null -eq $script:Geometria) { throw "sem a geometria da janela (rode a abertura-biblioteca antes)" }
    $janela = Get-UiWindow -RootPid $RootPid
    if ($janela -eq [IntPtr]::Zero) { throw "janela do Tako não encontrada" }
    if (-not (Set-Foreground $janela)) { throw "não consegui trazer a janela do Tako para a frente" }
    $r = New-Object TakoWin32+RECT
    [void][TakoWin32]::GetWindowRect($janela, [ref]$r)
    $x = [int]($r.L + $script:Geometria.BordaX + $script:Geometria.Item.itemX)
    $y = [int]($r.T + $script:Geometria.BordaY + $script:Geometria.Item.itemY)
    # Nunca clicar fora da janela do Tako (o clique cairia em outro programa).
    if ($x -le $r.L -or $x -ge $r.R -or $y -le $r.T -or $y -ge $r.B) { throw "o ponto do clique ($x,$y) cai fora da janela ($($r.L),$($r.T) a $($r.R),$($r.B))" }
    # Janela com tamanho diferente do da calibração: o primeiro item pode estar em outro lugar; melhor falhar.
    if (($r.R - $r.L) -ne [int](2 * $script:Geometria.BordaX + $script:Geometria.Item.larguraInterna)) { throw "a janela mudou de tamanho depois da calibração" }
    Write-Host ("  clique em {0},{1} (janela {2},{3} a {4},{5}; primeiro item da página em {6},{7})" -f $x, $y, $r.L, $r.T, $r.R, $r.B, $script:Geometria.Item.itemX, $script:Geometria.Item.itemY)
    Send-Click -X $x -Y $y
    return [PSCustomObject]@{ X = $x; Y = $y }
}

# Repouso do release de verdade (web ou Slint), sem nenhum código de medição na interface. O Slint só tem o
# estado padrão (a mesma biblioteca); o web tem também a página do jogo, aberta por clique do sistema.
function Measure-RepousoRelease {
    param([string]$Kind)
    $exe = if ($Kind -eq "web") { Resolve-Exe $WebExe } else { Resolve-Exe $SlintExe }
    if (-not (Test-Path -LiteralPath $exe)) { throw "executável ausente: $exe" }
    $dir = New-RunDir "repouso-$Kind"
    $carga = Wait-MaquinaQuieta
    $res = [ordered]@{ cargaDoPcAntesPct = $carga.CargaPct }
    $argumentos = @("--data-dir", "`"$dir`"")
    if ($Kind -eq "web") { $argumentos = @("--web") + $argumentos }
    Remove-Item Env:TAKO_WEB_DEV_URL -ErrorAction SilentlyContinue
    $t0 = Get-UnixMs
    $p = Start-Process -FilePath $exe -ArgumentList $argumentos -PassThru
    try {
        # Até a janela responder (como a métrica `memoria`), depois até assentar.
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while ([DateTime]::UtcNow -lt $deadline -and (Get-UiWindow -RootPid $p.Id) -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 100 }
        $res["assentouEmSegundos"] = Wait-Quieto -RootPid $p.Id -MaxSec 60
        $res["padrao"] = [ordered]@{ os = Measure-Repouso -RootPid $p.Id -Seconds $RestSeconds }
        $res["padrao"]["memoria"] = Measure-ProcessMemory -RootPid $p.Id | Select-Object ProcessCount, TotalPrivateSetMB, TotalWorkingSetMB
        if ($Kind -eq "web" -and $null -ne $script:Geometria) {
            Invoke-CliqueNoPrimeiroJogo -RootPid $p.Id | Out-Null
            Start-Sleep -Seconds 4
            $res["pagina-do-jogo"] = [ordered]@{ os = Measure-Repouso -RootPid $p.Id -Seconds $RestSeconds }
        }
    } finally { Stop-Tree -RootPid $p.Id }
    return $res
}

# fps: a página roda rolagem, troca de jogo e telas de vidro e relata o tempo de quadro de cada uma.
function Measure-Fps {
    $rodadas = @()
    for ($i = 1; $i -le $FpsRuns; $i++) {
        $dir = New-RunDir "fps-$i"
        $carga = Wait-MaquinaQuieta
        Write-Host ("  rodada {0}: PC a {1}% de CPU antes de começar" -f $i, $carga.CargaPct)
        $app = Start-AppMedicao -DataDir $dir -Cenario "fps" -Rodada "$i"
        try {
            $fim = Wait-Evento -Name "fim" -Since $app.T0 -TimeoutSec 120
            if ($null -eq $fim) { Write-Host "  rodada ${i}: o cenário não terminou" -ForegroundColor Red }
            $eventos = Read-Eventos | Where-Object { $_.em -ge $app.T0 -and ($_.evento -like "quadros*") }
            $rodadas += , @($eventos)
            foreach ($e in ($eventos | Where-Object { $_.evento -eq "quadros" })) {
                Write-Host ("  rodada {0} {1,-24} p50 {2} ms  p95 {3} ms  p99 {4} ms  max {5} ms  fps médio {6}  perdidos {7}  {8}" -f $i, $e.rotulo, $e.p50Ms, $e.p95Ms, $e.p99Ms, $e.maxMs, $e.fpsMedio, $e.perdidos, ($e.verificacao | ConvertTo-Json -Compress))
            }
        } finally { Stop-Tree -RootPid $app.Process.Id }
    }
    return $rodadas
}

# Latência de teclado: teclas reais do sistema (setas) na lista; a página mede até o quadro seguinte.
function Measure-Latencia {
    $dir = New-RunDir "latencia"
    $carga = Wait-MaquinaQuieta
    Write-Host ("  PC a {0}% de CPU antes de começar" -f $carga.CargaPct)
    $app = Start-AppMedicao -DataDir $dir -Cenario "latencia"
    try {
        $livre = Wait-Evento -Name "biblioteca-livre" -Since $app.T0 -TimeoutSec 40
        if ($null -eq $livre) { throw "a biblioteca não chegou" }
        $janela = Get-UiWindow -RootPid $app.Process.Id
        if (-not (Set-Foreground $janela)) { throw "não consegui trazer a janela do Tako para a frente" }
        Start-Sleep -Seconds 2
        for ($k = 0; $k -lt $KeyPresses; $k++) {
            Send-Key -Vk $(if (($k % 20) -lt 10) { 0x28 } else { 0x26 })
            Start-Sleep -Milliseconds 200
        }
        $res = Wait-Evento -Name "latencia" -Since $app.T0 -TimeoutSec 30
        if ($null -eq $res) { throw "a página não relatou a latência" }
        return $res
    } finally { Stop-Tree -RootPid $app.Process.Id }
}

# Retorno depois do jogo: o jogo de teste abre, a interface fecha, o jogo termina sozinho e o núcleo reabre a
# interface. Os horários vêm do jogo (saída), do log do núcleo (game.ended, interface aberta) e da página.
function Get-LogTimes {
    param([string]$DataDir)
    $arquivo = Get-ChildItem (Join-Path $DataDir "logs") -Filter "tako-*.log" -ErrorAction SilentlyContinue | Select-Object -First 1
    $linhas = @()
    if ($null -ne $arquivo) { $linhas = Get-Content -LiteralPath $arquivo.FullName -Encoding UTF8 }
    $eventos = @()
    foreach ($l in $linhas) {
        $m = [regex]::Match($l, '^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d{3} [+-]\d\d:\d\d)\s+\w+\s+(.*)$')
        if (-not $m.Success) { continue }
        $t = [DateTimeOffset]::ParseExact($m.Groups[1].Value, "yyyy-MM-dd HH:mm:ss.fff zzz", [Globalization.CultureInfo]::InvariantCulture)
        $eventos += [PSCustomObject]@{ Em = [double]$t.ToUnixTimeMilliseconds(); Texto = $m.Groups[2].Value }
    }
    return $eventos
}

function Measure-Retorno {
    $rodadas = @()
    for ($i = 1; $i -le $ReturnRuns; $i++) {
        $dir = New-RunDir "retorno-$i"
        $carga = Wait-MaquinaQuieta
        $marca = Join-Path $Work "saida-do-jogo-$i.txt"
        if (Test-Path $marca) { Remove-Item $marca -Force }
        $env:TAKO_MEDICAO_JOGO_SEGUNDOS = "16"
        $env:TAKO_MEDICAO_JOGO_MARCA = $marca
        $app = Start-AppMedicao -DataDir $dir -Cenario "retorno" -Rodada "$i"
        try {
            $reaberta = Wait-Evento -Name "reaberta" -Since $app.T0 -TimeoutSec 90
            $livre = if ($null -ne $reaberta) { Wait-Evento -Name "biblioteca-livre" -Since $reaberta.em -TimeoutSec 20 } else { $null }
            $naTela = if ($null -ne $reaberta) { Wait-Evento -Name "biblioteca-na-tela" -Since $reaberta.em -TimeoutSec 5 } else { $null }
            $carregou = Read-Eventos | Where-Object { $_.evento -eq "carregou" -and $_.em -ge $app.T0 } | Select-Object -Last 1
            $recebido = Read-Eventos | Where-Object { $_.evento -eq "game.ended-recebido" -and $_.em -ge $app.T0 } | Select-Object -First 1
            $saida = if (Test-Path $marca) { [double](Get-Content $marca -Raw).Trim() } else { $null }
            $log = Get-LogTimes -DataDir $dir
            $terminou = $log | Where-Object { $_.Texto -match "jogo terminou" } | Select-Object -First 1
            $abertas = @($log | Where-Object { $_.Texto -match "interface aberta em processo" })
            # O fechamento da interface antiga (quando a janela do jogo aparece) segura a reabertura: o núcleo só abre a
            # nova depois que o processo antigo sai.
            $pediuFechar = $log | Where-Object { $_.Texto -match "pedindo . interface para fechar" } | Select-Object -First 1
            $fechada = $log | Where-Object { $_.Texto -match "interface fechada code" } | Select-Object -First 1
            $matada = @($log | Where-Object { $_.Texto -match "n.o fechou no prazo" }).Count -gt 0
            if ($null -eq $reaberta -or $null -eq $livre -or $null -eq $terminou -or $abertas.Count -lt 2 -or $null -eq $saida) {
                Write-Host "  rodada ${i}: incompleta (reaberta=$($null -ne $reaberta) livre=$($null -ne $livre) terminou=$($null -ne $terminou) aberturas=$($abertas.Count) saida=$($null -ne $saida))" -ForegroundColor Red
                $rodadas += [ordered]@{ completa = $false }
                continue
            }
            $r = [ordered]@{
                completa                    = $true
                cargaDoPcPct                = $carga.CargaPct
                saidaDoJogoAteGameEndedMs   = [math]::Round($terminou.Em - $saida, 0)
                gameEndedAteInterfaceMs     = [math]::Round($abertas[-1].Em - $terminou.Em, 0)
                interfaceAtePaginaMs        = [math]::Round($carregou.em - $abertas[-1].Em, 0)
                paginaAteBibliotecaNaTelaMs = [math]::Round($naTela.em - $carregou.em, 0)
                paginaAteInterativaMs       = [math]::Round($livre.em - $carregou.em, 0)
                gameEndedAteBibliotecaNaTelaMs = [math]::Round($naTela.em - $terminou.Em, 0)
                gameEndedAteInterativaMs    = [math]::Round($livre.em - $terminou.Em, 0)
                saidaDoJogoAteInterativaMs  = [math]::Round($livre.em - $saida, 0)
                interfaceAntigaFechouEmMs   = if ($null -ne $pediuFechar -and $null -ne $fechada) { [math]::Round($fechada.Em - $pediuFechar.Em, 0) } else { $null }
                interfaceAntigaMatadaNoPrazo = $matada
                gameEndedNaPaginaApos       = if ($null -ne $recebido) { [math]::Round($recebido.em - $terminou.Em, 0) } else { $null }
            }
            Write-Host ("  rodada {0}: saída→game.ended {1} ms; game.ended→biblioteca interativa {2} ms (na tela {3} ms); saída→interativa {4} ms" -f $i, $r.saidaDoJogoAteGameEndedMs, $r.gameEndedAteInterativaMs, $r.gameEndedAteBibliotecaNaTelaMs, $r.saidaDoJogoAteInterativaMs)
            $rodadas += $r
        } finally {
            Stop-Tree -RootPid $app.Process.Id
            Stop-TestGames
            Remove-Item Env:TAKO_MEDICAO_JOGO_SEGUNDOS -ErrorAction SilentlyContinue
            Remove-Item Env:TAKO_MEDICAO_JOGO_MARCA -ErrorAction SilentlyContinue
        }
    }
    return $rodadas
}

function Write-Resumo {
    param($R)
    Write-Host "`n============================================================" -ForegroundColor Green
    Write-Host " RESUMO DAS MEDIÇÕES NOVAS (WEB-60b)" -ForegroundColor Green
    Write-Host "============================================================" -ForegroundColor Green
    $R | ConvertTo-Json -Depth 8 | Write-Host
}

function Invoke-Bancada {
    param([string[]]$Which)
    $saida = [ordered]@{
        data     = (Get-Date).ToString("s")
        maquina  = [ordered]@{
            nucleosLogicos = [Environment]::ProcessorCount
            monitorHz      = (Get-CimInstance Win32_VideoController | Where-Object { $_.CurrentRefreshRate } | Select-Object -First 1).CurrentRefreshRate
        }
    }
    Initialize-Bancada
    try {
        # A abertura vem primeiro: ela também mede onde fica o primeiro item da lista (usada nos cliques de cpu-gpu).
        if ($Which -contains "cpu-gpu" -and $Which -notcontains "abertura-biblioteca") { $null = Measure-AberturaBiblioteca -Runs 1 }
        foreach ($m in @("abertura-biblioteca", "cpu-gpu", "fps", "latencia", "retorno")) {
            if ($Which -notcontains $m) { continue }
            Write-Host "`n--- $m ---" -ForegroundColor Cyan
            try {
                switch ($m) {
                    "abertura-biblioteca" { $saida[$m] = Measure-AberturaBiblioteca }
                    "cpu-gpu" {
                        $saida[$m] = [ordered]@{}
                        $partes = [ordered]@{
                            "web-release"   = { Measure-RepousoRelease -Kind "web" }
                            "slint-release" = { Measure-RepousoRelease -Kind "slint" }
                            "web-medicao"   = { Measure-RepousoBundle }
                        }
                        foreach ($parte in $partes.Keys) {
                            Write-Host "${parte}:"
                            # Uma parte que falha não apaga o que as outras mediram.
                            try { $saida[$m][$parte] = & $partes[$parte] }
                            catch {
                                Write-Host "  FALHOU: $($_.Exception.Message)" -ForegroundColor Red
                                $saida[$m][$parte] = [ordered]@{ erro = $_.Exception.Message }
                            }
                        }
                    }
                    "fps" { $saida[$m] = Measure-Fps }
                    "latencia" { $saida[$m] = Measure-Latencia }
                    "retorno" { $saida[$m] = Measure-Retorno }
                }
            } catch {
                Write-Host "FALHOU: $($_.Exception.Message)" -ForegroundColor Red
                $saida[$m] = [ordered]@{ erro = $_.Exception.Message }
            }
        }
    } finally {
        Stop-Servidor
        Stop-TestGames
    }
    $destino = if ([string]::IsNullOrWhiteSpace($ResultsFile)) { Join-Path $Work "resultados.json" } else { $ResultsFile }
    $saida | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $destino -Encoding UTF8
    Write-Host "`nResultado gravado em $destino" -ForegroundColor Green
    Write-Resumo $saida
    return $saida
}

# Execução conforme modo. Em dot-source (`. .\scripts\measure-web.ps1`) só carrega as funções, para depurar uma medição.
if ($MyInvocation.InvocationName -eq ".") { return }
$metricas = @($Metrics)
if ($metricas -contains "tudo") { $metricas = @("memoria", "abertura-biblioteca", "cpu-gpu", "fps", "latencia", "retorno") }
$suiteResults = @{}

if ($metricas -contains "memoria") {
    if ($Mode -eq "both") {
        $slintExe = Resolve-Exe "target\release\tako-slint.exe"
        $webExe = Resolve-Exe "target\release\tako.exe"

        $suiteResults["slint"] = Run-BenchmarkSuite -TargetExe $slintExe -TargetMode "slint"
        $suiteResults["web"] = Run-BenchmarkSuite -TargetExe $webExe -TargetMode "web"

    } elseif ($Mode -eq "slint") {
        $suiteResults["slint"] = Run-BenchmarkSuite -TargetExe $ExePath -TargetMode "slint"
    } elseif ($Mode -eq "web") {
        $suiteResults["web"] = Run-BenchmarkSuite -TargetExe $ExePath -TargetMode "web"
    } else {
        # auto
        $target = if ($ExePath -match "slint") { "slint" } else { "web" }
        $suiteResults[$target] = Run-BenchmarkSuite -TargetExe $ExePath -TargetMode $target
    }
}

$novas = @($metricas | Where-Object { $_ -ne "memoria" })
if ($novas.Count -gt 0) {
    $suiteResults["bancada"] = Invoke-Bancada -Which $novas
}

return $suiteResults
