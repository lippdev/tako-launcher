using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Windows;
using System.Windows.Threading;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Windows.SessionTools;
using Launcher.Core.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>Por que o contador de FPS pode (ou não) medir agora; decide o que Configurações e a Central mostram.</summary>
public enum FpsCounterState
{
    /// <summary>O PresentMon não veio nesta compilação (ex.: rodando pelo código-fonte).</summary>
    NotBundled,

    /// <summary>"Mostrar FPS" desligado nas Configurações.</summary>
    Disabled,

    /// <summary>Ligado, mas o usuário não está no grupo "Usuários de log de desempenho".</summary>
    NeedsPermission,

    /// <summary>Ligado e permitido; falta sair e entrar no Windows.</summary>
    NeedsSignOut,

    Ready,
}

/// <summary>
/// Ferramentas da Central de Controle durante o jogo: janelas abertas, contador de FPS (PresentMon, sem injeção,
/// ADR-0017), limite de FPS (driver de vídeo, ADR-0018), ponteiro, ControlFS e desempenho do PC. Guarda as escolhas do
/// contador e do limite no <c>tako.db</c>, aplica-as no início de cada sessão e desfaz tudo no fim (PresentMon fechado,
/// ponteiro visível, limite do usuário de volta). Chamadas ao driver rodam em fila fora da thread da UI (centenas de ms).
/// </summary>
public sealed partial class SessionToolsService : IDisposable
{
    public const string EnabledKey = "FpsCounter.Enabled";
    private const string StyleKey = "ControlCenter.FpsStyle";
    private const string PositionKey = "ControlCenter.FpsPosition";
    private const string SizeKey = "ControlCenter.FpsSize";
    private const string CardKey = "ControlCenter.FpsCard";
    private const string LimitKey = "ControlCenter.FpsLimit";

    private readonly SettingsStore _store;
    private readonly ILogger<SessionToolsService> _logger;
    private readonly Func<FpsOverlayWindow?> _createOverlay;
    private readonly IDisplayControlService _displays;
    private readonly object _queueGate = new();
    private Task _limitQueue = Task.CompletedTask;
    private string? _gameExe;
    private DispatcherTimer? _overlayTimer;
    private FpsOverlayWindow? _overlay;
    private Func<IReadOnlyCollection<int>>? _gameProcesses;
    private HardwareSample? _overlaySample;
    private bool _overlaySampling;
    private bool _inSession;

    public SessionToolsService(
        IWindowSwitcher windows,
        IFpsCounter fps,
        IPerfLogAccess permission,
        IFrameRateLimiter limiter,
        IPointerService cursor,
        IControlFsService controlFs,
        IHardwareMonitor hardware,
        IDisplayControlService displays,
        SettingsStore store,
        ILogger<SessionToolsService> logger,
        Func<FpsOverlayWindow?>? createOverlay = null)
    {
        Windows = windows;
        Fps = fps;
        Permission = permission;
        Limiter = limiter;
        Cursor = cursor;
        ControlFs = controlFs;
        Hardware = hardware;
        _displays = displays;
        _store = store;
        _logger = logger;
        _createOverlay = createOverlay ?? (() => new FpsOverlayWindow());
        try
        {
            var saved = store.LoadAll();
            FpsCounterEnabled = bool.TryParse(saved.GetValueOrDefault(EnabledKey), out bool enabled) && enabled;
            FpsStyle = FpsOverlay.Normalize(saved.GetValueOrDefault(StyleKey));
            Layout = new FpsOverlayLayout
            {
                Position = FpsOverlay.NormalizePosition(saved.GetValueOrDefault(PositionKey)),
                Size = FpsOverlay.NormalizeSize(saved.GetValueOrDefault(SizeKey)),
                Card = !bool.TryParse(saved.GetValueOrDefault(CardKey), out bool card) || card,
            };
            FpsLimit = int.TryParse(saved.GetValueOrDefault(LimitKey), NumberStyles.Integer, CultureInfo.InvariantCulture, out int limit) ? FpsLimits.Normalize(limit) : 0;
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogPreferencesFailed(logger, ex);
        }

        PermissionState = Permission.Check();
    }

    public IWindowSwitcher Windows { get; }

    public IFpsCounter Fps { get; }

    public IPerfLogAccess Permission { get; }

    /// <summary>Limite de FPS pelo driver de vídeo (NVIDIA, AMD ou Intel), sem RTSS.</summary>
    public IFrameRateLimiter Limiter { get; }

