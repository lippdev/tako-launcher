using System.Collections.Concurrent;
using System.Text.Json;

namespace ConsoleMode.Runtime.Ipc;

/// <summary>
/// Uma conexão IPC já negociada, simétrica: os dois lados enviam comandos (com resposta correlacionada pelo
/// <see cref="IpcEnvelope.Id"/>) e eventos. Respostas podem chegar em qualquer ordem; resposta que chega depois do
/// prazo é descartada. O transporte é qualquer <see cref="Stream"/> bidirecional (Named Pipe na produção, memória nos
/// testes). Crie com <see cref="ConnectAsync"/> (cliente) ou <see cref="AcceptAsync"/> (servidor).
/// </summary>
public sealed class IpcConnection : IAsyncDisposable
{
    private readonly Stream _stream;
    private readonly IpcConnectionOptions _options;
    private readonly SemaphoreSlim _writeLock = new(1, 1);
    private readonly ConcurrentDictionary<long, TaskCompletionSource<JsonElement?>> _pending = new();
    private readonly CancellationTokenSource _closing = new();
    private readonly TaskCompletionSource _closed = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private long _nextId;
    private long _lateResponses;
    private int _closeStarted;

    private IpcConnection(Stream stream, IpcConnectionOptions options, int version, string remotePeer, int remoteProcessId)
    {
        _stream = stream;
        _options = options;
        Version = version;
        RemotePeer = remotePeer;
        RemoteProcessId = remoteProcessId;
    }

    /// <summary>Versão negociada; todo quadro depois do handshake deve tê-la no cabeçalho.</summary>
    public int Version { get; }

    public string RemotePeer { get; }

    /// <summary>PID informado pelo outro lado no handshake (ex.: o atualizador espera o supervisor sair).</summary>
    public int RemoteProcessId { get; }

    /// <summary>Completa (sem falhar) quando a conexão fecha, por qualquer motivo.</summary>
    public Task Completion => _closed.Task;

    /// <summary>Motivo do fechamento; nulo se o outro lado fechou normalmente ou se <see cref="DisposeAsync"/> foi chamado.</summary>
    public Exception? CloseReason { get; private set; }

    public bool IsConnected => !_closed.Task.IsCompleted;

    /// <summary>Respostas recebidas depois do prazo (ou com <see cref="IpcEnvelope.Id"/> desconhecido) e descartadas.</summary>
    public long LateResponses => Interlocked.Read(ref _lateResponses);

    /// <summary>Lado cliente: envia <c>hello</c>, espera <c>welcome</c> e passa a ler mensagens.</summary>
    /// <exception cref="IpcVersionMismatchException">O servidor não aceita nenhuma versão desta faixa.</exception>
    public static async Task<IpcConnection> ConnectAsync(Stream stream, IpcConnectionOptions options, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(stream);
        ArgumentNullException.ThrowIfNull(options);
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(options.HandshakeTimeout);
        try
        {
            var hello = new IpcHello(options.MinVersion, options.MaxVersion, options.PeerName, Environment.ProcessId);
            await IpcFraming.WriteAsync(stream, new IpcEnvelope { Version = options.MaxVersion, Kind = IpcMessageKind.Hello, Payload = ToElement(hello) }, timeout.Token).ConfigureAwait(false);
            var reply = await IpcFraming.ReadAsync(stream, options.MaxFrameBytes, timeout.Token).ConfigureAwait(false)
                ?? throw new IpcDisconnectedException("O servidor IPC fechou a conexão durante o handshake.");
            switch (reply.Kind)
            {
                case IpcMessageKind.Welcome:
                    var welcome = FromElement<IpcWelcome>(reply.Payload) ?? throw new IpcProtocolException("Welcome IPC sem carga.");
                    if (welcome.Version < options.MinVersion || welcome.Version > options.MaxVersion)
                    {
                        throw new IpcVersionMismatchException(
                            $"O servidor IPC escolheu a versão {welcome.Version}, fora da faixa {options.MinVersion}–{options.MaxVersion} deste lado.");
                    }

                    return Start(stream, options, welcome.Version, welcome.Peer, welcome.ProcessId);
                case IpcMessageKind.Reject when reply.Error?.Code == IpcErrorCodes.VersionMismatch:
                    throw new IpcVersionMismatchException(reply.Error.Message);
                case IpcMessageKind.Reject:
                    throw new IpcRemoteException(reply.Error?.Code ?? IpcErrorCodes.InvalidMessage, reply.Error?.Message ?? "Conexão IPC recusada.");
                default:
                    throw new IpcProtocolException($"Esperava 'welcome' no handshake IPC e chegou '{reply.Kind}'.");
            }
        }
        catch (OperationCanceledException) when (timeout.IsCancellationRequested && !cancellationToken.IsCancellationRequested)
        {
            throw new TimeoutException($"Handshake IPC sem resposta em {options.HandshakeTimeout.TotalMilliseconds:0} ms.");
        }
    }

