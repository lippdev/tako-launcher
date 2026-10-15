using System.Windows.Threading;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.Display;
using Launcher.Core.Session;
using Launcher.Core.Session.GameScreen;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// "Modo console" (MVP-002/012/036): ao abrir, o Tako vai para a tela de jogo como o Console Mode e só volta para a mesa
/// ao sair ou em "Voltar ao PC". Também concentra o trabalho lento fora da thread da UI (voltar das telas, restaurar
/// monitores e áudio) com "Voltando para a mesa…" na tela, a recuperação depois de uma queda e o atalho de emergência.
/// A lógica fica no <see cref="ConsoleModeSession"/> (Core).
/// </summary>
public sealed partial class ShellViewModel
{
    private const string ReturningToDesk = "Voltando para a mesa…";
    private const string EndingSession = "Encerrando a sessão…";
    private const string ReturningDetail = "Telas, áudio e TV voltando a como estavam";

    private readonly Dispatcher _uiDispatcher = Dispatcher.CurrentDispatcher;

    /// <summary>Última volta pedida: a próxima só começa depois dela (o fim de um jogo, "Voltar ao PC" e as restaurações nunca se cruzam).</summary>
    private Task _deskTail = Task.CompletedTask;

    /// <summary>A tela de jogo foi focada por este jogo (fora do modo console) e volta quando ele fechar.</summary>
    private bool _screenFocusedForGame;

    private int _offUiWork;

    public ConsoleModeSession ConsoleSession { get; }

    /// <summary>A tela de jogo está em foco desde a abertura (modo console); "Voltar ao PC" aparece no menu e na Central.</summary>
    public bool IsConsoleModeActive => ConsoleSession.IsActive;

    /// <summary>Atalho de teclado que devolve as telas para a mesa em qualquer situação.</summary>
    public string EmergencyKeys => _emergencyHotkey.Keys;

    /// <summary>Há uma volta para a mesa (ou restauração) em andamento fora da thread da UI.</summary>
    public bool IsReturningToDesk => Volatile.Read(ref _offUiWork) > 0;

    // ---------------------------------------------------------------- Abertura

    /// <summary>
    /// Abertura, nesta ordem: atalho de emergência, recuperação do modo console (o Tako caiu na tela de jogo), oferta de
    /// restaurar monitores e áudio de um jogo (ADR-0013) e, por fim, o modo console ao abrir. Com a oferta na tela, o
    /// Tako fica na mesa nesta abertura: a resposta vem antes de mexer nas telas de novo.
    /// </summary>
    public async void StartScreensOnLaunch(bool safeMode)
    {
        EnsureUiContext();
        StartEmergencyHotkey();
        await RecoverConsoleModeAsync().ConfigureAwait(true);
        if (OfferEnvironmentRestore())
        {
            if (Settings.ConsoleModeOnLaunch)
            {
                LogConsoleModeDeferred(_logger);
            }

            return;
        }

        await StartConsoleModeAsync(safeMode).ConfigureAwait(true);
    }

    /// <summary>
    /// Se o Tako caiu no modo console, volta para a mesa sem perguntar (o usuário pode não estar vendo tela nenhuma),
    /// fora da thread da UI. O registro do jogo que estava aberto (ADR-0013) foi feito com a tela de jogo em foco e
    /// deixa de valer.
    /// </summary>
    public async Task RecoverConsoleModeAsync()
    {
        if (!ConsoleSession.HasPendingRecovery)
        {
            return;
        }

        var outcome = await RunOffUiAsync(ReturningToDesk, "O Tako fechou no modo console da última vez", ConsoleSession.RecoverFromPreviousRun).ConfigureAwait(true);
        if (outcome == ConsoleModeRecoveryOutcome.NothingPending)
        {
            return;
        }

        _environment.Discard();
        Toast(outcome switch
        {
            ConsoleModeRecoveryOutcome.Restored => "O Tako fechou no modo console · suas telas voltaram para a mesa",
            ConsoleModeRecoveryOutcome.Incomplete => "O Tako fechou no modo console · nem tudo voltou para a mesa (veja os logs)",
            _ => "O Tako fechou no modo console · suas telas já estavam na mesa",
        });
    }

    /// <summary>"Ao abrir o Tako, ir para a tela de jogo": decide e entra (nunca no modo seguro).</summary>
    public async Task StartConsoleModeAsync(bool safeMode)
    {
        switch (ConsoleModePolicy.AtStartup(Settings.ConsoleModeOnLaunch, Settings.GameScreen, safeMode))
        {
            case ConsoleModeStart.Off:
                return;
            case ConsoleModeStart.NoGameScreen:
                Toast("Modo console: escolha a tela de jogo nas Configurações");
                return;
            case ConsoleModeStart.SafeMode:
                Toast("Modo seguro: o Tako abriu na mesa, sem ir para a tela de jogo");
                return;
        }

        await EnterConsoleModeAsync().ConfigureAwait(true);
    }

