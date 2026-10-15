using System.Text.Json;

namespace ConsoleMode.Runtime.Ipc;

/// <summary>Estado do <see cref="IpcClient"/>.</summary>
public enum IpcClientState
{
    Connecting,
    Connected,

    /// <summary>Versão incompatível: o cliente parou de tentar (reconectar não resolve).</summary>
    Faulted,
    Disposed,
}

/// <summary>Configuração do cliente com reconexão.</summary>
public sealed record IpcClientOptions
{
    public IpcConnectionOptions Connection { get; init; } = new();

    /// <summary>Primeira espera entre tentativas; dobra a cada falha até <see cref="MaxReconnectDelay"/>.</summary>
    public TimeSpan ReconnectDelay { get; init; } = TimeSpan.FromMilliseconds(200);

    public TimeSpan MaxReconnectDelay { get; init; } = TimeSpan.FromSeconds(5);
}

/// <summary>
/// Cliente IPC que se mantém conectado: conecta, negocia a versão e, quando a conexão cai (servidor reiniciou, pipe
/// quebrou), tenta de novo com espera crescente. Versão incompatível para as tentativas (<see cref="IpcClientState.Faulted"/>).
/// Quem precisa refazer estado no servidor depois de uma reconexão escuta <see cref="Connected"/>.
/// </summary>
public sealed class IpcClient(IIpcClientTransport transport, IpcClientOptions options) : IAsyncDisposable
{
    private readonly CancellationTokenSource _stop = new();
    private readonly object _gate = new();
    private TaskCompletionSource<IpcConnection> _ready = NewReady();
    private IpcConnection? _current;
    private Task? _loop;

    /// <summary>A cada conexão (a primeira e cada reconexão), já negociada.</summary>
    public event EventHandler<IpcConnectionEventArgs>? Connected;

    /// <summary>A conexão atual caiu; o cliente já está tentando reconectar.</summary>
    public event EventHandler? Disconnected;

    public IpcClientState State { get; private set; } = IpcClientState.Connecting;

    /// <summary>Por que o cliente parou (<see cref="IpcClientState.Faulted"/>).</summary>
    public Exception? Fault { get; private set; }

    /// <summary>Última falha de conexão (diagnóstico); limpa ao conectar.</summary>
    public Exception? LastError { get; private set; }

    /// <summary>Conexões feitas até agora (1 = primeira; 2+ = reconexões).</summary>
    public int ConnectionCount { get; private set; }

    /// <summary>Conexão aberta agora, ou nula.</summary>
    public IpcConnection? Current
    {
        get
        {
            lock (_gate)
            {
                return _current is { IsConnected: true } current ? current : null;
            }
        }
    }

    public void Start()
    {
        if (_loop is not null)
        {
            throw new InvalidOperationException("Cliente IPC já iniciado.");
        }

        _loop = Task.Run(RunAsync);
    }

    /// <summary>Espera haver conexão aberta (até <paramref name="timeout"/>); ignora a conexão que acabou de cair.</summary>
    /// <exception cref="IpcVersionMismatchException">O servidor recusou a versão deste cliente.</exception>
    /// <exception cref="TimeoutException">Sem conexão no prazo.</exception>
    public async Task<IpcConnection> WaitForConnectionAsync(TimeSpan timeout, CancellationToken cancellationToken = default)
    {
        var deadline = DateTime.UtcNow + timeout;
        while (true)
        {
            Task<IpcConnection> ready;
            lock (_gate)
            {
                ready = _ready.Task;
            }

            var remaining = deadline - DateTime.UtcNow;
            try
            {
                var connection = await ready.WaitAsync(remaining > TimeSpan.Zero ? remaining : TimeSpan.Zero, cancellationToken).ConfigureAwait(false);
                if (connection.IsConnected)
                {
                    return connection;
                }

                // Caiu e o laço ainda não trocou a espera: aguarda a troca.
                await Task.Delay(10, cancellationToken).ConfigureAwait(false);
            }
            catch (TimeoutException ex)
            {
                throw new TimeoutException($"Sem conexão IPC em {timeout.TotalMilliseconds:0} ms.", LastError ?? ex);
            }
        }
    }