    /// <summary>
    /// Lado servidor: espera <c>hello</c>, escolhe a maior versão em comum e responde <c>welcome</c>; sem versão em
    /// comum, responde <c>reject</c> com <see cref="IpcErrorCodes.VersionMismatch"/> e lança
    /// <see cref="IpcVersionMismatchException"/>.
    /// </summary>
    public static async Task<IpcConnection> AcceptAsync(Stream stream, IpcConnectionOptions options, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(stream);
        ArgumentNullException.ThrowIfNull(options);
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(options.HandshakeTimeout);
        try
        {
            var first = await IpcFraming.ReadAsync(stream, options.MaxFrameBytes, timeout.Token).ConfigureAwait(false)
                ?? throw new IpcDisconnectedException("O cliente IPC fechou a conexão antes do handshake.");
            var hello = first.Kind == IpcMessageKind.Hello ? FromElement<IpcHello>(first.Payload) : null;
            if (hello is null)
            {
                await RejectAsync(stream, options, IpcErrorCodes.InvalidMessage, "A primeira mensagem IPC deve ser 'hello'.", timeout.Token).ConfigureAwait(false);
                throw new IpcProtocolException($"Esperava 'hello' no handshake IPC e chegou '{first.Kind}'.");
            }

            int? version = IpcProtocol.Negotiate(options.MinVersion, options.MaxVersion, hello.MinVersion, hello.MaxVersion);
            if (version is null)
            {
                string message =
                    $"Versão do protocolo IPC incompatível: '{options.PeerName}' aceita {options.MinVersion}–{options.MaxVersion} e "
                    + $"'{hello.Peer}' aceita {hello.MinVersion}–{hello.MaxVersion}. Os dois processos precisam ser da mesma versão do Tako.";
                await RejectAsync(stream, options, IpcErrorCodes.VersionMismatch, message, timeout.Token).ConfigureAwait(false);
                throw new IpcVersionMismatchException(message);
            }

            var welcome = new IpcWelcome(version.Value, options.PeerName, Environment.ProcessId);
            await IpcFraming.WriteAsync(stream, new IpcEnvelope { Version = version.Value, Kind = IpcMessageKind.Welcome, Payload = ToElement(welcome) }, timeout.Token).ConfigureAwait(false);
            return Start(stream, options, version.Value, hello.Peer, hello.ProcessId);
        }
        catch (OperationCanceledException) when (timeout.IsCancellationRequested && !cancellationToken.IsCancellationRequested)
        {
            throw new TimeoutException($"Cliente IPC não se apresentou em {options.HandshakeTimeout.TotalMilliseconds:0} ms.");
        }
    }

    /// <summary>Envia um comando e espera a resposta (correlacionada pelo id, em qualquer ordem).</summary>
    /// <exception cref="TimeoutException">Sem resposta no prazo; uma resposta atrasada é descartada.</exception>
    /// <exception cref="IpcRemoteException">O outro lado respondeu com erro.</exception>
    /// <exception cref="IpcDisconnectedException">A conexão caiu antes da resposta.</exception>
    public async Task<JsonElement?> RequestAsync(string method, object? payload = null, TimeSpan? timeout = null, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrEmpty(method);
        long id = Interlocked.Increment(ref _nextId);
        var reply = new TaskCompletionSource<JsonElement?>(TaskCreationOptions.RunContinuationsAsynchronously);
        _pending[id] = reply;
        try
        {
            if (!IsConnected)
            {
                throw new IpcDisconnectedException();
            }

            await SendAsync(new IpcEnvelope { Version = Version, Kind = IpcMessageKind.Request, Id = id, Method = method, Payload = ToElement(payload) }, cancellationToken).ConfigureAwait(false);
            var limit = timeout ?? _options.RequestTimeout;
            try
            {
                return await reply.Task.WaitAsync(limit, cancellationToken).ConfigureAwait(false);
            }
            catch (TimeoutException)
            {
                throw new TimeoutException($"Comando IPC '{method}' sem resposta em {limit.TotalMilliseconds:0} ms.");
            }
        }
        finally
        {
            _pending.TryRemove(id, out _);
        }
    }

