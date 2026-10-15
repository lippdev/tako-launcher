using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Windows;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.SessionTools;
using Launcher.Core.Library;
using Launcher.Core.Presentation;
using Launcher.Core.Session;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Sessão de jogo real (MVP-004/026/027): estado pela máquina de estados, abrir o jogo, acompanhar o processo ou a pasta
/// até ele fechar e retomar a biblioteca. A sessão simulada (jogos sem <c>LaunchTarget</c>) continua em ShellViewModel.cs.
/// </summary>
public sealed partial class ShellViewModel
{
    private readonly SessionStateMachine _sessionMachine = new(SessionState.Browsing);
    private GameProcessMonitor? _monitor;

    /// <summary>Estado da sessão: só muda por transições válidas (MVP-004); as inválidas são registradas e ignoradas.</summary>
    public SessionState Session
    {
        get => _sessionMachine.State;
        private set
        {
            var transition = _sessionMachine.TryTransition(value);
            if (!transition.Accepted)
            {
                LogSessionTransitionRejected(_logger, transition.From, transition.To);
            }
            else if (!transition.IsNoOp)
            {
                OnPropertyChanged();
            }
        }
    }

    /// <summary>Um jogo real está aberto e sendo acompanhado.</summary>
    public bool IsTrackingRealGame => _monitor?.IsTracking == true;

    /// <summary>Abre o jogo de verdade (URI da loja ou executável/atalho) e começa a acompanhar até ele fechar.</summary>
    private async Task StartRealSessionAsync(GameItemViewModel item)
    {
        var game = item.Game;
        var effective = ResolveSessionSettings(game.Id); // Configurações do Tako + deste jogo (UX-007)
        CaptureEnvironmentBeforeGame(game.Id); // antes da tela de jogo: o registro é o estado de antes de tudo
        bool tvReady = await WakeTvForGameScreenAsync(game, effective).ConfigureAwait(true); // a TV liga antes de virar a tela de jogo
        if (!await FocusGameScreenAsync(effective).ConfigureAwait(true))
        {
            Session = SessionState.Browsing; // tela de jogo não confirmada: as telas já voltaram
            _environment.Discard();
            return;
        }

        Process? process;
        try
        {
            process = Process.Start(new ProcessStartInfo(game.LaunchTarget!)
            {
                UseShellExecute = true,
                Arguments = effective.Arguments(game.LaunchArguments),
                WorkingDirectory = game.WorkingDirectory ?? string.Empty,
            });
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException or FileNotFoundException)
        {
            Session = SessionState.Browsing; // falha ao abrir: nada mudou
            RestoreGameScreen();
            _environment.Discard();
            LogLaunchFailed(_logger, ex, game.Id);
            // Cliente da loja ausente (STORE-012): o protocolo não está registrado no Windows.
            Toast(game.Store == GameStore.Epic && LibrarySources.Reports.GetValueOrDefault(GameStore.Epic) is { ClientInstalled: false }
                ? $"Epic Games Launcher não encontrado: instale-o para abrir {game.Title}"
                : $"Não foi possível abrir {game.Title}: {ex.Message}");
            return;
        }

        var launchedAt = _time.GetUtcNow();
        int? root = RootProcessId(process);
        var plan = GameTracking.Plan(game, root is not null);
        Library.RecordLaunch(game.Id);
        if (plan.Strategy == GameTrackingStrategy.None)
        {
            // Sem como acompanhar (ex.: URI sem pasta conhecida): o jogo só é aberto, como antes do MVP-026.
            Session = SessionState.Browsing;
            RestoreGameScreen(); // sem saber quando o jogo fecha, a tela de jogo não teria quando voltar
            _environment.Discard(); // sem saber quando o jogo fecha, não há momento seguro para restaurar
            Toast($"Abrindo {game.Title}{LaunchVia(game)}");
            return;
        }

        var tracker = new GameRunTracker(plan, launchedAt, plan.Strategy == GameTrackingStrategy.Process ? root : null);
        RunningGame = ItemFor(Library.Find(game.Id) ?? game);
        RunningGame.IsRunning = true;
        if (tracker.Status == GameRunStatus.Running)
        {
            Session = SessionState.Playing;
        }

        _monitor ??= new GameProcessMonitor(_time);
        _monitor.Start(tracker, OnRealGameStatusChanged, _devices.SteamRunningAppId);
        // Contador de FPS (filtra os processos do jogo, que mudam: launcher → jogo) e limite escolhido na Central.
        SessionTools.BeginSession(GameExecutables.FromLaunchTarget(game.LaunchTarget), () => _monitor?.Tracker?.GameProcessIds.ToArray() ?? [], ToolChoices(effective));
        BeginSessionDevices(game, tvReady, effective); // TV (se ainda não ligou) e saída de áudio (MVP-002) em segundo plano
        LogRealSessionStarted(_logger, game.Id, plan.Strategy);
        Toast(tracker.Status == GameRunStatus.Running
            ? $"Jogando {game.Title} · o Tako espera você voltar"
            : $"Abrindo {game.Title}{LaunchVia(game)} · aguardando o jogo iniciar");
        CurrentPage.Refresh();
    }

    /// <summary>" pela Steam", " pela Epic Games", " pelo GOG Galaxy" ou nada (executável direto).</summary>
    private static string LaunchVia(Game game) => game.Store switch
    {
        GameStore.Steam => " pela Steam",
        GameStore.Epic => " pela Epic Games",
        GameStore.Gog when game.LaunchTarget?.EndsWith("GalaxyClient.exe", StringComparison.OrdinalIgnoreCase) == true => " pelo GOG Galaxy",
        _ => string.Empty,
    };

