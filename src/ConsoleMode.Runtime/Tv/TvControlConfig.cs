// Adaptado de lippdev/consolemode: src/ConsoleMode/Models/Models.cs (TvControlConfig)
using System.Text.Json;
using System.Text.Json.Serialization;

namespace ConsoleMode.Runtime.Tv;

/// <summary>
/// Controle da TV durante a sessão: ligar e trocar para a entrada do PC no início e, se pedido, colocar em espera no
/// fim. Desligado por padrão (<see cref="Enabled"/> falso).
/// </summary>
public sealed record TvControlConfig
{
    public static TvControlConfig Default { get; } = new();

    /// <summary>Ligar a TV e trocar a entrada ao abrir um jogo.</summary>
    public bool Enabled { get; init; }

    /// <summary>Como falar com a TV (padrão: Google TV / Android TV por ADB, o único tipo antes de LG e Samsung).</summary>
    public TvKind Kind { get; init; } = TvKind.Android;

    /// <summary>IP da TV na rede. No Android aceita nome e ":porta" opcional (padrão 5555); LG e Samsung usam o IP.</summary>
    public string Host { get; init; } = string.Empty;

    /// <summary>Endereço MAC para o Wake-on-LAN quando a TV está em espera profunda; vazio = não enviar.</summary>
    public string MacAddress { get; init; } = string.Empty;

    /// <summary>Entrada HDMI onde o PC está ligado (1 a 4).</summary>
    public int HdmiInput { get; init; } = 1;

    /// <summary>Só Android: comando de shell que troca para a entrada do PC, para TVs que ignoram a tecla HDMI.</summary>
    public string InputCommand { get; init; } = string.Empty;

    /// <summary>Colocar a TV em espera quando o jogo termina. Desligado por padrão.</summary>
    public bool StandbyAfterGame { get; init; }

    /// <summary>Ligada e com endereço: há o que fazer.</summary>
    [JsonIgnore]
    public bool IsActive => Enabled && !string.IsNullOrWhiteSpace(Host);

    /// <summary>Comando que troca para a entrada do PC (o personalizado ou a tecla HDMI escolhida).</summary>
    [JsonIgnore]
    public string InputSwitchCommand => string.IsNullOrWhiteSpace(InputCommand)
        ? $"input keyevent {AdbProtocol.HdmiKey(HdmiInput)}"
        : InputCommand.Trim();
}

/// <summary>Formato JSON da configuração (guardada como texto nas configurações do Tako).</summary>
public static class TvControlConfigJson
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        Converters = { new JsonStringEnumConverter() },
    };

    public static string Serialize(TvControlConfig config) => JsonSerializer.Serialize(config, Options);

    /// <summary>Texto vazio ou ilegível devolve o padrão (desligado).</summary>
    public static TvControlConfig Parse(string? json)
    {
        if (string.IsNullOrWhiteSpace(json))
        {
            return TvControlConfig.Default;
        }

        try
        {
            var config = JsonSerializer.Deserialize<TvControlConfig>(json, Options) ?? TvControlConfig.Default;
            return config with
            {
                Host = config.Host ?? string.Empty,
                MacAddress = config.MacAddress ?? string.Empty,
                InputCommand = config.InputCommand ?? string.Empty,
                HdmiInput = Math.Clamp(config.HdmiInput, 1, 4),
                Kind = Enum.IsDefined(config.Kind) ? config.Kind : TvKind.Android,
            };
        }
        catch (JsonException)
        {
            return TvControlConfig.Default;
        }
    }
}

/// <summary>Falha que pode ser mostrada ao usuário como está (texto em português, sem jargão).</summary>
public sealed class TvControlException : Exception
{
    public TvControlException()
    {
    }

    public TvControlException(string message)
        : base(message)
    {
    }

    public TvControlException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