    /// <summary>Envia um comando e converte a resposta.</summary>
    public async Task<TResult?> RequestAsync<TResult>(string method, object? payload = null, TimeSpan? timeout = null, CancellationToken cancellationToken = default)
    {
        var element = await RequestAsync(method, payload, timeout, cancellationToken).ConfigureAwait(false);
        return FromElement<TResult>(element);
    }

    /// <summary>Envia um evento (sem resposta).</summary>
    public Task SendEventAsync(string name, object? payload = null, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrEmpty(name);
        return SendAsync(new IpcEnvelope { Version = Version, Kind = IpcMessageKind.Event, Method = name, Payload = ToElement(payload) }, cancellationToken);
    }

    /// <summary>Converte a carga de uma mensagem.</summary>
    public static T? FromElement<T>(JsonElement? element)
    {
        if (element is not { } value || value.ValueKind is JsonValueKind.Null or JsonValueKind.Undefined)
        {
            return default;
        }

        try
        {
            return value.Deserialize<T>(IpcProtocol.SerializerOptions);
        }
        catch (JsonException ex)
        {
            throw new IpcProtocolException($"Carga IPC não confere com {typeof(T).Name}.", ex);
        }
    }

    public async ValueTask DisposeAsync()
    {
        Close(null);
        await _closed.Task.ConfigureAwait(false);
    }

    private static JsonElement? ToElement(object? payload) => payload switch
    {
        null => null,
        JsonElement element => element,
        _ => JsonSerializer.SerializeToElement(payload, payload.GetType(), IpcProtocol.SerializerOptions),
    };

    private static IpcConnection Start(Stream stream, IpcConnectionOptions options, int version, string remotePeer, int remoteProcessId)
    {
        var connection = new IpcConnection(stream, options, version, remotePeer, remoteProcessId);
        _ = Task.Run(connection.ReadLoopAsync);
        return connection;
    }

    private static async Task RejectAsync(Stream stream, IpcConnectionOptions options, string code, string message, CancellationToken cancellationToken)
    {
        try
        {
            await IpcFraming.WriteAsync(stream, new IpcEnvelope { Version = options.MaxVersion, Kind = IpcMessageKind.Reject, Error = new IpcError(code, message) }, cancellationToken).ConfigureAwait(false);
        }
        catch (IOException)
        {
            // O cliente já foi embora; a recusa é só cortesia.
        }
    }