    /// <summary>
    /// Vai para a tela de jogo como o Console Mode: liga a TV (se configurada), foca a tela, troca o áudio e leva o Tako
    /// para lá. Na primeira vez com uma tela ou modo novo, pergunta se dá para ver a imagem; sem resposta, volta tudo.
    /// </summary>
    public async Task<bool> EnterConsoleModeAsync()
    {
        if (Settings.GameScreen is not { } settings || ConsoleSession.IsActive || RunningGame is not null || IsReturningToDesk)
        {
            return false;
        }

        EnsureUiContext();

        string name = Settings.GameScreenName.Length > 0 ? Settings.GameScreenName : "tela de jogo";
        var progress = new ProgressOverlayViewModel("Indo para a tela de jogo…", Settings.TvControl.IsActive ? "Ligando a TV e trocando a entrada" : $"Usando {name}");
        PushOverlay(progress);
        GameScreenResult result;
        try
        {
            result = await ConsoleSession.EnterAsync(settings, DeviceOptions(), GameScreenConfirmations.Matching(Settings.GameScreenConfirmedSetup, settings.SetupKey)).ConfigureAwait(true);
        }
        finally
        {
            CloseIfOpen(progress);
        }

        switch (result.Outcome)
        {
            case GameScreenOutcome.Skipped:
                Toast($"Modo console: {result.Reason} · o Tako abriu na mesa");
                return false;
            case GameScreenOutcome.Reverted:
                Toast($"Modo console: {result.Reason} · as telas voltaram como estavam");
                return false;
        }

        if (result.TargetBounds is { } bounds)
        {
            MoveLauncherTo(bounds);
        }

        OnConsoleModeChanged();
        if (result.NeedsConfirmation)
        {
            if (!await ConfirmGameScreenAsync(result.Target?.Name ?? name).ConfigureAwait(true))
            {
                var report = await RunOffUiAsync(ReturningToDesk, ReturningDetail, () => ConsoleSession.Exit(standbyTv: false)).ConfigureAwait(true);
                AfterConsoleModeLeft(report, "Tela de jogo não confirmada · suas telas voltaram ao normal");
                return false;
            }

            Settings.GameScreenConfirmedSetup = GameScreenConfirmations.Remember(Settings.GameScreenConfirmedSetup, settings.SetupKey);
            if (!ConsoleSession.IsActive)
            {
                return false; // o atalho de emergência já voltou as telas durante a pergunta
            }
        }

        Toast($"Modo console em {result.Target?.Name ?? name} · \"Voltar ao PC\" fica no menu");
        return true;
    }

    // ---------------------------------------------------------------- Voltar ao PC

