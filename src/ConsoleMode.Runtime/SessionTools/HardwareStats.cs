// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/HardwareStats.cs
using System.Globalization;
using System.Text.RegularExpressions;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Uma leitura do PC para a Central de Controle e o contador de FPS. Nulo = indisponível nesta máquina.</summary>
public sealed record HardwareSample
{
    public double? CpuUsage { get; init; }

    public double? CpuClockMhz { get; init; }

    public double? GpuUsage { get; init; }

    public double? GpuClockMhz { get; init; }

    public double? GpuTempC { get; init; }

    public double? VramUsedMb { get; init; }

    public double? VramTotalMb { get; init; }

    public double? RamUsedMb { get; init; }

    public double? RamTotalMb { get; init; }

    /// <summary>RAM em uso em porcentagem, quando o total é conhecido.</summary>
    public double? RamUsage => RamUsedMb is { } used && RamTotalMb is > 0 ? Math.Clamp(used / RamTotalMb.Value * 100, 0, 100) : null;
}

/// <summary>
/// Lê as instâncias dos contadores "GPU Engine" / "GPU Adapter Memory" como o Gerenciador de Tarefas: o uso de um
/// adaptador é o do seu motor mais ocupado, cada motor somado em todos os processos.
/// </summary>
public static partial class GpuCounters
{
    /// <summary>
    /// <paramref name="engines"/>: instâncias de "\GPU Engine(*)\Utilization Percentage", ex.:
    /// pid_1234_luid_0x00000000_0x0000D1E4_phys_0_eng_0_engtype_3D. <paramref name="dedicated"/>:
    /// "\GPU Adapter Memory(*)\Dedicated Usage" (luid_…_phys_0), o desempate quando toda GPU está parada.
    /// </summary>
    public static GpuAdapterUsage? Busiest(IEnumerable<(string Instance, double Value)> engines, IEnumerable<(string Instance, double Value)>? dedicated = null)
    {
        ArgumentNullException.ThrowIfNull(engines);
        var perEngine = new Dictionary<(string Key, uint Node), (double Sum, bool Is3D)>();
        foreach (var (instance, value) in engines)
        {
            var m = EngineInstance().Match(instance);
            if (!m.Success)
            {
                continue;
            }

            var key = (AdapterKey(m), uint.Parse(m.Groups["node"].Value, CultureInfo.InvariantCulture));
            bool is3D = string.Equals(m.Groups["type"].Value, "3D", StringComparison.OrdinalIgnoreCase);
            perEngine[key] = perEngine.TryGetValue(key, out var e) ? (e.Sum + value, e.Is3D || is3D) : (value, is3D);
        }

        if (perEngine.Count == 0)
        {
            return null;
        }

        var memory = new Dictionary<string, double>(StringComparer.OrdinalIgnoreCase);
        foreach (var (instance, value) in dedicated ?? [])
        {
            var m = AdapterInstance().Match(instance);
            if (m.Success)
            {
                memory[AdapterKey(m)] = value;
            }
        }

        return perEngine.GroupBy(e => e.Key.Key)
            .Select(g =>
            {
                // O clock vem do motor 3D (o que os jogos usam); sem ele, do primeiro motor.
                uint node = g.OrderByDescending(e => e.Value.Is3D).ThenBy(e => e.Key.Node).First().Key.Node;
                var m = AdapterInstance().Match(g.Key);
                return new GpuAdapterUsage(g.Key, Hex(m.Groups["low"].Value), (int)Hex(m.Groups["high"].Value), node, Math.Clamp(g.Max(e => e.Value.Sum), 0, 100));
            })
            .OrderByDescending(a => a.Usage)
            .ThenByDescending(a => memory.GetValueOrDefault(a.Key))
            .First();
    }

    /// <summary>"\GPU Adapter Memory(*)\Dedicated Usage" de um adaptador, em MB.</summary>
    public static double? DedicatedMb(IEnumerable<(string Instance, double Value)> dedicated, string adapterKey)
    {
        ArgumentNullException.ThrowIfNull(dedicated);
        foreach (var (instance, value) in dedicated)
        {
            var m = AdapterInstance().Match(instance);
            if (m.Success && string.Equals(AdapterKey(m), adapterKey, StringComparison.OrdinalIgnoreCase))
            {
                return value / (1024 * 1024);
            }
        }

        return null;
    }

    private static string AdapterKey(Match m) =>
        $"luid_0x{m.Groups["high"].Value}_0x{m.Groups["low"].Value}_phys_{m.Groups["phys"].Value}".ToLowerInvariant();

    private static uint Hex(string value) => uint.Parse(value, NumberStyles.HexNumber, CultureInfo.InvariantCulture);

    [GeneratedRegex(@"luid_0x(?<high>[0-9A-Fa-f]{8})_0x(?<low>[0-9A-Fa-f]{8})_phys_(?<phys>\d+)_eng_(?<node>\d+)_engtype_(?<type>.*)$")]
    private static partial Regex EngineInstance();

    [GeneratedRegex(@"luid_0x(?<high>[0-9A-Fa-f]{8})_0x(?<low>[0-9A-Fa-f]{8})_phys_(?<phys>\d+)")]
    private static partial Regex AdapterInstance();
}

/// <summary>A GPU em que o jogo roda (a mais ocupada) e o nó do motor de onde ler o clock.</summary>
public readonly record struct GpuAdapterUsage(string Key, uint LuidLow, int LuidHigh, uint Node, double Usage);