    /// <summary>Comando com espera pela conexão: se ela está caindo e voltando, o prazo cobre a reconexão.</summary>
    public async Task<JsonElement?> RequestAsync(string method, object? payload = null, TimeSpan? timeout = null, CancellationToken cancellationToken = default)
    {
        var limit = timeout ?? options.Connection.RequestTimeout;
        var connection = await WaitForConnectionAsync(limit, cancellationToken).ConfigureAwait(false);
        return await connection.RequestAsync(method, payload, limit, cancellationToken).ConfigureAwait(false);
    }

    public async Task<TResult?> RequestAsync<TResult>(string method, object? payload = null, TimeSpan? timeout = null, CancellationToken cancellationToken = default) =>
        IpcConnection.FromElement<TResult>(await RequestAsync(method, payload, timeout, cancellationToken).ConfigureAwait(false));

    /// <summary>Evento na conexão atual. Sem conexão, falso (eventos não ficam em fila: o próximo substitui).</summary>
    public async Task<bool> TrySendEventAsync(string name, object? payload = null, CancellationToken cancellationToken = default)
    {
        if (Current is not { } connection)
        {
            return false;
        }

        try
        {
            await connection.SendEventAsync(name, payload, cancellationToken).ConfigureAwait(false);
            return true;
        }
        catch (IpcDisconnectedException)
        {
            return false;
        }
    }

    public async ValueTask DisposeAsync()
    {
        State = IpcClientState.Disposed;
        await _stop.CancelAsync().ConfigureAwait(false);
        if (_loop is not null)
        {
            await _loop.ConfigureAwait(false);
        }

        if (Current is { } connection)
        {
            await connection.DisposeAsync().ConfigureAwait(false);
        }

        lock (_gate)
        {
            _ready.TrySetException(new ObjectDisposedException(nameof(IpcClient)));
        }
    }

    private static TaskCompletionSource<IpcConnection> NewReady() => new(TaskCreationOptions.RunContinuationsAsynchronously);

    private async Task RunAsync()
    {
        var delay = options.ReconnectDelay;
        while (!_stop.IsCancellationRequested)
        {
            IpcConnection? connection = null;
            try
            {
                var stream = await transport.ConnectAsync(_stop.Token).ConfigureAwait(false);
                try
                {
                    connection = await IpcConnection.ConnectAsync(stream, options.Connection, _stop.Token).ConfigureAwait(false);
                }
                catch
                {
                    await stream.DisposeAsync().ConfigureAwait(false);
                    throw;
                }
            }
            catch (OperationCanceledException) when (_stop.IsCancellationRequested)
            {
                return;
            }
            catch (IpcVersionMismatchException ex)
            {
                Fault = ex;
                State = IpcClientState.Faulted;
                lock (_gate)
                {
                    _ready.TrySetException(ex);
                }

                return;
            }
            catch (Exception ex) when (ex is IOException or TimeoutException or IpcException or UnauthorizedAccessException or ObjectDisposedException)
            {
                LastError = ex; // servidor ausente ou reiniciando: tenta de novo
            }

            if (connection is not null)
            {
                delay = options.ReconnectDelay;
                LastError = null;
                lock (_gate)
                {
                    _current = connection;
                    ConnectionCount++;
                    State = IpcClientState.Connected;
                    _ready.TrySetResult(connection);
                }

                Connected?.Invoke(this, new IpcConnectionEventArgs(connection));
                await connection.Completion.WaitAsync(_stop.Token).ConfigureAwait(ConfigureAwaitOptions.SuppressThrowing);
                if (_stop.IsCancellationRequested)
                {
                    return;
                }

                lock (_gate)
                {
                    _current = null;
                    State = IpcClientState.Connecting;
                    _ready = NewReady();
                }

                Disconnected?.Invoke(this, EventArgs.Empty); // espera só o intervalo inicial: o servidor costuma voltar rápido
            }

            try
            {
                await Task.Delay(delay, _stop.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                return;
            }

            delay = TimeSpan.FromTicks(Math.Min(delay.Ticks * 2, options.MaxReconnectDelay.Ticks));
        }
    }
}
