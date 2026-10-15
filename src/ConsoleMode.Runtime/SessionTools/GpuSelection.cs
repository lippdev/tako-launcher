using System.Globalization;

namespace ConsoleMode.Runtime.SessionTools;

public enum GpuVendor
{
    Unknown,
    Nvidia,
    Amd,
    Intel,
}

public static class GpuVendors
{
    public const uint NvidiaPciId = 0x10DE;
    public const uint AmdPciId = 0x1002;
    public const uint AmdAltPciId = 0x1022;
    public const uint IntelPciId = 0x8086;

    /// <summary>Adaptador "Microsoft Basic Render Driver" (WARP): não é placa de vídeo.</summary>
    public const uint MicrosoftPciId = 0x1414;

    public static GpuVendor FromPciId(uint vendorId) => vendorId switch
    {
        NvidiaPciId => GpuVendor.Nvidia,
        AmdPciId or AmdAltPciId => GpuVendor.Amd,
        IntelPciId => GpuVendor.Intel,
        _ => GpuVendor.Unknown,
    };

    public static string Name(GpuVendor vendor) => vendor switch
    {
        GpuVendor.Nvidia => "NVIDIA",
        GpuVendor.Amd => "AMD",
        GpuVendor.Intel => "Intel",
        _ => "de vídeo",
    };
}

/// <summary>Uma placa de vídeo como o DXGI lista (ordem do DXGI: a primeira é a da tela principal).</summary>
public sealed record GpuAdapter(string Name, uint VendorId, ulong DedicatedVideoMemory, bool IsSoftware)
{
    public GpuVendor Vendor => GpuVendors.FromPciId(VendorId);

    public override string ToString() => string.Create(CultureInfo.InvariantCulture, $"{Name} ({DedicatedVideoMemory / (1024 * 1024)} MB)");
}

/// <summary>Preferência de GPU do Windows para um exe (Configurações › Sistema › Tela › Gráficos).</summary>
public enum GpuPreference
{
    /// <summary>"Deixar o Windows decidir" ou sem escolha.</summary>
    Default,
    PowerSaving,
    HighPerformance,
}

/// <summary>Qual placa roda o jogo. Puro: a lista de placas e a preferência entram como parâmetros.</summary>
public static class GpuSelection
{
    /// <summary>Chave do registro (HKCU) com a preferência por exe: valor = caminho do exe, dado = <c>GpuPreference=2;</c>.</summary>
    public const string PreferencesKey = @"Software\Microsoft\DirectX\UserGpuPreferences";

    /// <summary>Lê o dado do registro (<c>GpuPreference=1;</c> = economia, <c>2</c> = alto desempenho).</summary>
    public static GpuPreference ParsePreference(string? registryValue)
    {
        if (string.IsNullOrWhiteSpace(registryValue))
        {
            return GpuPreference.Default;
        }

        foreach (string part in registryValue.Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            string[] pair = part.Split('=', 2, StringSplitOptions.TrimEntries);
            if (pair.Length == 2 && pair[0].Equals("GpuPreference", StringComparison.OrdinalIgnoreCase)
                && int.TryParse(pair[1], NumberStyles.Integer, CultureInfo.InvariantCulture, out int value))
            {
                return value switch
                {
                    1 => GpuPreference.PowerSaving,
                    2 => GpuPreference.HighPerformance,
                    _ => GpuPreference.Default,
                };
            }
        }

        return GpuPreference.Default;
    }

    /// <summary>
    /// A placa que deve rodar o jogo, entre as de verdade (sem WARP nem adaptadores de software):
    /// <list type="bullet">
    /// <item>só uma: ela (o caso de PCs com uma placa e portáteis como Steam Deck/ROG Ally);</item>
    /// <item>"economia de energia": a de menos memória dedicada (a integrada);</item>
    /// <item>"alto desempenho" ou sem escolha: a de mais memória dedicada (a dedicada), como os drivers fazem com jogos.</item>
    /// </list>
    /// Empate fica com a que vem primeiro na ordem do DXGI (a da tela principal). Nenhuma: nulo.
    /// </summary>
    public static GpuAdapter? Choose(IReadOnlyList<GpuAdapter> adapters, GpuPreference preference)
    {
        ArgumentNullException.ThrowIfNull(adapters);
        var real = adapters.Where(a => !a.IsSoftware && a.VendorId != GpuVendors.MicrosoftPciId).ToList();
        if (real.Count <= 1)
        {
            return real.FirstOrDefault();
        }

        // OrderBy é estável: no empate vale a ordem do DXGI.
        return preference == GpuPreference.PowerSaving
            ? real.OrderBy(a => a.DedicatedVideoMemory).First()
            : real.OrderByDescending(a => a.DedicatedVideoMemory).First();
    }

    /// <summary>
    /// Qual item da lista do driver (pelo nome) corresponde à placa do DXGI: o nome igual, senão um que contenha o outro,
    /// senão o primeiro. Lista vazia: -1.
    /// </summary>
    public static int MatchByName(IReadOnlyList<string?> driverNames, string adapterName)
    {
        ArgumentNullException.ThrowIfNull(driverNames);
        if (driverNames.Count == 0)
        {
            return -1;
        }

        string wanted = adapterName?.Trim() ?? string.Empty;
        for (int i = 0; i < driverNames.Count; i++)
        {
            if (string.Equals(driverNames[i]?.Trim(), wanted, StringComparison.OrdinalIgnoreCase))
            {
                return i;
            }
        }

        for (int i = 0; i < driverNames.Count; i++)
        {
            string name = driverNames[i]?.Trim() ?? string.Empty;
            if (name.Length > 0 && wanted.Length > 0
                && (name.Contains(wanted, StringComparison.OrdinalIgnoreCase) || wanted.Contains(name, StringComparison.OrdinalIgnoreCase)))
            {
                return i;
            }
        }

        return 0;
    }
}
