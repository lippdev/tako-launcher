using System.Globalization;

namespace Launcher.Core.Library.Steam;

/// <summary>Jogo instalado numa biblioteca da Steam.</summary>
public sealed record SteamApp(
    int AppId,
    string Name,
    string InstallPath,
    long SizeOnDisk,
    DateTimeOffset InstalledAt,
    DateTimeOffset? LastPlayed,
    TimeSpan PlayTime);

/// <summary>
/// Lê os jogos instalados direto dos arquivos da Steam, sem API nem Steam aberta: bibliotecas em
/// <c>steamapps/libraryfolders.vdf</c>, jogos em <c>appmanifest_*.acf</c> e tempo jogado / última vez em
/// <c>userdata/&lt;conta&gt;/config/localconfig.vdf</c> (da conta usada por último).
/// </summary>
public static class SteamLibraryScanner
{
    private const int FullyInstalled = 4;

    /// <summary>Ferramentas que a Steam instala como "apps" e que não são jogos.</summary>
    private static readonly int[] ToolAppIds = [228980, 1070560, 1391110, 1628350, 1493710, 2180100, 250820];

    private static readonly string[] ToolPrefixes = ["Steamworks Common", "Proton", "Steam Linux Runtime", "SteamVR"];

    public static IReadOnlyList<SteamApp> Scan(string steamRoot)
    {
        ArgumentNullException.ThrowIfNull(steamRoot);
        var usage = ReadUsage(steamRoot);
        var apps = new Dictionary<int, SteamApp>();
        foreach (string library in LibraryFolders(steamRoot))
        {
            string steamapps = System.IO.Path.Combine(library, "steamapps");
            if (!Directory.Exists(steamapps))
            {
                continue;
            }

            foreach (string manifest in Directory.EnumerateFiles(steamapps, "appmanifest_*.acf"))
            {
                if (TryReadManifest(manifest, steamapps, usage, out var app) && !apps.ContainsKey(app.AppId))
                {
                    apps[app.AppId] = app;
                }
            }
        }

        return [.. apps.Values.OrderBy(a => a.Name, StringComparer.CurrentCultureIgnoreCase)];
    }

    /// <summary>Pastas de biblioteca: a da própria Steam e as de <c>libraryfolders.vdf</c> (formatos novo e antigo).</summary>
    public static IReadOnlyList<string> LibraryFolders(string steamRoot)
    {
        var folders = new List<string> { steamRoot };
        string file = System.IO.Path.Combine(steamRoot, "steamapps", "libraryfolders.vdf");
        if (!File.Exists(file))
        {
            return folders;
        }

        var root = Vdf.Load(file);
        var list = root.Node("libraryfolders") ?? root.Node("LibraryFolders");
        foreach (var (key, value) in list?.Children ?? [])
        {
            if (!int.TryParse(key, NumberStyles.None, CultureInfo.InvariantCulture, out _))
            {
                continue; // ex.: "TimeNextStatsReport", "contentstatsid"
            }

            string? path = value switch
            {
                VdfNode node => node.Value("path"),
                string text => text,
                _ => null,
            };
            if (!string.IsNullOrWhiteSpace(path) && !folders.Any(f => SamePath(f, path)))
            {
                folders.Add(path);
            }
        }

        return folders;
    }

    private static bool TryReadManifest(string file, string steamapps, Dictionary<int, (DateTimeOffset? LastPlayed, TimeSpan PlayTime)> usage, out SteamApp app)
    {
        app = null!;
        VdfNode? state;
        try
        {
            state = Vdf.Load(file).Node("AppState");
        }
        catch (IOException)
        {
            return false;
        }

        if (state is null
            || !int.TryParse(state.Value("appid"), NumberStyles.None, CultureInfo.InvariantCulture, out int appId)
            || state.Value("name") is not { Length: > 0 } name
            || state.Value("installdir") is not { Length: > 0 } installDir)
        {
            return false;
        }

        _ = int.TryParse(state.Value("StateFlags"), NumberStyles.None, CultureInfo.InvariantCulture, out int flags);
        if ((flags & FullyInstalled) == 0 || ToolAppIds.Contains(appId) || ToolPrefixes.Any(p => name.StartsWith(p, StringComparison.OrdinalIgnoreCase)))
        {
            return false;
        }

        _ = long.TryParse(state.Value("SizeOnDisk"), NumberStyles.None, CultureInfo.InvariantCulture, out long size);
        var (lastPlayed, playTime) = usage.GetValueOrDefault(appId);
        lastPlayed ??= UnixTime(state.Value("LastPlayed"));
        app = new SteamApp(
            appId,
            name,
            System.IO.Path.Combine(steamapps, "common", installDir),
            size,
            new DateTimeOffset(File.GetCreationTimeUtc(file), TimeSpan.Zero),
            lastPlayed,
            playTime);
        return true;
    }

    /// <summary>Tempo jogado (minutos) e última vez por jogo, da conta cujo <c>localconfig.vdf</c> mudou por último.</summary>
    private static Dictionary<int, (DateTimeOffset? LastPlayed, TimeSpan PlayTime)> ReadUsage(string steamRoot)
    {
        var usage = new Dictionary<int, (DateTimeOffset?, TimeSpan)>();
        string userdata = System.IO.Path.Combine(steamRoot, "userdata");
        if (!Directory.Exists(userdata))
        {
            return usage;
        }

        string? config = Directory.EnumerateDirectories(userdata)
            .Select(d => System.IO.Path.Combine(d, "config", "localconfig.vdf"))
            .Where(File.Exists)
            .OrderByDescending(File.GetLastWriteTimeUtc)
            .FirstOrDefault();
        if (config is null)
        {
            return usage;
        }

        VdfNode? apps;
        try
        {
            apps = Vdf.Load(config).Path("UserLocalConfigStore", "Software", "Valve", "Steam", "apps");
        }
        catch (IOException)
        {
            return usage;
        }

        foreach (var (key, value) in apps?.Children ?? [])
        {
            if (value is VdfNode node && int.TryParse(key, NumberStyles.None, CultureInfo.InvariantCulture, out int appId))
            {
                _ = int.TryParse(node.Value("Playtime"), NumberStyles.None, CultureInfo.InvariantCulture, out int minutes);
                usage[appId] = (UnixTime(node.Value("LastPlayed")), TimeSpan.FromMinutes(minutes));
            }
        }

        return usage;
    }

    private static DateTimeOffset? UnixTime(string? value) =>
        long.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out long seconds) && seconds > 0
            ? DateTimeOffset.FromUnixTimeSeconds(seconds)
            : null;

    private static bool SamePath(string a, string b) =>
        string.Equals(System.IO.Path.GetFullPath(a).TrimEnd('\\', '/'), System.IO.Path.GetFullPath(b).TrimEnd('\\', '/'), StringComparison.OrdinalIgnoreCase);
}
