using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Windows.SessionTools;
using Launcher.Core.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Ferramentas da Central de Controle durante o jogo: janelas abertas, contador de FPS (RTSS), limite de FPS (driver de
/// vídeo, ADR-0018), ponteiro, ControlFS e desempenho do PC. Guarda as escolhas do contador e do limite no <c>tako.db</c>,
/// aplica-as no início de cada sessão e desfaz tudo no fim (ponteiro visível, limite do usuário de volta). Chamadas ao
/// RTSS e ao driver rodam em fila fora da thread da UI: abrir o RTSS leva até 1,5 s e o driver, centenas de ms.
/// </summary>
public sealed partial class SessionToolsService : IDisposable
{
    private const string StyleKey = "ControlCenter.FpsStyle";
    private const string PositionKey = "ControlCenter.FpsPosition";
    private const string SizeKey = "ControlCenter.FpsSize";
    private const string CardKey = "ControlCenter.FpsCard";
    private const string LimitKey = "ControlCenter.FpsLimit";

    private readonly SettingsStore _store;
    private readonly ILogger<SessionToolsService> _logger;
    private readonly object _queueGate = new();
    private readonly IDisplayControlService _displays;
    private Task _rtssQueue = Task.CompletedTask;
    private bool _inSession;
    private string? _gameExe;

    public SessionToolsService(
        IWindowSwitcher windows,
        IFpsOverlayService fps,
        IPointerService cursor,
        IControlFsService controlFs,
        IHardwareMonitor hardware,
        IFrameRateLimiter limiter,
        IDisplayControlService displays,
        SettingsStore store,
        ILogger<SessionToolsService> logger)
    {
        Windows = windows;
        Fps = fps;
        Cursor = cursor;
        ControlFs = controlFs;
        Hardware = hardware;
        Limiter = limiter;
        _displays = displays;
        _store = store;
        _logger = logger;
        try
        {
            var saved = store.LoadAll();
            FpsStyle = RtssOverlay.Normalize(saved.GetValueOrDefault(StyleKey));
            Layout = new FpsOverlayLayout
            {
                Position = RtssOverlay.NormalizePosition(saved.GetValueOrDefault(PositionKey)),
                Size = RtssOverlay.NormalizeSize(saved.GetValueOrDefault(SizeKey)),
                Card = !bool.TryParse(saved.GetValueOrDefault(CardKey), out bool card) || card,
            };
            FpsLimit = int.TryParse(saved.GetValueOrDefault(LimitKey), NumberStyles.Integer, CultureInfo.InvariantCulture, out int limit) ? FpsLimits.Normalize(limit) : 0;
        }
        catch (Exception ex) when (ex is SqliteException or IOException or UnauthorizedAccessException)
        {
            LogPreferencesFailed(logger, ex);
        }
    }

    public IWindowSwitcher Windows { get; }

    public IFpsOverlayService Fps { get; }

    public IPointerService Cursor { get; }

    public IControlFsService ControlFs { get; }

    public IHardwareMonitor Hardware { get; }

    /// <summary>Limite de FPS pelo driver de vídeo (NVIDIA, AMD ou Intel), sem RTSS.</summary>
    public IFrameRateLimiter Limiter { get; }

    /// <summary>O que o driver da placa oferece; nulo enquanto a primeira sondagem (em segundo plano) não terminou.</summary>
    public FrameRateLimitCapabilities? LimitCapabilities { get; private set; }

    /// <summary>Taxa da tela principal na última leitura (para "Tela − 3"); nula se desconhecida.</summary>
    public int? DisplayRefreshHz { get; private set; }

    /// <summary>Estilo do contador (<see cref="RtssOverlay.Styles"/>); "external" deixa o RTSS/Afterburner como estão.</summary>
    public string FpsStyle { get; private set; } = RtssOverlay.External;

    public FpsOverlayLayout Layout { get; private set; } = new();

    /// <summary>Limite de FPS da sessão (0 = sem limite, o do usuário fica valendo; <see cref="FpsLimits.MatchDisplay"/> = tela − 3).</summary>
    public int FpsLimit { get; private set; }

    /// <summary>
    /// Início da sessão de jogo: contador e limite escolhidos voltam a valer. <paramref name="gameExePath"/> é o exe do
    /// jogo quando já se sabe (atalho direto para .exe); senão chega por <see cref="GameExecutableFound"/>.
    /// </summary>
    public void BeginSession(string? gameExePath = null)
    {
        _inSession = true;
        _gameExe = gameExePath;
        if (FpsLimit != 0)
        {
            ApplyLimitNow();
        }

        if (!Fps.IsInstalled)
        {
            return;
        }

        string style = FpsStyle;
        var layout = Layout;
        if (style != RtssOverlay.External)
        {
            Enqueue("mostrar o contador de FPS", () => Fps.ApplyOverlay(style, layout));
        }
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

    /// <summary>Fim da sessão (ou do app): tira o contador, devolve o limite do usuário, mostra o ponteiro.</summary>
    public void EndSession()
    {
        _inSession = false;
        _gameExe = null;
        Cursor.Reset();
        RestoreLimit();
        if (Fps.IsInstalled)
        {
            Enqueue("restaurar o RTSS", () =>
            {
                Fps.Restore();
                return true;
            });
        }
    }

    /// <summary>Abertura do Tako: desfaz o que uma execução que caiu deixou (ponteiro escondido, limite de FPS).</summary>
    public void RecoverAfterCrash()
    {
        Cursor.RecoverAfterCrash();
        RestoreLimit();
        Enqueue("ler o limite de FPS do driver", () =>
        {
            LimitCapabilities = Limiter.Capabilities;
            DisplayRefreshHz = ReadRefreshHz();
            return true;
        });
        if (Fps.IsInstalled)
        {
            Enqueue("restaurar o RTSS", () =>
            {
                Fps.Restore();
                return true;
            });
        }
    }

    public void SetFpsStyle(string style)
    {
        FpsStyle = RtssOverlay.Normalize(style);
        Save(StyleKey, FpsStyle);
        ApplyOverlayNow();
    }

    public void SetLayout(FpsOverlayLayout layout)
    {
        ArgumentNullException.ThrowIfNull(layout);
        Layout = layout;
        Save(PositionKey, layout.Position);
        Save(SizeKey, layout.Size);
        Save(CardKey, layout.Card ? "true" : "false");
        ApplyOverlayNow();
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

    /// <summary>Abre uma página no navegador padrão (ex.: download do RTSS ou do ControlFS).</summary>
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
        Task pending;
        lock (_queueGate)
        {
            pending = _rtssQueue;
        }

        // Fechando: termina a fila (as falhas dela já foram registradas) e devolve RTSS e driver antes de o processo sair.
        pending.ContinueWith(_ => { }, TaskScheduler.Default).Wait(TimeSpan.FromSeconds(3));
        Limiter.Restore();
        if (Fps.IsInstalled)
        {
            Fps.Restore();
        }
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
            LogRtssFailed(_logger, ex, "ler a taxa da tela");
            return null;
        }
    }

    private void ApplyOverlayNow()
    {
        if (!_inSession || !Fps.IsInstalled)
        {
            return;
        }

        string style = FpsStyle;
        var layout = Layout;
        Enqueue("mostrar o contador de FPS", () => Fps.ApplyOverlay(style, layout));
    }

    private void Enqueue(string action, Func<bool> work)
    {
        lock (_queueGate)
        {
            _rtssQueue = _rtssQueue.ContinueWith(
                _ =>
                {
                    try
                    {
                        if (!work())
                        {
                            LogRtssRefused(_logger, action);
                        }
                    }
                    catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or InvalidOperationException or System.Runtime.InteropServices.ExternalException)
                    {
                        LogRtssFailed(_logger, ex, action);
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

    [LoggerMessage(Level = LogLevel.Information, Message = "Central de Controle: não foi possível {Action}")]
    private static partial void LogRtssRefused(ILogger logger, string action);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Central de Controle: falha ao {Action}")]
    private static partial void LogRtssFailed(ILogger logger, Exception exception, string action);

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
