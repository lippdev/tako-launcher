namespace ConsoleMode.Runtime.Ipc;

/// <summary>Conexão recém-negociada.</summary>
public sealed class IpcConnectionEventArgs(IpcConnection connection) : EventArgs
{
    public IpcConnection Connection { get; } = connection;
}

/// <summary>Cliente que não completou o handshake (versão incompatível, mensagem inválida, prazo).</summary>
public sealed class IpcHandshakeFailedEventArgs(Exception error) : EventArgs
{
    public Exception Error { get; } = error;
}

/// <summary>
/// Servidor IPC: aceita clientes em sequência, negocia a versão com cada um e mantém as conexões abertas até
/// <see cref="DisposeAsync"/>. Os tratadores de <see cref="IpcConnectionOptions.Handler"/> valem para todos os clientes.
/// </summary>
public sealed class IpcServer(IIpcServerTransport transport, IpcConnectionOptions options) : IAsyncDisposable
{
    private readonly CancellationTokenSource _stop = new();
    private readonly object _gate = new();
    private readonly List<IpcConnection> _connections = [];
    private Task? _acceptLoop;

    public event EventHandler<IpcConnectionEventArgs>? ClientConnected;

    public event EventHandler<IpcHandshakeFailedEventArgs>? HandshakeFailed;

    /// <summary>Conexões abertas agora.</summary>
    public IReadOnlyList<IpcConnection> Connections
    {
        get
        {
            lock (_gate)
            {
                return [.. _connections.Where(c => c.IsConnected)];
            }
        }
    }

    public void Start()
    {
        if (_acceptLoop is not null)
        {
            throw new InvalidOperationException("Servidor IPC já iniciado.");
        }

        _acceptLoop = Task.Run(AcceptLoopAsync);
    }

    public async ValueTask DisposeAsync()
    {
        await _stop.CancelAsync().ConfigureAwait(false);
        if (_acceptLoop is not null)
        {
            await _acceptLoop.ConfigureAwait(false);
        }

        IpcConnection[] open;
        lock (_gate)
        {
            open = [.. _connections];
            _connections.Clear();
        }

        foreach (var connection in open)
        {
            await connection.DisposeAsync().ConfigureAwait(false);
        }

        await transport.DisposeAsync().ConfigureAwait(false);
    }

    private async Task AcceptLoopAsync()
    {
        while (!_stop.IsCancellationRequested)
        {
            Stream stream;
            try
            {
                stream = await transport.AcceptAsync(_stop.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (_stop.IsCancellationRequested)
            {
                return;
            }
            catch (IOException ex)
            {
                // Cliente desistiu no meio da conexão; o servidor segue aceitando.
                HandshakeFailed?.Invoke(this, new IpcHandshakeFailedEventArgs(ex));
                continue;
            }

            _ = Task.Run(() => NegotiateAsync(stream));
        }
    }

    private async Task NegotiateAsync(Stream stream)
    {
        IpcConnection connection;
        try
        {
            connection = await IpcConnection.AcceptAsync(stream, options, _stop.Token).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is IpcException or IOException or TimeoutException or OperationCanceledException or ObjectDisposedException)
        {
            await stream.DisposeAsync().ConfigureAwait(false);
            if (ex is not OperationCanceledException)
            {
                HandshakeFailed?.Invoke(this, new IpcHandshakeFailedEventArgs(ex));
            }

            return;
        }

        lock (_gate)
        {
            if (!_stop.IsCancellationRequested)
            {
                _connections.RemoveAll(c => !c.IsConnected);
                _connections.Add(connection);
            }
        }

        if (_stop.IsCancellationRequested)
        {
            await connection.DisposeAsync().ConfigureAwait(false); // chegou durante o encerramento
            return;
        }

        ClientConnected?.Invoke(this, new IpcConnectionEventArgs(connection));
    }
}
