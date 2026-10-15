using System.Globalization;
using System.Text.RegularExpressions;

namespace Launcher.Core.Input;

/// <summary>
/// Vendor e product id tirados do caminho de interface HID do Windows, sem abrir o dispositivo (o diagnóstico do
/// sistema não pode disputar o controle com a leitura do launcher). USB: <c>\\?\hid#vid_054c&amp;pid_0ce6#...</c>;
/// Bluetooth: <c>\\?\hid#{00001124-...}_vid&amp;0002054c_pid&amp;0ce6#...</c>.
/// </summary>
public static partial class HidDevicePath
{
    public static bool TryParseIds(string path, out ushort vendorId, out ushort productId)
    {
        ArgumentNullException.ThrowIfNull(path);
        vendorId = productId = 0;
        var match = Ids().Match(path);
        return match.Success
            && ushort.TryParse(match.Groups["vid"].Value, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out vendorId)
            && ushort.TryParse(match.Groups["pid"].Value, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out productId);
    }

    /// <summary>
    /// Caminho de um dispositivo HID pelo Bluetooth: serviço HID clássico (<c>{00001124-...}</c>), HID sobre BLE
    /// (<c>{00001812-...}</c>) ou o vendor id com a fonte Bluetooth (<c>_vid&amp;0002054c</c>).
    /// </summary>
    public static bool IsBluetooth(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        return path.Contains("{00001124-0000-1000-8000-00805f9b34fb}", StringComparison.OrdinalIgnoreCase)
            || path.Contains("{00001812-0000-1000-8000-00805f9b34fb}", StringComparison.OrdinalIgnoreCase)
            || path.Contains("_vid&0002", StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>Modelos PlayStation suportados entre os caminhos, sem repetição (um controle expõe várias coleções HID).</summary>
    public static IReadOnlyList<SonyController> SonyControllers(IEnumerable<string> paths)
    {
        ArgumentNullException.ThrowIfNull(paths);
        var seen = new List<(SonyController Model, string Instance)>();
        foreach (string path in paths)
        {
            if (TryParseIds(path, out ushort vendor, out ushort product) && SonyHidReport.Identify(vendor, product) is { } model)
            {
                // Mesmo dispositivo físico = mesmo trecho de instância (entre o primeiro e o segundo '#').
                string[] parts = path.Split('#');
                string instance = parts.Length > 2 ? parts[2] : path;
                if (!seen.Contains((model, instance)))
                {
                    seen.Add((model, instance));
                }
            }
        }

        return [.. seen.Select(s => s.Model)];
    }

    [GeneratedRegex(@"vid[_&](?:0002)?(?<vid>[0-9a-f]{4})[_&]pid[_&](?<pid>[0-9a-f]{4})", RegexOptions.IgnoreCase | RegexOptions.CultureInvariant)]
    private static partial Regex Ids();
}