    private async Task SendAsync(IpcEnvelope envelope, CancellationToken cancellationToken)
    {
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, _closing.Token);
        try
        {
            await _writeLock.WaitAsync(linked.Token).ConfigureAwait(false);
            try
            {
                await IpcFraming.WriteAsync(_stream, envelope, linked.Token).ConfigureAwait(false);
            }
            finally
            {
                _writeLock.Release();
            }
        }
        catch (OperationCanceledException) when (_closing.IsCancellationRequested && !cancellationToken.IsCancellationRequested)
        {
            throw new IpcDisconnectedException();
        }
        catch (Exception ex) when (ex is IOException or ObjectDisposedException)
        {
            Close(ex);
            throw new IpcDisconnectedException("Conexão IPC encerrada ao enviar.", ex);
        }
    }

    private async Task ReadLoopAsync()
    {
        Exception? reason = null;
        try
        {
            while (!_closing.IsCancellationRequested)
            {
                var envelope = await IpcFraming.ReadAsync(_stream, _options.MaxFrameBytes, _closing.Token).ConfigureAwait(false);
                if (envelope is null)
                {
                    break; // o outro lado fechou
                }

                Dispatch(envelope);
            }
        }
        catch (OperationCanceledException) when (_closing.IsCancellationRequested)
        {
        }
        catch (Exception ex) when (ex is IOException or ObjectDisposedException or IpcException)
        {
            reason = ex;
        }
        finally
        {
            Close(reason);
        }
    }

    private void Dispatch(IpcEnvelope envelope)
    {
        if (envelope.Version != Version)
        {
            throw new IpcProtocolException($"Mensagem IPC com versão {envelope.Version}; a conexão negociou a versão {Version}.");
        }

        switch (envelope.Kind)
        {
            case IpcMessageKind.Response when envelope.Id is { } id:
                if (!_pending.TryRemove(id, out var reply))
                {
                    Interlocked.Increment(ref _lateResponses); // prazo vencido: quem pediu já recebeu TimeoutException
                }
                else if (envelope.Error is { } error)
                {
                    reply.TrySetException(new IpcRemoteException(error.Code, error.Message));
                }
                else
                {
                    reply.TrySetResult(envelope.Payload);
                }

                break;
            case IpcMessageKind.Request when envelope.Id is { } requestId && envelope.Method is { Length: > 0 } method:
                _ = Task.Run(() => AnswerAsync(requestId, method, envelope.Payload));
                break;
            case IpcMessageKind.Event when envelope.Method is { Length: > 0 } name:
                try
                {
                    _options.Handler?.HandleEvent(this, name, envelope.Payload);
                }
                catch (Exception ex) when (ex is not OutOfMemoryException)
                {
                    // Limite do IPC: um tratador com defeito não derruba a conexão; quem configurou registra.
                    _options.HandlerError?.Invoke(ex);
                }

                break;
            default:
                throw new IpcProtocolException($"Mensagem IPC inesperada: '{envelope.Kind}' (id {envelope.Id?.ToString(System.Globalization.CultureInfo.InvariantCulture) ?? "-"}).");
        }
    }

    private async Task AnswerAsync(long id, string method, JsonElement? payload)
    {
        IpcEnvelope reply;
        try
        {
            if (_options.Handler is not { } handler)
            {
                throw new IpcRemoteException(IpcErrorCodes.MethodNotFound, $"Comando IPC desconhecido: '{method}'.");
            }

            object? result = await handler.HandleRequestAsync(this, method, payload, _closing.Token).ConfigureAwait(false);
            reply = new IpcEnvelope { Version = Version, Kind = IpcMessageKind.Response, Id = id, Payload = ToElement(result) };
        }
        catch (OperationCanceledException) when (_closing.IsCancellationRequested)
        {
            return; // conexão fechando: não há a quem responder
        }
        catch (IpcRemoteException ex)
        {
            reply = new IpcEnvelope { Version = Version, Kind = IpcMessageKind.Response, Id = id, Error = new IpcError(ex.Code, ex.Message) };
        }
        catch (Exception ex) when (ex is not OutOfMemoryException)
        {
            // Limite do IPC: o erro volta para quem pediu e fica com quem configurou a conexão.
            _options.HandlerError?.Invoke(ex);
            reply = new IpcEnvelope { Version = Version, Kind = IpcMessageKind.Response, Id = id, Error = new IpcError(IpcErrorCodes.HandlerFailed, ex.Message) };
        }

        try
        {
            await SendAsync(reply, CancellationToken.None).ConfigureAwait(false);
        }
        catch (IpcDisconnectedException)
        {
            // Quem pediu já foi embora.
        }
    }

    private void Close(Exception? reason)
    {
        if (Interlocked.Exchange(ref _closeStarted, 1) != 0)
        {
            return;
        }

        CloseReason = reason;
        _closing.Cancel();
        foreach (var pending in _pending.Values)
        {
            pending.TrySetException(new IpcDisconnectedException("Conexão IPC encerrada antes da resposta.", reason ?? new EndOfStreamException()));
        }

        _pending.Clear();
        try
        {
            _stream.Dispose();
        }
        catch (IOException)
        {
            // Pipe já quebrado.
        }

        _closed.TrySetResult();
    }
}
