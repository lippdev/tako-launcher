using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace ConsoleMode.Runtime.Tv;

/// <summary>Resposta ao pedido de registro.</summary>
public enum WebOsRegisterState
{
    /// <summary>A TV está mostrando o aviso para aceitar este PC.</summary>
    Prompted,

    /// <summary>Aceito: a resposta traz a client-key.</summary>
    Registered,

    /// <summary>Recusado na TV (ou erro no registro).</summary>
    Denied,
}

public sealed record WebOsRegisterReply(WebOsRegisterState State, string? ClientKey, string? Error);

/// <summary>Resposta a um pedido <c>ssap://</c>. <see cref="Payload"/> é um clone (vive sem o documento).</summary>
public sealed record WebOsResponse(string? Id, bool Ok, JsonElement Payload, string? Error);

public sealed record WebOsInput(string Id, string Label);

/// <summary>Estado de energia (<c>getPowerState</c>); TVs antigas não respondem e ficam em <see cref="Unknown"/>.</summary>
public enum WebOsPower
{
    Unknown,
    Active,

    /// <summary>Tela apagada com a TV ligada (modo "só áudio").</summary>
    ScreenOff,

    /// <summary>Em espera com a rede ligada ("Sempre pronta" / "Ligar via Wi-Fi"): acorda com Wake-on-LAN.</summary>
    Standby,
}

/// <summary>
/// API local do webOS (LG): JSON por WebSocket em <c>wss://IP:3001</c> (ou <c>ws://IP:3000</c> nas TVs antigas). O
/// cliente se registra com um manifesto; na primeira vez a TV mostra um aviso e, aceito, devolve uma client-key que vale
/// para as próximas conexões. Protocolo como documentado publicamente por projetos abertos (PyWebOSTV, aiopylgtv, MIT).
/// </summary>
public static class WebOsProtocol
{
    public const int SecurePort = 3001;
    public const int PlainPort = 3000;
    public const string RegisterId = "register_0";
    public const string TurnOffUri = "ssap://system/turnOff";
    public const string SwitchInputUri = "ssap://tv/switchInput";
    public const string ExternalInputsUri = "ssap://tv/getExternalInputList";
    public const string PowerStateUri = "ssap://com.webos.service.tvpower/power/getPowerState";
    public const string TurnOnScreenUri = "ssap://com.webos.service.tvpower/power/turnOnScreen";

    /// <summary>
    /// Manifesto assinado do "LG Remote App" (dados públicos do LG Connect SDK, Apache-2.0, como usados pelo
    /// PyWebOSTV, MIT; ver NOTICE.md). A assinatura cobre só o bloco <c>signed</c>; as permissões pedidas ficam fora
    /// dele e são só as que o Tako usa: energia, entradas e estado.
    /// </summary>
    private const string Manifest = """
        {
          "manifestVersion": 1,
          "appVersion": "1.1",
          "signed": {
            "created": "20140509",
            "appId": "com.lge.test",
            "vendorId": "com.lge",
            "localizedAppNames": { "": "LG Remote App", "ko-KR": "리모컨 앱", "zxx-XX": "ЛГ Rэмotэ AПП" },
            "localizedVendorNames": { "": "LG Electronics" },
            "permissions": [ "TEST_SECURE", "CONTROL_INPUT_TEXT", "CONTROL_MOUSE_AND_KEYBOARD", "READ_INSTALLED_APPS", "READ_LGE_SDX", "READ_NOTIFICATIONS", "SEARCH", "WRITE_SETTINGS", "WRITE_NOTIFICATION_ALERT", "CONTROL_POWER", "READ_CURRENT_CHANNEL", "READ_RUNNING_APPS", "READ_UPDATE_INFO", "UPDATE_FROM_REMOTE_APP", "READ_LGE_TV_INPUT_EVENTS", "READ_TV_CURRENT_TIME" ],
            "serial": "2f930e2d2cfe083771f68e4fe7bb07"
          },
          "permissions": [ "CONTROL_POWER", "CONTROL_TV_POWER", "CONTROL_TV_SCREEN", "CONTROL_TV_STANBY", "CONTROL_INPUT_TV", "READ_INPUT_DEVICE_LIST", "READ_POWER_STATE", "READ_TV_CURRENT_TIME", "CONTROL_WOL" ],
          "signatures": [ {
            "signatureVersion": 1,
            "signature": "eyJhbGdvcml0aG0iOiJSU0EtU0hBMjU2Iiwia2V5SWQiOiJ0ZXN0LXNpZ25pbmctY2VydCIsInNpZ25hdHVyZVZlcnNpb24iOjF9.hrVRgjCwXVvE2OOSpDZ58hR+59aFNwYDyjQgKk3auukd7pcegmE2CzPCa0bJ0ZsRAcKkCTJrWo5iDzNhMBWRyaMOv5zWSrthlf7G128qvIlpMT0YNY+n/FaOHE73uLrS/g7swl3/qH/BGFG2Hu4RlL48eb3lLKqTt2xKHdCs6Cd4RMfJPYnzgvI4BNrFUKsjkcu+WD4OO2A27Pq1n50cMchmcaXadJhGrOqH5YmHdOCj5NSHzJYrsW0HPlpuAx/ECMeIZYDh6RMqaFM2DXzdKX9NmmyqzJ3o/0lkk/N97gfVRLW5hA29yeAwaCViZNCP8iC9aO0q9fQojoa7NQnAtw=="
          } ]
        }
        """;

