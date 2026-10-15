using System.Text.Json;

namespace ConsoleMode.Runtime.Ipc;

/// <summary>Quem atende comandos e eventos que chegam por uma <see cref="IpcConnection"/>.</summary>
public interface IIpcMessageHandler
{
    /// <summary>
    /// Atende um comando; o retorno vira a carga da resposta. Comandos rodam em paralelo (as respostas podem sair fora de
    /// ordem). Lance <see cref="IpcRemoteException"/> para responder com um código de erro específico.
    /// </summary>
    Task<object?> HandleRequestAsync(IpcConnection connection, string method, JsonElement? payload, CancellationToken cancellationToken);

    /// <summary>Recebe um evento. Eventos são entregues na ordem em que chegaram, um de cada vez.</summary>
    void HandleEvent(IpcConnection connection, string name, JsonElement? payload);
}

/// <summary>Tabela de tratadores por nome. Comando sem tratador responde <see cref="IpcErrorCodes.MethodNotFound"/>.</summary>
public sealed class IpcHandlerMap : IIpcMessageHandler
{
    private readonly Dictionary<string, Func<IpcConnection, JsonElement?, CancellationToken, Task<object?>>> _requests = new(StringComparer.Ordinal);
    private readonly Dictionary<string, Action<IpcConnection, JsonElement?>> _events = new(StringComparer.Ordinal);

    public IpcHandlerMap OnRequest(string method, Func<IpcConnection, JsonElement?, CancellationToken, Task<object?>> handler)
    {
        ArgumentException.ThrowIfNullOrEmpty(method);
        ArgumentNullException.ThrowIfNull(handler);
        _requests[method] = handler;
        return this;
    }

    /// <summary>Comando síncrono com resposta imediata.</summary>
    public IpcHandlerMap OnRequest(string method, Func<JsonElement?, object?> handler)
    {
        ArgumentNullException.ThrowIfNull(handler);
        return OnRequest(method, (_, payload, _) => Task.FromResult(handler(payload)));
    }

    public IpcHandlerMap OnEvent(string name, Action<IpcConnection, JsonElement?> handler)
    {
        ArgumentException.ThrowIfNullOrEmpty(name);
        ArgumentNullException.ThrowIfNull(handler);
        _events[name] = handler;
        return this;
    }

    public IpcHandlerMap OnEvent(string name, Action<JsonElement?> handler)
    {
        ArgumentNullException.ThrowIfNull(handler);
        return OnEvent(name, (_, payload) => handler(payload));
    }

    public Task<object?> HandleRequestAsync(IpcConnection connection, string method, JsonElement? payload, CancellationToken cancellationToken) =>
        _requests.TryGetValue(method, out var handler)
            ? handler(connection, payload, cancellationToken)
            : throw new IpcRemoteException(IpcErrorCodes.MethodNotFound, $"Comando IPC desconhecido: '{method}'.");

    /// <summary>Evento sem tratador é ignorado (versões novas podem emitir eventos que esta não conhece).</summary>
    public void HandleEvent(IpcConnection connection, string name, JsonElement? payload)
    {
        if (_events.TryGetValue(name, out var handler))
        {
            handler(connection, payload);
        }
    }
}

/// <summary>Configuração de uma ponta da conexão.</summary>
public sealed record IpcConnectionOptions
{
    /// <summary>Nome desta ponta, enviado no handshake (diagnóstico). Ex.: <c>tako-ui</c>.</summary>
    public string PeerName { get; init; } = "ipc";

    public int MinVersion { get; init; } = IpcProtocol.MinimumVersion;

    public int MaxVersion { get; init; } = IpcProtocol.CurrentVersion;

    /// <summary>Prazo padrão de um comando sem resposta.</summary>
    public TimeSpan RequestTimeout { get; init; } = TimeSpan.FromSeconds(5);

    /// <summary>Prazo do handshake: quem conecta e não se apresenta é desconectado.</summary>
    public TimeSpan HandshakeTimeout { get; init; } = TimeSpan.FromSeconds(5);

    public int MaxFrameBytes { get; init; } = IpcProtocol.MaxFrameBytes;

    public IIpcMessageHandler? Handler { get; init; }

    /// <summary>Falha em um tratador (comando ou evento). A conexão continua; quem configura decide como registrar.</summary>
    public Action<Exception>? HandlerError { get; init; }
}
