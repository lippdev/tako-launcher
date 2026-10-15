// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/DisplayIdentity.cs
using System.Text;

namespace ConsoleMode.Runtime.Display;

/// <summary>O que o EDID do monitor diz sobre ele: nome, número de série e resolução nativa (preferida).</summary>
public sealed record EdidInfo(string Name, string Serial, int PreferredWidth, int PreferredHeight)
{
    public static EdidInfo Empty { get; } = new(string.Empty, string.Empty, 0, 0);
}

/// <summary>
/// Identidade de monitores e escolha de fontes de vídeo, sem chamadas ao sistema (testável fora do Windows). Os valores
/// são os mesmos que o Console Mode grava, para as configurações serem compatíveis.
/// </summary>
public static class DisplayIdentity
{
    /// <summary>
    /// <c>\\?\DISPLAY#GSM5B7F#5&amp;2d8e1c7&amp;0&amp;UID4352#{e6f07b5f-…}</c> → identificador de hardware
    /// <c>GSM5B7F</c> e instância do dispositivo <c>DISPLAY\GSM5B7F\5&amp;2d8e1c7&amp;0&amp;UID4352</c>.
    /// </summary>
    public static bool TryParseDevicePath(string? devicePath, out string hardwareId, out string instanceId)
    {
        hardwareId = instanceId = string.Empty;
        if (string.IsNullOrWhiteSpace(devicePath))
        {
            return false;
        }

        string path = devicePath.StartsWith(@"\\?\", StringComparison.Ordinal) ? devicePath[4..] : devicePath;
        string[] parts = path.Split('#');
        if (parts.Length < 3 || parts[0].Length == 0 || parts[1].Length == 0 || parts[2].Length == 0)
        {
            return false;
        }

        hardwareId = parts[1];
        instanceId = $@"{parts[0]}\{parts[1]}\{parts[2]}";
        return true;
    }

    /// <summary>
    /// Identificador do monitor no formato do Console Mode (o ID de dispositivo que o Windows dá ao monitor), ex.:
    /// <c>MONITOR\GSM5B7F\{4d36e96e-e325-11ce-bfc1-08002be10318}\0004</c>; a última parte é a chave do driver.
    /// </summary>
    public static string MonitorId(string hardwareId, string? driverKey) =>
        string.IsNullOrWhiteSpace(hardwareId) || string.IsNullOrWhiteSpace(driverKey) ? string.Empty : $@"MONITOR\{hardwareId}\{driverKey}";

    /// <summary>Descritores de nome (0xFC) e número de série (0xFF) e a resolução preferida (nativa) do EDID.</summary>
    public static EdidInfo ParseEdid(byte[]? edid)
    {
        if (edid is null || edid.Length < 128)
        {
            return EdidInfo.Empty;
        }

        string name = string.Empty;
        string serial = string.Empty;
        int width = 0;
        int height = 0;
        for (int offset = 54; offset <= 108; offset += 18)
        {
            if (edid[offset] != 0 || edid[offset + 1] != 0)
            {
                // Temporização detalhada; a primeira é o modo preferido.
                if (width == 0)
                {
                    width = edid[offset + 2] | ((edid[offset + 4] & 0xF0) << 4);
                    height = edid[offset + 5] | ((edid[offset + 7] & 0xF0) << 4);
                }

                continue;
            }

            string text = DescriptorText(edid, offset);
            switch (edid[offset + 3])
            {
                case 0xFC when name.Length == 0:
                    name = text;
                    break;
                case 0xFF when serial.Length == 0:
                    serial = text;
                    break;
            }
        }

        return new EdidInfo(name, serial, width, height);
    }

    /// <summary>
    /// Escolhe uma fonte de vídeo para cada monitor. Os ativos mantêm a sua; cada inativo recebe a preferida (a última
    /// que usou) se estiver livre, senão a primeira candidata livre. Assim dois monitores nunca dividem um nome
    /// (<c>\\.\DISPLAYn</c>) e um monitor religado volta com o nome antigo.
    /// </summary>
    public static Dictionary<TTarget, TSource> AssignSources<TTarget, TSource>(
        IEnumerable<(TTarget Target, TSource? Active, TSource? Preferred, IReadOnlyList<TSource> Candidates)> targets)
        where TTarget : notnull
        where TSource : struct
    {
        ArgumentNullException.ThrowIfNull(targets);
        var list = targets.ToList();
        var result = new Dictionary<TTarget, TSource>();
        var used = new HashSet<TSource>();
        foreach (var t in list)
        {
            if (t.Active is { } active)
            {
                result[t.Target] = active;
                used.Add(active);
            }
        }

        foreach (var t in list.Where(t => t.Active is null))
        {
            TSource? pick = t.Preferred is { } preferred && t.Candidates.Contains(preferred) && !used.Contains(preferred) ? preferred : null;
            pick ??= t.Candidates.Where(c => !used.Contains(c)).Select(c => (TSource?)c).FirstOrDefault();
            if (pick is not { } chosen)
            {
                continue;
            }

            result[t.Target] = chosen;
            used.Add(chosen);
        }

        return result;
    }

    private static string DescriptorText(byte[] edid, int offset)
    {
        var sb = new StringBuilder();
        for (int i = offset + 5; i < offset + 18; i++)
        {
            if (edid[i] is 0x0A or 0)
            {
                break;
            }

            sb.Append((char)edid[i]);
        }

        return sb.ToString().Trim();
    }
}
