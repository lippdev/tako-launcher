using System.Text;

namespace Launcher.Core.Emulation;

/// <summary>Arquivo de firmware/BIOS citado no <c>.info</c> do core (<c>firmwareN_path</c>), relativo à pasta <c>system</c>.</summary>
/// <param name="Optional">O core roda sem ele (<c>firmwareN_opt = "true"</c>).</param>
public sealed record CoreFirmware(string Path, string Description, bool Optional);

/// <summary>Core do RetroArch instalado: arquivo em <c>cores\</c> e o que o <c>info\*.info</c> diz dele.</summary>
/// <param name="Id">Nome do arquivo sem "_libretro.dll" ("snes9x").</param>
/// <param name="CorePath">Caminho completo do .dll.</param>
/// <param name="Name">Nome curto ("Snes9x"), do <c>corename</c> ou da tabela de sugestões.</param>
/// <param name="Systems">Sistemas do Tako atendidos (pelo <c>database</c> do .info ou pela tabela).</param>
/// <param name="Extensions">Extensões aceitas (<c>supported_extensions</c>), minúsculas e com ponto.</param>
public sealed record RetroArchCore(string Id, string CorePath, string Name, IReadOnlyList<string> Systems, IReadOnlyList<string> Extensions, IReadOnlyList<CoreFirmware> Firmware);

/// <summary>Leitura dos arquivos do RetroArch: <c>.info</c> dos cores e <c>retroarch.cfg</c> (formato <c>chave = "valor"</c>). Puro, testado.</summary>
public static class RetroArch
{
    public const string CoreSuffix = "_libretro.dll";

    /// <summary>Lê <c>chave = "valor"</c> (comentários com #); a última ocorrência vence.</summary>
    public static IReadOnlyDictionary<string, string> ParseKeyValues(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        var values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (string raw in text.Split('\n'))
        {
            if (TryParseLine(raw, out string key, out string value))
            {
                values[key] = value;
            }
        }

        return values;
    }