    /// <summary>O que o driver da placa oferece; nulo enquanto a primeira sondagem (em segundo plano) não terminou.</summary>
    public FrameRateLimitCapabilities? LimitCapabilities { get; private set; }

    /// <summary>Taxa da tela principal na última leitura (para "Tela − 3"); nula se desconhecida.</summary>
    public int? DisplayRefreshHz { get; private set; }

    public IPointerService Cursor { get; }

    public IControlFsService ControlFs { get; }

    public IHardwareMonitor Hardware { get; }

    /// <summary>"Mostrar FPS" nas Configurações (desligado por padrão; ligar pede a permissão do Windows uma vez).</summary>
    public bool FpsCounterEnabled { get; private set; }

    /// <summary>Última leitura da permissão (<see cref="RefreshPermission"/>).</summary>
    public PerfLogStatus PermissionState { get; private set; }

    public FpsCounterState CounterState =>
        !Fps.IsAvailable ? FpsCounterState.NotBundled
        : !FpsCounterEnabled ? FpsCounterState.Disabled
        : PermissionState switch
        {
            PerfLogStatus.Ready => FpsCounterState.Ready,
            PerfLogStatus.NeedsSignOut => FpsCounterState.NeedsSignOut,
            _ => FpsCounterState.NeedsPermission,
        };

    /// <summary>Estilo do contador (<see cref="FpsOverlay.Styles"/>).</summary>
    public string FpsStyle { get; private set; } = FpsOverlay.FpsOnly;

    public FpsOverlayLayout Layout { get; private set; } = new();

    /// <summary>Limite de FPS da sessão (0 = sem limite, o do usuário fica valendo; <see cref="FpsLimits.MatchDisplay"/> = tela − 3).</summary>
    public int FpsLimit { get; private set; }

    /// <summary>Números do jogo agora (nulo sem contador ou sem quadros recentes).</summary>
    public FrameStats? CurrentFrames => _inSession && CounterState == FpsCounterState.Ready ? Fps.Current : null;

    /// <summary>Lê de novo se o usuário está no grupo (depois do UAC, ao abrir Configurações e a cada sessão).</summary>
    public PerfLogStatus RefreshPermission() => PermissionState = Permission.Check();

    public void SetFpsCounterEnabled(bool enabled)
    {
        FpsCounterEnabled = enabled;
        Save(EnabledKey, enabled ? "true" : "false");
        if (!enabled)
        {
            StopCounter();
        }
        else if (_inSession)
        {
            StartCounter();
        }
    }

    /// <summary>
    /// Início da sessão de jogo: contador e limite escolhidos voltam a valer. <paramref name="gameExePath"/> é o exe do
    /// jogo quando já se sabe (atalho direto para .exe); senão chega por <see cref="GameExecutableFound"/>.
    /// <paramref name="gameProcesses"/> lista os processos do jogo real agora, para o contador (nulo na sessão simulada,
    /// que não tem o que medir).
    /// </summary>
    public void BeginSession(string? gameExePath = null, Func<IReadOnlyCollection<int>>? gameProcesses = null)
    {
        _inSession = true;
        _gameExe = gameExePath;
        _gameProcesses = gameProcesses;
        if (FpsLimit != 0)
        {
            ApplyLimitNow();
        }

        RefreshPermission();
        StartCounter();
    }

