using System.Text.Json;
using System.Text.Json.Serialization;

namespace ConsoleMode.Runtime.Ipc;

/// <summary>Tipo de mensagem. <c>hello</c>/<c>welcome</c>/<c>reject</c> só aparecem no handshake.</summary>
public enum IpcMessageKind
{
    /// <summary>Cliente → servidor: faixa de versões aceita e identificação.</summary>
    Hello,

    /// <summary>Servidor → cliente: versão escolhida.</summary>
    Welcome,

    /// <summary>Servidor → cliente: conexão recusada (ex.: versão incompatível); o servidor fecha em seguida.</summary>
    Reject,

    /// <summary>Comando que espera resposta (<see cref="IpcEnvelope.Id"/> obrigatório).</summary>
    Request,

    /// <summary>Resposta a um comando, com o mesmo <see cref="IpcEnvelope.Id"/>; carga ou <see cref="IpcEnvelope.Error"/>.</summary>
    Response,

    /// <summary>Notificação sem resposta.</summary>
    Event,
}

/// <summary>Erro de protocolo ou de um comando, com código estável (<see cref="IpcErrorCodes"/>).</summary>
public sealed record IpcError(string Code, string Message);

/// <summary>
/// Envelope de toda mensagem. <see cref="Version"/> é o cabeçalho de versão: no handshake, a maior versão de quem envia;
/// depois dele, sempre a versão negociada (outra versão fecha a conexão).
/// </summary>
public sealed record IpcEnvelope
{
    [JsonPropertyName("v")]
    public required int Version { get; init; }

    [JsonPropertyName("kind")]
    public required IpcMessageKind Kind { get; init; }

    /// <summary>Correlação requisição/resposta, única por conexão e por lado.</summary>
    [JsonPropertyName("id")]
    public long? Id { get; init; }

    /// <summary>Nome do comando ou do evento (ex.: <c>ui.heartbeat</c>).</summary>
    [JsonPropertyName("method")]
    public string? Method { get; init; }

    [JsonPropertyName("payload")]
    public JsonElement? Payload { get; init; }

    [JsonPropertyName("error")]
    public IpcError? Error { get; init; }
}

/// <summary>Carga do <c>hello</c>: faixa de versões do cliente e quem ele é.</summary>
public sealed record IpcHello(int MinVersion, int MaxVersion, string Peer, int ProcessId);

/// <summary>Carga do <c>welcome</c>: versão escolhida e quem é o servidor.</summary>
public sealed record IpcWelcome(int Version, string Peer, int ProcessId);