    /// <summary>Endereços a tentar, nesta ordem: o cifrado (TVs de 2018 em diante só aceitam ele) e o antigo.</summary>
    public static IReadOnlyList<Uri> Endpoints(string host) =>
    [
        new UriBuilder("wss", host, SecurePort, "/").Uri,
        new UriBuilder("ws", host, PlainPort, "/").Uri,
    ];

    /// <summary>Pedido de registro; com a client-key salva a TV não pergunta de novo.</summary>
    public static string Register(string? clientKey)
    {
        var payload = new JsonObject
        {
            ["forcePairing"] = false,
            ["pairingType"] = "PROMPT",
            ["manifest"] = JsonNode.Parse(Manifest),
        };
        if (!string.IsNullOrEmpty(clientKey))
        {
            payload["client-key"] = clientKey;
        }

        return new JsonObject { ["type"] = "register", ["id"] = RegisterId, ["payload"] = payload }.ToJsonString();
    }

    public static string Request(string id, string uri, JsonObject? payload = null)
    {
        var message = new JsonObject { ["type"] = "request", ["id"] = id, ["uri"] = uri };
        if (payload is not null)
        {
            message["payload"] = payload;
        }

        return message.ToJsonString();
    }

    /// <summary>Id da entrada "HDMI n" no webOS (1 a 4).</summary>
    public static string InputId(int hdmi) => "HDMI_" + Math.Clamp(hdmi, 1, 4).ToString(CultureInfo.InvariantCulture);

    /// <summary>Resposta ao registro, ou nulo se a mensagem é sobre outra coisa.</summary>
    public static WebOsRegisterReply? ParseRegisterReply(string json)
    {
        using var document = Parse(json);
        var root = document.RootElement;
        if (Text(root, "id") is { } id && id != RegisterId)
        {
            return null;
        }

        string? type = Text(root, "type");
        root.TryGetProperty("payload", out var payload);
        return type switch
        {
            "registered" when payload.ValueKind == JsonValueKind.Object && Text(payload, "client-key") is { Length: > 0 } key => new(WebOsRegisterState.Registered, key, null),
            "registered" => new(WebOsRegisterState.Denied, null, "registro sem client-key"),
            "error" => new(WebOsRegisterState.Denied, null, Text(root, "error") ?? "erro"),
            "response" when payload.ValueKind == JsonValueKind.Object && Text(payload, "pairingType") == "PROMPT" => new(WebOsRegisterState.Prompted, null, null),
            _ => null,
        };
    }

    public static WebOsResponse ParseResponse(string json)
    {
        using var document = Parse(json);
        var root = document.RootElement;
        var payload = root.TryGetProperty("payload", out var p) ? p.Clone() : default;
        bool ok = Text(root, "type") == "response"
            && !(payload.ValueKind == JsonValueKind.Object && payload.TryGetProperty("returnValue", out var rv) && rv.ValueKind == JsonValueKind.False);
        string? error = Text(root, "error")
            ?? (payload.ValueKind == JsonValueKind.Object ? Text(payload, "errorText") : null);
        return new(Text(root, "id"), ok, payload, ok ? null : error ?? "erro");
    }

    /// <summary>Entradas externas de <c>getExternalInputList</c> (<c>devices[].id</c> e <c>label</c>).</summary>
    public static IReadOnlyList<WebOsInput> ParseInputs(JsonElement payload)
    {
        if (payload.ValueKind != JsonValueKind.Object || !payload.TryGetProperty("devices", out var devices) || devices.ValueKind != JsonValueKind.Array)
        {
            return [];
        }

        var inputs = new List<WebOsInput>();
        foreach (var device in devices.EnumerateArray())
        {
            if (device.ValueKind == JsonValueKind.Object && Text(device, "id") is { Length: > 0 } id)
            {
                inputs.Add(new(id, Text(device, "label") ?? id));
            }
        }

        return inputs;
    }

    public static WebOsPower ParsePowerState(JsonElement payload)
    {
        string? state = payload.ValueKind == JsonValueKind.Object ? Text(payload, "state") : null;
        return state switch
        {
            "Active" => WebOsPower.Active,
            "Screen Off" or "Screen Saver" => WebOsPower.ScreenOff,
            "Active Standby" or "Suspend" or "Power Off" => WebOsPower.Standby,
            _ => WebOsPower.Unknown,
        };
    }

    /// <summary>
    /// Entrada a usar: <c>HDMI_n</c> se a TV listou (ou se a lista veio vazia: TVs que não respondem a lista ainda
    /// aceitam o id). Se a TV listou entradas e não tem essa, erro com as que existem.
    /// </summary>
    public static string ChooseInput(IReadOnlyList<WebOsInput> inputs, int hdmi)
    {
        ArgumentNullException.ThrowIfNull(inputs);
        string wanted = InputId(hdmi);
        if (inputs.Count == 0 || inputs.Any(i => string.Equals(i.Id, wanted, StringComparison.OrdinalIgnoreCase)))
        {
            return wanted;
        }

        string available = string.Join(", ", inputs.Select(i => i.Label));
        throw new TvControlException($"A TV não tem a entrada HDMI {Math.Clamp(hdmi, 1, 4)}. Entradas da TV: {available}.");
    }

    private static JsonDocument Parse(string json)
    {
        try
        {
            return JsonDocument.Parse(json);
        }
        catch (JsonException ex)
        {
            throw new InvalidDataException("Resposta da TV LG ilegível", ex);
        }
    }

    private static string? Text(JsonElement element, string name) =>
        element.ValueKind == JsonValueKind.Object && element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;
}
