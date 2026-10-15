using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace ConsoleMode.Runtime.Tv;

/// <summary>O que a TV Samsung informa em <c>http://IP:8001/api/v2/</c>.</summary>
/// <param name="PowerState">"on" ou "standby" (modelos de 2018 em diante); nulo nos mais antigos.</param>
/// <param name="TokenAuth">A TV usa o canal cifrado com token (porta 8002).</param>
public sealed record SamsungInfo(string? Name, string? Model, string? PowerState, bool TokenAuth, bool IsSamsung)
{
    public bool IsStandby => string.Equals(PowerState, "standby", StringComparison.OrdinalIgnoreCase);
}

public enum SamsungEventKind
{
    /// <summary>Conectado (e autorizado); pode trazer o token novo.</summary>
    Connected,

    /// <summary>Recusado na TV (ou "Acesso de dispositivos IP remotos" negou).</summary>
    Unauthorized,

    /// <summary>Ninguém respondeu ao aviso na TV a tempo.</summary>
    TimedOut,

    /// <summary>Outros eventos do canal (ex.: <c>ms.channel.ready</c>).</summary>
    Other,
}

public sealed record SamsungChannelEvent(SamsungEventKind Kind, string? Token);

/// <summary>
/// API de controle remoto das TVs Samsung com Tizen (2016 em diante): teclas do controle por WebSocket em
/// <c>wss://IP:8002/api/v2/channels/samsung.remote.control?name=…&amp;token=…</c>. Na primeira vez a TV pergunta se
/// permite o dispositivo e, aceito, devolve um token. Protocolo como documentado publicamente por projetos abertos.
/// </summary>
public static class SamsungTvProtocol
{
    public const int PlainPort = 8001;
    public const int SecurePort = 8002;
    public const string KeyPower = "KEY_POWER";

    public static Uri InfoUri(string host) => new UriBuilder("http", host, PlainPort, "/api/v2/").Uri;

    /// <summary>Canal de teclas; o nome aparece na TV em base64, como a API pede.</summary>
    public static Uri RemoteUri(string host, string name, string? token, bool secure)
    {
        string query = "name=" + Uri.EscapeDataString(Convert.ToBase64String(Encoding.UTF8.GetBytes(name)));
        if (!string.IsNullOrEmpty(token))
        {
            query += "&token=" + Uri.EscapeDataString(token);
        }

        return new UriBuilder(secure ? "wss" : "ws", host, secure ? SecurePort : PlainPort, "/api/v2/channels/samsung.remote.control") { Query = query }.Uri;
    }

    /// <summary>Tecla da entrada "HDMI n" (1 a 4).</summary>
    public static string HdmiKey(int hdmi) => "KEY_HDMI" + Math.Clamp(hdmi, 1, 4).ToString(CultureInfo.InvariantCulture);

    /// <summary>Um toque de tecla do controle remoto.</summary>
    public static string Key(string key) => new JsonObject
    {
        ["method"] = "ms.remote.control",
        ["params"] = new JsonObject
        {
            ["Cmd"] = "Click",
            ["DataOfCmd"] = key,
            ["Option"] = "false",
            ["TypeOfRemote"] = "SendRemoteKey",
        },
    }.ToJsonString();

    /// <summary>Informação da TV; nulo se não for JSON de uma TV Samsung.</summary>
    public static SamsungInfo? ParseInfo(string json)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty("device", out var device) || device.ValueKind != JsonValueKind.Object)
            {
                return null;
            }

            string? type = Text(root, "type") ?? Text(device, "type");
            bool isSamsung = (type?.Contains("Samsung", StringComparison.OrdinalIgnoreCase) ?? false)
                || string.Equals(Text(device, "OS"), "Tizen", StringComparison.OrdinalIgnoreCase);
            return new SamsungInfo(
                Text(device, "name") ?? Text(root, "name"),
                Text(device, "modelName"),
                Text(device, "PowerState"),
                !string.Equals(Text(device, "TokenAuthSupport"), "false", StringComparison.OrdinalIgnoreCase),
                isSamsung);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    public static SamsungChannelEvent ParseEvent(string json)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            var root = document.RootElement;
            string? name = root.ValueKind == JsonValueKind.Object ? Text(root, "event") : null;
            string? token = null;
            if (root.ValueKind == JsonValueKind.Object && root.TryGetProperty("data", out var data) && data.ValueKind == JsonValueKind.Object
                && data.TryGetProperty("token", out var t))
            {
                // Algumas TVs mandam o token como número.
                token = t.ValueKind switch
                {
                    JsonValueKind.String => t.GetString(),
                    JsonValueKind.Number => t.GetRawText(),
                    _ => null,
                };
            }

            return name switch
            {
                "ms.channel.connect" => new(SamsungEventKind.Connected, string.IsNullOrEmpty(token) ? null : token),
                "ms.channel.unauthorized" => new(SamsungEventKind.Unauthorized, null),
                "ms.channel.timeOut" => new(SamsungEventKind.TimedOut, null),
                _ => new(SamsungEventKind.Other, null),
            };
        }
        catch (JsonException ex)
        {
            throw new InvalidDataException("Resposta da TV Samsung ilegível", ex);
        }
    }

    private static string? Text(JsonElement element, string name) =>
        element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;
}