    /// <summary>Menu ou Central de Controle › "Voltar ao PC": pergunta antes, porque as telas mudam.</summary>
    [RelayCommand]
    public void ConfirmReturnToDesk()
    {
        if (!ConsoleSession.IsActive)
        {
            Toast("O Tako já está na mesa");
            return;
        }

        string game = RunningGame is { } running ? $"{running.Title} continua aberto. " : string.Empty;
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            "Voltar ao PC?",
            $"{game}As telas, o áudio e a TV voltam a como estavam antes do modo console. O Tako continua aberto.",
            [
                new OverlayAction("Voltar ao PC", ActionStyle.Primary, new AsyncRelayCommand(async () =>
                {
                    CloseOverlay(dialog!);
                    await ReturnToDeskAsync().ConfigureAwait(true);
                }), IsDefault: true),
                new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
            ]);
        PushOverlay(dialog);
    }

    /// <summary>Sai do modo console fora da thread da UI, com "Voltando para a mesa…" na tela.</summary>
    public async Task ReturnToDeskAsync()
    {
        var report = await RunOffUiAsync(ReturningToDesk, ReturningDetail, () => ConsoleSession.Exit(standbyTv: true)).ConfigureAwait(true);
        AfterConsoleModeLeft(report, "Você está de volta à mesa");
    }

    /// <summary>
    /// Fechando o Tako (App.OnExit): ainda no modo console, volta para a mesa agora, na thread que estiver. "Sair para
    /// o desktop" já faz isso antes, com a tela de andamento; aqui fica a garantia para os outros caminhos (Alt+F4, fim
    /// da sessão do Windows, recuperação de erro).
    /// </summary>
    public void LeaveConsoleModeOnExit()
    {
        if (ConsoleSession.IsActive)
        {
            ConsoleSession.Exit(standbyTv: true);
        }
    }

    // ---------------------------------------------------------------- Atalho de emergência

    /// <summary>Registra o atalho de emergência (Ctrl+Alt+Shift+D) para a vida inteira do Tako.</summary>
    public bool StartEmergencyHotkey()
    {
        bool started = _emergencyHotkey.Start(OnEmergencyHotkey);
        if (!started)
        {
            LogEmergencyHotkeyUnavailable(_logger, _emergencyHotkey.Keys);
        }

        return started;
    }

    /// <summary>
    /// Thread do atalho, não a da UI: funciona com a interface ocupada ou com o jogo na frente. Sai do modo console sem
    /// colocar a TV em espera e desfaz a tela de jogo de um jogo; a UI é atualizada depois.
    /// </summary>
    private void OnEmergencyHotkey()
    {
        LogEmergencyHotkey(_logger);
        var report = ConsoleSession.Exit(standbyTv: false);
        bool hadScreen = GameScreen.IsActive;
        string screens = hadScreen ? EndGameScreen() : string.Empty;
        _uiDispatcher.BeginInvoke(() =>
        {
            if (report.WasActive)
            {
                AfterConsoleModeLeft(report, $"Telas de volta na mesa ({EmergencyKeys})");
            }
            else
            {
                Toast(hadScreen ? $"Telas de volta na mesa ({EmergencyKeys}){screens}" : "As telas já estão na mesa");
            }
        });
    }

    // ---------------------------------------------------------------- Trabalho fora da thread da UI

    /// <summary>
    /// Roda <paramref name="work"/> fora da thread da UI com a tela de andamento por cima, uma volta por vez. A tela
    /// continua respondendo (heartbeat do supervisor, animações); "Jogar" espera a volta terminar.
    /// </summary>
    private async Task<T> RunOffUiAsync<T>(string title, string detail, Func<T> work)
    {
        EnsureUiContext();
        var progress = new ProgressOverlayViewModel(title, detail);
        Interlocked.Increment(ref _offUiWork);
        PushOverlay(progress);
        try
        {
            // Encadeada na anterior (termine ela bem ou não), sempre numa thread do pool.
            var current = _deskTail.ContinueWith(_ => work(), CancellationToken.None, TaskContinuationOptions.None, TaskScheduler.Default);
            _deskTail = current;
            return await current.ConfigureAwait(true);
        }
        finally
        {
            Interlocked.Decrement(ref _offUiWork);
            CloseIfOpen(progress);
        }
    }

    /// <summary>
    /// Os awaits daqui voltam para a thread da UI pelo contexto de sincronização. Chamado fora de um ciclo do Dispatcher
    /// (ex.: um comando executado direto), a thread da UI ainda não tem o contexto: instala o do Dispatcher dela.
    /// </summary>
    private void EnsureUiContext()
    {
        if (SynchronizationContext.Current is null && _uiDispatcher.CheckAccess())
        {
            SynchronizationContext.SetSynchronizationContext(new DispatcherSynchronizationContext(_uiDispatcher));
        }
    }

    private void CloseIfOpen(object overlay)
    {
        if (Overlays.Contains(overlay))
        {
            CloseOverlay(overlay);
        }
    }

    /// <summary>Depois de sair do modo console (thread da UI): menu, aviso e o registro do jogo ainda aberto.</summary>
    private void AfterConsoleModeLeft(ConsoleModeExitReport report, string message)
    {
        if (!report.WasActive)
        {
            return;
        }

        OnConsoleModeChanged();

        // Jogo ainda aberto: o registro dele (ADR-0013) foi feito na tela de jogo; o "antes" do jogo agora é a mesa.
        if (RunningGame is { } game && IsTrackingRealGame)
        {
            _environment.BeginSession(game.Id, Settings.RestoreEnvironmentAfterGame);
        }

        Toast(report.Succeeded ? message : $"{message} · as telas não voltaram exatamente como antes (veja os logs)");
    }

    private void OnConsoleModeChanged()
    {
        OnPropertyChanged(nameof(IsConsoleModeActive));
        if (SidebarItems.FirstOrDefault(s => s.Key == "desk") is { } desk)
        {
            desk.IsHidden = !ConsoleSession.IsActive;
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Modo console ao abrir adiado: a pergunta sobre monitores e áudio vem antes")]
    private static partial void LogConsoleModeDeferred(ILogger logger);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Atalho de emergência pressionado: voltando as telas para a mesa")]
    private static partial void LogEmergencyHotkey(ILogger logger);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Atalho de emergência {Keys} indisponível (outro programa já usa a combinação)")]
    private static partial void LogEmergencyHotkeyUnavailable(ILogger logger, string keys);
}
