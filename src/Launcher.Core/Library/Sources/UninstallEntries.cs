namespace Launcher.Core.Library.Sources;

/// <summary>Um programa de "Aplicativos instalados" do Windows (chave <c>Uninstall</c> do registro).</summary>
public sealed record UninstallEntry(
    string KeyPath,
    string KeyName,
    string? DisplayName,
    string? Publisher,
    string? InstallLocation,
    string? UninstallString,
    string? DisplayIcon);

/// <summary>
/// Programas instalados, lidos das chaves <c>Uninstall</c> (64 e 32 bits da máquina e do usuário). Ubisoft Connect,
/// Battle.net e Amazon Games registram ali os jogos e o próprio cliente (ADR-0019).
/// </summary>
public static class UninstallEntries
{
    public static readonly IReadOnlyList<string> Roots =
    [
        @"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        @"HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    public static IReadOnlyList<UninstallEntry> Read(IRegistryReader registry)
    {
        ArgumentNullException.ThrowIfNull(registry);
        var entries = new List<UninstallEntry>();
        foreach (string root in Roots)
        {
            foreach (string name in registry.SubKeyNames(root))
            {
                entries.Add(Read(registry, $@"{root}\{name}", name));
            }
        }

        return entries;
    }

    /// <summary>Uma chave pelo nome (ex.: <c>Uplay Install 635</c>), na primeira raiz em que existir.</summary>
    public static UninstallEntry? Find(IRegistryReader registry, string keyName)
    {
        ArgumentNullException.ThrowIfNull(registry);
        foreach (string root in Roots)
        {
            var entry = Read(registry, $@"{root}\{keyName}", keyName);
            if (entry.DisplayName is not null || entry.InstallLocation is not null)
            {
                return entry;
            }
        }

        return null;
    }

    private static UninstallEntry Read(IRegistryReader registry, string key, string name) => new(
        key,
        name,
        Clean(registry.GetString(key, "DisplayName")),
        Clean(registry.GetString(key, "Publisher")),
        Clean(registry.GetString(key, "InstallLocation"))?.Trim('"'),
        Clean(registry.GetString(key, "UninstallString")),
        Clean(registry.GetString(key, "DisplayIcon")));

    private static string? Clean(string? value) => string.IsNullOrWhiteSpace(value) ? null : value.Trim();
}

/// <summary>Caminhos do Windows em texto, testáveis fora do Windows (sempre com <c>\</c>).</summary>
public static class WindowsPath
{
    /// <summary>Junta pasta e partes com <c>\</c>, aceitando <c>/</c> na pasta (o registro da Ubisoft usa <c>/</c>).</summary>
    public static string Combine(string folder, params string[] parts)
    {
        ArgumentNullException.ThrowIfNull(folder);
        ArgumentNullException.ThrowIfNull(parts);
        string path = Normalize(folder);
        foreach (string part in parts)
        {
            string clean = Normalize(part).TrimStart('\\');
            path = path.EndsWith('\\') ? path + clean : path + "\\" + clean;
        }

        return path;
    }

    /// <summary>Barras normalizadas para <c>\</c>, sem barra no fim (exceto a raiz "C:\").</summary>
    public static string Normalize(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        string normalized = path.Trim().Trim('"').Replace('/', '\\').TrimEnd('\\');
        return normalized.Length == 2 && normalized[1] == ':' ? normalized + "\\" : normalized;
    }

    /// <summary>Último nome do caminho ("D:\Jogos\Farol" → "Farol").</summary>
    public static string LastName(string path)
    {
        string normalized = Normalize(path);
        int slash = normalized.LastIndexOf('\\');
        return slash < 0 ? normalized : normalized[(slash + 1)..];
    }
}

/// <summary>Quando o jogo chegou ao PC, pela data de criação da pasta ("Adicionados recentemente").</summary>
public static class InstallDates
{
    public static DateTimeOffset Of(string folder)
    {
        try
        {
            return Directory.Exists(folder) ? new DateTimeOffset(Directory.GetCreationTimeUtc(folder), TimeSpan.Zero) : default;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or ArgumentException or NotSupportedException)
        {
            return default;
        }
    }
}
