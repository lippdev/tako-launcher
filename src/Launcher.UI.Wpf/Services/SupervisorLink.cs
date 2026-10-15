using System.IO;
using System.Windows;
using System.Windows.Threading;
using ConsoleMode.Runtime.Ipc;
using Launcher.Core.Supervision;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Lado da UI do supervisor (MVP-006): conecta ao pipe recebido em <c>--supervisor-pipe</c>, manda heartbeats a partir
/// da thread da UI (se ela trava, eles param e o supervisor reinicia a UI) e anuncia saídas de propósito (atualização,
/// fim de sessão do Windows) para o supervisor não reabrir a UI. Sem o argumento (<c>dotnet run</c>, testes,
/// <c>--no-supervisor</c>) fica inativo.
/// </summary>
public sealed partial class SupervisorLink(ILogger<SupervisorLink> logger) : IDisposable
{
    private static readonly TimeSpan AnnounceTimeout = TimeSpan.FromSeconds(1);
    private IpcClient? _client;
    private DispatcherTimer? _timer;
    private long _sequence;

    /// <summary>A UI foi aberta pelo supervisor.</summary>
    public bool IsSupervised => _client is not null;

    /// <summary>PID do supervisor (do handshake): o atualizador portátil espera ele sair também.</summary>
    public int? SupervisorProcessId { get; private set; }

    /// <summary>Estado informado pelo supervisor na última conexão (diagnóstico).</summary>
    public SupervisorStatus? Status { get; private set; }

    /// <summary>Liga o vínculo se a linha de comando trouxer o pipe do supervisor.</summary>
    public void Start(IReadOnlyList<string> args, Application application, TimeSpan? heartbeatInterval = null)
    {
        ArgumentNullException.ThrowIfNull(application);
        if (_client is not null || SupervisorArguments.PipeName(args) is not { } pipe)
        {
            return;
        }

        _client = new IpcClient(new NamedPipeClientTransport(pipe), new IpcClientOptions
        {
            Connection = new IpcConnectionOptions { PeerName = SupervisorMessages.UiPeer },
        });
        _client.Connected += OnConnected;
        _client.Start();

        // DispatcherTimer: o heartbeat só sai se a thread da UI estiver respondendo.
        _timer = new DispatcherTimer(heartbeatInterval ?? new SupervisorOptions().HeartbeatInterval, DispatcherPriority.Normal, (_, _) => Beat(), application.Dispatcher);
        _timer.Start();
        application.SessionEnding += (_, _) => AnnounceClosing(UiCloseReason.SessionEnding);
        LogLinked(logger, pipe);
    }

    /// <summary>
    /// Avisa que a UI vai sair de propósito: o supervisor sai junto em vez de reabri-la. Espera no máximo
    /// <see cref="AnnounceTimeout"/>; sem supervisor, não faz nada.
    /// </summary>
    public void AnnounceClosing(UiCloseReason reason)
    {
        if (_client?.Current is not { } connection)
        {
            return;
        }

        try
        {
            connection.SendEventAsync(SupervisorMessages.Closing, new UiClosing(Environment.ProcessId, reason)).Wait(AnnounceTimeout);
        }
        catch (AggregateException ex) when (ex.InnerException is IpcException or IOException)
        {
            // Supervisor já foi embora: nada a avisar.
        }
    }

    public void Dispose()
    {
        _timer?.Stop();
        if (_client is { } client)
        {
            client.Connected -= OnConnected;
            client.DisposeAsync().AsTask().Wait(AnnounceTimeout);
        }
    }

    private void Beat()
    {
        var beat = new UiHeartbeat(Environment.ProcessId, Interlocked.Increment(ref _sequence));
        _ = _client?.TrySendEventAsync(SupervisorMessages.Heartbeat, beat);
    }

    private void OnConnected(object? sender, IpcConnectionEventArgs e)
    {
        SupervisorProcessId = e.Connection.RemoteProcessId;
        _ = QueryStatusAsync(e.Connection);
    }

    private async Task QueryStatusAsync(IpcConnection connection)
    {
        try
        {
            Status = await connection.RequestAsync<SupervisorStatus>(SupervisorMessages.Status).ConfigureAwait(false);
            if (Status is { Restarts: > 0 } status)
            {
                LogRestarted(logger, status.Restarts, status.Mode);
            }
        }
        catch (Exception ex) when (ex is IpcException or TimeoutException)
        {
            LogStatusFailed(logger, ex.Message);
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "UI ligada ao supervisor pelo pipe {Pipe}")]
    private static partial void LogLinked(ILogger logger, string pipe);

    [LoggerMessage(Level = LogLevel.Warning, Message = "UI reaberta pelo supervisor ({Restarts} reinícios; modo {Mode})")]
    private static partial void LogRestarted(ILogger logger, int restarts, UiLaunchMode mode);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Supervisor não respondeu ao status: {Reason}")]
    private static partial void LogStatusFailed(ILogger logger, string reason);
}
