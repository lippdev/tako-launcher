using System.Text.Json;
using System.Text.Json.Serialization;

namespace ConsoleMode.Runtime.Ipc;

/// <summary>
/// Protocolo IPC local (MVP-003, ADR-0012): mensagens JSON com cabeçalho de versão, em quadros
/// <c>[tamanho Int32 little-endian][JSON UTF-8]</c>, sobre Named Pipes restritos ao usuário atual.
/// </summary>
public static class IpcProtocol
{
    /// <summary>Versão mais nova que este código fala. Mudou o contrato de forma incompatível: incremente.</summary>
    public const int CurrentVersion = 1;

    /// <summary>Versão mais antiga que este código ainda aceita.</summary>
    public const int MinimumVersion = 1;

    /// <summary>Limite de um quadro. Mensagem maior é erro de protocolo (o outro lado é dado não confiável).</summary>
    public const int MaxFrameBytes = 1024 * 1024;

    /// <summary>Serialização das mensagens e das cargas: camelCase, enums como texto, nulos omitidos.</summary>
    public static JsonSerializerOptions SerializerOptions { get; } = CreateOptions();

    /// <summary>
    /// Maior versão em comum entre as faixas dos dois lados, ou nulo quando não há nenhuma (a conexão é recusada com
    /// <see cref="IpcErrorCodes.VersionMismatch"/>).
    /// </summary>
    public static int? Negotiate(int localMin, int localMax, int remoteMin, int remoteMax)
    {
        if (localMin > localMax || remoteMin > remoteMax)
        {
            return null;
        }

        int best = Math.Min(localMax, remoteMax);
        return best >= Math.Max(localMin, remoteMin) ? best : null;
    }

    private static JsonSerializerOptions CreateOptions()
    {
        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        };
        options.Converters.Add(new JsonStringEnumConverter(JsonNamingPolicy.CamelCase));
        options.MakeReadOnly(populateMissingResolver: true);
        return options;
    }
}

/// <summary>Códigos de erro estáveis do protocolo (o texto da mensagem é só para diagnóstico).</summary>
public static class IpcErrorCodes
{
    /// <summary>As faixas de versão dos dois lados não se cruzam.</summary>
    public const string VersionMismatch = "version_mismatch";

    /// <summary>Mensagem fora do formato ou fora de ordem no handshake.</summary>
    public const string InvalidMessage = "invalid_message";

    /// <summary>Ninguém trata o comando pedido.</summary>
    public const string MethodNotFound = "method_not_found";

    /// <summary>O tratador do comando falhou.</summary>
    public const string HandlerFailed = "handler_failed";
}