    /// <summary>O processo do jogo apareceu: o perfil do driver passa a ser o dele (exe diferente do atalho, jogo da Steam).</summary>
    public void GameExecutableFound(string? exePath)
    {
        if (!_inSession || string.IsNullOrWhiteSpace(exePath) || string.Equals(exePath, _gameExe, StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        _gameExe = exePath;
        if (FpsLimit != 0)
        {
            ApplyLimitNow();
        }
    }

    /// <summary>Fim da sessão (ou do app): fecha o PresentMon, devolve o limite do usuário, mostra o ponteiro.</summary>
    public void EndSession()
    {
        _inSession = false;
        _gameProcesses = null;
        _gameExe = null;
        Cursor.Reset();
        StopCounter();
        RestoreLimit();
    }

    /// <summary>Abertura do Tako: desfaz o que uma execução que caiu deixou (ponteiro, sessão do PresentMon, limite).</summary>
    public void RecoverAfterCrash()
    {
        Cursor.RecoverAfterCrash();
        Fps.RecoverAfterCrash();
        RestoreLimit();
        Enqueue("ler o limite de FPS do driver", () =>
        {
            LimitCapabilities = Limiter.Capabilities;
            DisplayRefreshHz = ReadRefreshHz();
            return true;
        });
    }

    public void SetFpsStyle(string style)
    {
        FpsStyle = FpsOverlay.Normalize(style);
        Save(StyleKey, FpsStyle);
        RefreshOverlay();
    }

    public void SetLayout(FpsOverlayLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        Layout = layout;
        Save(PositionKey, layout.Position);
        Save(SizeKey, layout.Size);
        Save(CardKey, layout.Card ? "true" : "false");
        RefreshOverlay();
    }

    public void SetFpsLimit(int fps)
    {
        FpsLimit = FpsLimits.Normalize(fps);
        Save(LimitKey, FpsLimit.ToString(CultureInfo.InvariantCulture));
        if (_inSession)
        {
            ApplyLimitNow();
        }
    }

    /// <summary>Uma leitura de CPU/GPU/memória fora da thread da UI; nulo se o Windows não respondeu.</summary>
    public Task<HardwareSample?> SampleAsync() => Task.Run<HardwareSample?>(() =>
    {
        try
        {
            return Hardware.Sample();
        }
        catch (Exception ex) when (ex is System.Runtime.InteropServices.ExternalException or InvalidOperationException or DllNotFoundException or EntryPointNotFoundException)
        {
            LogHardwareFailed(_logger, ex);
            return null;
        }
    });

    /// <summary>Abre uma página no navegador padrão (ex.: download do ControlFS).</summary>
    public bool OpenPage(string url)
    {
        try
        {
            Process.Start(new ProcessStartInfo(url) { UseShellExecute = true })?.Dispose();
            return true;
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            LogOpenPageFailed(_logger, ex, url);
            return false;
        }
    }

    public void Dispose()
    {
        _inSession = false;
        Cursor.Reset();
        StopCounter();
        Task pending;
        lock (_queueGate)
        {
            pending = _limitQueue;
        }

        // Fechando: termina a fila (as falhas dela já foram registradas) e devolve o driver antes de o processo sair.
        pending.ContinueWith(_ => { }, TaskScheduler.Default).Wait(TimeSpan.FromSeconds(3));
        Limiter.Restore();
    }

    private void StartCounter()
    {
        if (!_inSession || _gameProcesses is not { } processes || CounterState != FpsCounterState.Ready)
        {
            return;
        }

        if (!Fps.StartCapture(processes))
        {
            LogCounterRefused(_logger, Fps.Status);
            return;
        }

        RefreshOverlay();
    }

    private void StopCounter()
    {
        CloseOverlay();
        Fps.StopCapture();
    }

    /// <summary>Mostra, atualiza ou fecha a janela do contador conforme o estilo; um tique por segundo.</summary>
    private void RefreshOverlay()
    {
        if (!_inSession || FpsStyle == FpsOverlay.Off || CounterState != FpsCounterState.Ready || Fps.Status is FpsCounterStatus.Idle or FpsCounterStatus.Unavailable)
        {
            CloseOverlay();
            return;
        }

        if (_overlayTimer is null)
        {
            _overlayTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
            _overlayTimer.Tick += (_, _) => TickOverlay();
            _overlayTimer.Start();
        }

        TickOverlay();
    }

    private void TickOverlay()
    {
        if (!_inSession || Fps.Status is FpsCounterStatus.Denied or FpsCounterStatus.Failed)
        {
            CloseOverlay();
            return;
        }

        // Com o Tako na frente (Central de Controle aberta), o contador sai de cena.
        if (Application.Current?.MainWindow is { IsActive: true })
        {
            _overlay?.Hide();
            return;
        }

        if (FpsOverlay.NeedsHardware(FpsStyle) && !_overlaySampling)
        {
            _ = SampleForOverlayAsync();
        }

        var frames = Fps.Current;
        var rows = FpsOverlay.Rows(FpsStyle, frames, FpsOverlay.NeedsHardware(FpsStyle) ? _overlaySample : null);
        _overlay ??= _createOverlay();
        _overlay?.Update(rows, Layout, GameWindow(frames));
        _overlay?.KeepOnTop();
    }

    private async Task SampleForOverlayAsync()
    {
        _overlaySampling = true;
        try
        {
            _overlaySample = await SampleAsync().ConfigureAwait(true) ?? _overlaySample;
        }
        finally
        {
            _overlaySampling = false;
        }
    }

    /// <summary>Janela principal do processo que está apresentando (para achar o monitor do jogo).</summary>
    private nint GameWindow(FrameStats? frames)
    {
        var candidates = new List<int>();
        if (frames is not null)
        {
            candidates.Add(frames.ProcessId);
        }

        candidates.AddRange(_gameProcesses?.Invoke() ?? []);
        foreach (int pid in candidates.Distinct())
        {
            try
            {
                using var process = Process.GetProcessById(pid);
                if (process.MainWindowHandle != 0)
                {
                    return process.MainWindowHandle;
                }
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException or Win32Exception)
            {
                // Processo fechou entre a lista e a leitura.
            }
        }

        return 0;
    }

    private void CloseOverlay()
    {
        _overlayTimer?.Stop();
        _overlayTimer = null;
        _overlay?.Close();
        _overlay = null;
        _overlaySample = null;
    }

    /// <summary>Aplica o limite escolhido ao jogo atual (fora da thread da UI); "Tela − 3" lê a taxa da tela na hora.</summary>
    private void ApplyLimitNow()
    {
        int option = FpsLimit;
        string? exe = _gameExe;
        Enqueue("aplicar o limite de FPS", () =>
        {
            int? refresh = option == FpsLimits.MatchDisplay ? ReadRefreshHz() : DisplayRefreshHz;
            DisplayRefreshHz = refresh;
            bool applied = FpsLimits.Resolve(option, refresh) is { } fps && Limiter.Apply(exe, fps);
            LimitCapabilities = Limiter.Capabilities;
            return applied;
        });
    }

    private void RestoreLimit() => Enqueue("devolver o limite de FPS do driver", () =>
    {
        Limiter.Restore();
        return true;
    });

    /// <summary>Taxa da tela principal (onde a tela de jogo põe o jogo); nula se não deu para ler.</summary>
    private int? ReadRefreshHz()
    {
        try
        {
            var displays = _displays.ListDisplays();
            var screen = displays.FirstOrDefault(d => d.IsActive && d.IsPrimary) ?? displays.FirstOrDefault(d => d.IsActive);
            return screen?.CurrentMode?.RefreshHz;
        }
        catch (RuntimeEnvironmentException ex)
        {
            LogQueueFailed(_logger, ex, "ler a taxa da tela");
            return null;
        }
    }

    private void Enqueue(string action, Func<bool> work)
    {
        lock (_queueGate)
        {
            _limitQueue = _limitQueue.ContinueWith(
                _ =>
                {
                    try
                    {
                        if (!work())
                        {
                            LogQueueRefused(_logger, action);
                        }
                    }
                    catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or InvalidOperationException or System.Runtime.InteropServices.ExternalException)
                    {
                        LogQueueFailed(_logger, ex, action);
                    }
                },
                CancellationToken.None,
                TaskContinuationOptions.None,
                TaskScheduler.Default);
        }
    }

    private void Save(string key, string value)
    {
        try
        {
            _store.Set(key, value);
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogPreferencesFailed(_logger, ex);
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Preferências da Central de Controle não puderam ser lidas ou salvas")]
    private static partial void LogPreferencesFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Information, Message = "Contador de FPS não começou: {Status}")]
    private static partial void LogCounterRefused(ILogger logger, FpsCounterStatus status);

    [LoggerMessage(Level = LogLevel.Information, Message = "Central de Controle: não foi possível {Action}")]
    private static partial void LogQueueRefused(ILogger logger, string action);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Central de Controle: falha ao {Action}")]
    private static partial void LogQueueFailed(ILogger logger, Exception exception, string action);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Leitura de desempenho falhou")]
    private static partial void LogHardwareFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Não foi possível abrir {Url}")]
    private static partial void LogOpenPageFailed(ILogger logger, Exception exception, string url);
}

/// <summary>
/// Último estado de um controle PlayStation (lido por HID na UI) para o ponteiro pelo controle, que roda em outra
/// thread e só lê XInput por conta própria. Amostra velha (controle desconectado) não vale.
/// </summary>
public sealed class PointerPadRelay
{
    private static readonly TimeSpan MaxAge = TimeSpan.FromMilliseconds(150);
    private readonly Stopwatch _clock = Stopwatch.StartNew();
    private volatile Box? _latest;

    public void Update(double leftX, double leftY, bool a) => _latest = new Box(new PointerPadSample(leftX, leftY, a), _clock.Elapsed);

    public PointerPadSample? Read() => _latest is { } box && _clock.Elapsed - box.At < MaxAge ? box.Sample : null;

    private sealed record Box(PointerPadSample Sample, TimeSpan At);
}
