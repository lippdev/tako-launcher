using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Windows.SessionTools;
using Launcher.Core.Persistence;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Ferramentas da Central de Controle durante o jogo: janelas abertas, contador e limite de FPS (RTSS), ponteiro,
/// ControlFS e desempenho do PC. Guarda as escolhas do contador e do limite no <c>tako.db</c>, aplica-as no início
/// de cada sessão e desfaz tudo no fim (ponteiro visível, limite do usuário de volta). Chamadas ao RTSS rodam em fila
/// fora da thread da UI: abri-lo leva até 1,5 s.
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
    private Task _rtssQueue = Task.CompletedTask;
    private bool _inSession;

    public SessionToolsService(
        IWindowSwitcher windows,
        IFpsOverlayService fps,
        IPointerService cursor,
        IControlFsService controlFs,
        IHardwareMonitor hardware,
        SettingsStore store,
        ILogger<SessionToolsService> logger)
    {
        Windows = windows;
        Fps = fps;
        Cursor = cursor;
        ControlFs = controlFs;
        Hardware = hardware;
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

    /// <summary>Estilo do contador (<see cref="RtssOverlay.Styles"/>); "external" deixa o RTSS/Afterburner como estão.</summary>
    public string FpsStyle { get; private set; } = RtssOverlay.External;

    public FpsOverlayLayout Layout { get; private set; } = new();

    /// <summary>Limite de FPS da sessão (0 = sem limite, o do usuário fica valendo).</summary>
    public int FpsLimit { get; private set; }

    /// <summary>Início da sessão de jogo: contador e limite escolhidos voltam a valer.</summary>
    public void BeginSession()
    {
        _inSession = true;
        if (!Fps.IsInstalled)
        {
            return;
        }

        string style = FpsStyle;
        var layout = Layout;
        int limit = FpsLimit;
        if (style != RtssOverlay.External)
        {
            Enqueue("mostrar o contador de FPS", () => Fps.ApplyOverlay(style, layout));
        }

        if (limit > 0)
        {
            Enqueue("aplicar o limite de FPS", () => Fps.SetFpsLimit(limit));
        }
    }

    /// <summary>Fim da sessão (ou do app): tira o contador, devolve o limite do usuário, mostra o ponteiro.</summary>
    public void EndSession()
    {
        _inSession = false;
        Cursor.Reset();
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
        if (_inSession && Fps.CanLimitFps)
        {
            int limit = FpsLimit;
            Enqueue("aplicar o limite de FPS", () => Fps.SetFpsLimit(limit));
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
        if (Fps.IsInstalled)
        {
            Task pending;
            lock (_queueGate)
            {
                pending = _rtssQueue;
            }

            // Fechando: termina a fila (as falhas dela já foram registradas) e devolve o RTSS antes de o processo sair.
            pending.ContinueWith(_ => { }, TaskScheduler.Default).Wait(TimeSpan.FromSeconds(3));
            Fps.Restore();
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

    [LoggerMessage(Level = LogLevel.Information, Message = "RTSS: não foi possível {Action}")]
    private static partial void LogRtssRefused(ILogger logger, string action);

    [LoggerMessage(Level = LogLevel.Warning, Message = "RTSS: falha ao {Action}")]
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