    private static int? RootProcessId(Process? process)
    {
        if (process is null)
        {
            return null;
        }

        try
        {
            return process.Id;
        }
        catch (InvalidOperationException)
        {
            return null;
        }
        finally
        {
            process.Dispose();
        }
    }

    private void OnRealGameStatusChanged(GameRunTracker tracker)
    {
        switch (tracker.Status)
        {
            case GameRunStatus.Running:
                Session = SessionState.Playing;
                SessionTools.GameExecutableFound(GameExecutables.Pick(tracker.GameExecutablePaths)); // perfil do driver (ADR-0018)
                break;
            case GameRunStatus.Exited:
                EndRealSession(tracker);
                break;
            case GameRunStatus.TimedOut:
                AbandonRealSession();
                break;
        }
    }

    /// <summary>O jogo não apareceu no prazo: volta a navegar sem registrar tempo.</summary>
    private void AbandonRealSession()
    {
        var game = RunningGame;
        _monitor?.Stop();
        CloseRealGameOverlays();
        SessionTools.EndSession();
        ClearRunningGame();
        Session = SessionState.Browsing;
        RestoreGameScreen(); // sem jogo, a tela de jogo não fica em foco
        EndSessionDevices(game, gameEnded: false); // devolve a saída de áudio; TV e Steam ficam como estão
        _environment.Discard(); // o jogo pode abrir depois: restaurar agora atrapalharia
        if (game is not null)
        {
            Toast($"{game.Title} não iniciou em {(int)GameRunTracker.DefaultStartTimeout.TotalMinutes} min · tempo de jogo não registrado");
        }

        CurrentPage.Refresh();
        OnOverlaysChanged();
    }

    /// <summary>O jogo fechou (MVP-027): registra o tempo real, traz o Tako para frente e devolve o foco ao jogo.</summary>
    private void EndRealSession(GameRunTracker tracker)
    {
        var game = RunningGame;
        var played = tracker.PlayTime;
        _monitor?.Stop();
        CloseRealGameOverlays();
        SessionTools.EndSession(); // ponteiro visível e limite de FPS do usuário de volta
        Session = SessionState.Returning;
        ClearRunningGame();
        if (game is not null && Library.Find(game.Id) is not null)
        {
            Library.RecordSession(game.Id, played);
            LogSessionEnded(_logger, game.Id);
        }

        Session = SessionState.Restoring;
        string screens = RestoreGameScreen(); // tela de jogo primeiro: a restauração do MVP-012 confere o resultado
        string devices = EndSessionDevices(game, gameEnded: true); // saída de áudio, Steam e TV (MVP-002)
        string restored = RestoreEnvironmentAfterGame() + screens + devices; // monitores e áudio antes de trazer a janela (MVP-012)
        BringLauncherToFront();
        Session = SessionState.Browsing;
        if (game is not null)
        {
            RestoreFocusAfterGame(game.Id);
            Toast($"Bem-vindo de volta · {GameText.SessionPlayed(played)}{restored}");
        }

        OnOverlaysChanged();
    }

    private void ClearRunningGame()
    {
        if (RunningGame is { } running)
        {
            running.IsRunning = false;
            if (_items.TryGetValue(running.Id, out var current))
            {
                current.IsRunning = false;
            }
        }

        RunningGame = null;
    }

    /// <summary>"Voltar ao jogo" com um jogo real: traz a janela dele para frente, quando há uma.</summary>
    private bool TryResumeRealGame()
    {
        if (_monitor is not { IsTracking: true } monitor || RunningGame is not { } game)
        {
            return false;
        }

        if (!monitor.TryBringGameToFront())
        {
            Toast(Session == SessionState.Playing ? $"{game.Title} está aberto · use Alt+Tab para voltar" : $"Aguardando {game.Title} iniciar");
        }

        return true;
    }

    /// <summary>Sem roubar o foco de forma agressiva: só restaura e ativa a janela (o Windows pode só piscar na barra).</summary>
    private static void BringLauncherToFront()
    {
        if (Application.Current?.MainWindow is not { } window)
        {
            return;
        }

        if (window.WindowState == WindowState.Minimized)
        {
            window.WindowState = WindowState.Maximized;
        }

        window.Activate();
    }

    /// <summary>Foco e seleção de antes (MVP-027): o mesmo jogo nos detalhes, no destaque da Início ou na grade.</summary>
    private void RestoreFocusAfterGame(string gameId)
    {
        if (TopOverlay is not null || Library.Find(gameId) is not { } game)
        {
            return;
        }

        var item = ItemFor(game);
        switch (CurrentPage)
        {
            case DetailsViewModel details when details.Game.Id == gameId:
                details.Refresh();
                RequestFocus("details:primary");
                break;
            case HomeViewModel home:
                home.Refresh();
                home.Hero = item;
                SetBackdrop(item);
                RequestFocus("home:play");
                break;
            case LibraryViewModel library:
                library.Refresh();
                RequestFocus($"grid:{gameId}");
                break;
            default:
                CurrentPage.Refresh();
                break;
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Transição de sessão recusada: {From} → {To}")]
    private static partial void LogSessionTransitionRejected(ILogger logger, SessionState from, SessionState to);

    [LoggerMessage(Level = LogLevel.Information, Message = "Jogo {GameId} aberto; acompanhando por {Strategy}")]
    private static partial void LogRealSessionStarted(ILogger logger, string gameId, GameTrackingStrategy strategy);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao abrir o jogo {GameId}")]
    private static partial void LogLaunchFailed(ILogger logger, Exception exception, string gameId);
}