    /// <summary>
    /// Cores em <c>&lt;RetroArch&gt;\cores</c> com os dados do <c>info\&lt;core&gt;_libretro.info</c> (o RetroArch instalado traz os .info;
    /// sem eles, a tabela de sugestões diz os sistemas). Core sem sistema conhecido fica de fora.
    /// </summary>
    public static IReadOnlyList<RetroArchCore> InstalledCores(IEmulationFileSystem disk, string retroArchFolder, EmulatorCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(disk);
        ArgumentNullException.ThrowIfNull(catalog);
        var cores = new List<RetroArchCore>();
        foreach (string file in disk.Files(WinPath.Combine(retroArchFolder, "cores")).Order(StringComparer.OrdinalIgnoreCase))
        {
            string name = WinPath.FileName(file);
            if (!name.EndsWith(CoreSuffix, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            string id = name[..^CoreSuffix.Length].ToLowerInvariant();
            string? info = disk.ReadText(WinPath.Combine(WinPath.Combine(retroArchFolder, "info"), id + "_libretro.info"));
            var core = Describe(id, file, info is null ? null : ParseKeyValues(info), catalog);
            if (core.Systems.Count > 0)
            {
                cores.Add(core);
            }
        }

        return cores;
    }

    /// <summary>Junta o .info (quando há) com a tabela de sugestões.</summary>
    public static RetroArchCore Describe(string id, string corePath, IReadOnlyDictionary<string, string>? info, EmulatorCatalog catalog)
    {
        ArgumentNullException.ThrowIfNull(catalog);
        var suggestion = catalog.RetroArchCores.FirstOrDefault(c => string.Equals(c.Core, id, StringComparison.OrdinalIgnoreCase));
        var systems = new List<string>();
        if (info is not null && info.TryGetValue("database", out string? databases))
        {
            foreach (string database in databases.Split('|', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
            {
                string? system = catalog.RetroArchDatabases.GetValueOrDefault(database)
                    ?? (database.StartsWith("MAME", StringComparison.OrdinalIgnoreCase) ? "arcade" : null);
                if (system is not null && !systems.Contains(system))
                {
                    systems.Add(system);
                }
            }
        }

        foreach (string system in suggestion?.Systems ?? [])
        {
            if (!systems.Contains(system))
            {
                systems.Add(system);
            }
        }

        var extensions = info?.GetValueOrDefault("supported_extensions") is { Length: > 0 } list
            ? list.Split('|', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries).Select(e => "." + e.TrimStart('.').ToLowerInvariant()).Distinct().ToList()
            : [];
        string name = suggestion?.Name ?? info?.GetValueOrDefault("corename") ?? id;
        return new RetroArchCore(id, corePath, name, systems, extensions, Firmware(info));
    }

    private static List<CoreFirmware> Firmware(IReadOnlyDictionary<string, string>? info)
    {
        var firmware = new List<CoreFirmware>();
        if (info is null || !int.TryParse(info.GetValueOrDefault("firmware_count"), System.Globalization.NumberStyles.Integer, System.Globalization.CultureInfo.InvariantCulture, out int count))
        {
            return firmware;
        }

        for (int i = 0; i < Math.Min(count, 64); i++)
        {
            if (info.GetValueOrDefault($"firmware{i}_path") is { Length: > 0 } path)
            {
                bool optional = string.Equals(info.GetValueOrDefault($"firmware{i}_opt"), "true", StringComparison.OrdinalIgnoreCase);
                firmware.Add(new CoreFirmware(path, info.GetValueOrDefault($"firmware{i}_desc") ?? path, optional));
            }
        }

        return firmware;
    }

    /// <summary>
    /// Pasta do <c>retroarch.cfg</c> em uso: ao lado do executável (instalador e versão portátil do Windows) ou
    /// <c>%APPDATA%\RetroArch</c>; nula se não há nenhum.
    /// </summary>
    public static string? ConfigFile(IEmulationFileSystem disk, string retroArchFolder, EmulationPaths paths)
    {
        ArgumentNullException.ThrowIfNull(disk);
        ArgumentNullException.ThrowIfNull(paths);
        string local = WinPath.Combine(retroArchFolder, "retroarch.cfg");
        if (disk.FileExists(local))
        {
            return local;
        }

        string roaming = WinPath.Combine(WinPath.Combine(paths.AppData, "RetroArch"), "retroarch.cfg");
        return disk.FileExists(roaming) ? roaming : null;
    }

    /// <summary>
    /// Pasta <c>system</c> (BIOS) do RetroArch: <c>system_directory</c> do cfg quando definido (":" = pasta do
    /// RetroArch), senão <c>&lt;RetroArch&gt;\system</c>.
    /// </summary>
    public static string SystemFolder(string retroArchFolder, IReadOnlyDictionary<string, string>? config)
    {
        string? configured = config?.GetValueOrDefault("system_directory");
        if (string.IsNullOrWhiteSpace(configured) || configured == "default")
        {
            return WinPath.Combine(retroArchFolder, "system");
        }

        return configured.StartsWith(':') ? WinPath.Resolve(retroArchFolder, configured.TrimStart(':', '\\', '/')) : configured;
    }

    /// <summary>
    /// Aplica as opções do Tako no texto do <c>retroarch.cfg</c> sem sobrescrever escolhas do usuário: só troca uma
    /// chave ausente ou com valor "não configurado" (<see cref="RetroArchSetting.Unset"/>). O resto do arquivo fica
    /// igual, byte a byte.
    /// </summary>
    public static RetroArchConfigChange Merge(string text, IEnumerable<RetroArchSetting> wanted)
    {
        ArgumentNullException.ThrowIfNull(text);
        ArgumentNullException.ThrowIfNull(wanted);
        var settings = wanted.ToList();
        var lines = text.Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n').ToList();
        if (lines.Count > 0 && lines[^1].Length == 0)
        {
            lines.RemoveAt(lines.Count - 1);
        }

        var applied = new List<string>();
        var kept = new List<string>();
        foreach (var setting in settings)
        {
            int index = lines.FindLastIndex(l => TryParseLine(l, out string key, out _) && string.Equals(key, setting.Key, StringComparison.OrdinalIgnoreCase));
            if (index < 0)
            {
                lines.Add(Line(setting));
                applied.Add(setting.Key);
                continue;
            }

            _ = TryParseLine(lines[index], out _, out string current);
            if (string.Equals(current, setting.Value, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (setting.Unset.Contains(current, StringComparer.OrdinalIgnoreCase))
            {
                lines[index] = Line(setting);
                applied.Add(setting.Key);
            }
            else
            {
                kept.Add(setting.Key); // o usuário escolheu outro valor: fica o dele
            }
        }

        string newline = text.Contains("\r\n", StringComparison.Ordinal) ? "\r\n" : "\n";
        var builder = new StringBuilder();
        foreach (string line in lines)
        {
            builder.Append(line).Append(newline);
        }

        return new RetroArchConfigChange(applied.Count == 0 ? text : builder.ToString(), applied, kept);
    }

    private static string Line(RetroArchSetting setting) => $"{setting.Key} = \"{setting.Value}\"";

    private static bool TryParseLine(string raw, out string key, out string value)
    {
        key = value = string.Empty;
        string line = raw.Trim();
        if (line.Length == 0 || line.StartsWith('#'))
        {
            return false;
        }

        int equals = line.IndexOf('=', StringComparison.Ordinal);
        if (equals <= 0)
        {
            return false;
        }

        key = line[..equals].Trim();
        value = line[(equals + 1)..].Trim();
        if (value.Length >= 2 && value[0] == '"' && value[^1] == '"')
        {
            value = value[1..^1];
        }

        return key.Length > 0;
    }
}

/// <summary>Resultado de <see cref="RetroArch.Merge"/>: o texto novo, as chaves aplicadas e as que ficaram com o valor do usuário.</summary>
public sealed record RetroArchConfigChange(string Text, IReadOnlyList<string> Applied, IReadOnlyList<string> KeptUserValue)
{
    public bool Changed => Applied.Count > 0;
}
