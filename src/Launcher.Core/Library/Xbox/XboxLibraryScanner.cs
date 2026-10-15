namespace Launcher.Core.Library.Xbox;

/// <summary>Pacote registrado para o usuário: nome completo e pasta de instalação.</summary>
public sealed record InstalledPackage(string FullName, string Folder);

/// <summary>
/// Acesso ao Windows que a fonte Xbox usa (registro, unidades, recursos de pacote). A implementação real fica na UI;
/// os testes usam uma falsa com pastas de exemplo.
/// </summary>
public interface IXboxSystem
{
    /// <summary>Raízes das unidades fixas prontas (ex.: <c>C:\</c>, <c>D:\</c>).</summary>
    IReadOnlyList<string> DriveRoots();

    /// <summary>Pacotes MSIX/AppX registrados para o usuário atual; nulo quando não foi possível ler.</summary>
    IReadOnlyList<InstalledPackage>? UserPackages();

    /// <summary>Pacotes que os Serviços de Jogos registram como jogos (nomes completos ou famílias).</summary>
    IReadOnlyCollection<string> GamingServicesPackages();

    bool GamingServicesInstalled();

    /// <summary>Resolve um texto indireto (<c>@{Pacote?ms-resource://…}</c>); nulo se falhar.</summary>
    string? LoadIndirectString(string indirect);
}

/// <summary>Jogo do app Xbox / Microsoft Store encontrado no disco.</summary>
public sealed record XboxInstalledGame
{
    public required string FamilyName { get; init; }

    /// <summary>Id do app no AUMID (<c>Família!AppId</c>).</summary>
    public required string AppId { get; init; }

    public required string Title { get; init; }

    /// <summary>Pasta do pacote (ex.: <c>C:\XboxGames\Jogo\Content</c> ou <c>WindowsApps\…</c>).</summary>
    public required string Folder { get; init; }

    public string? CoverPath { get; init; }

    public string? HeroPath { get; init; }

    public string Accent { get; init; } = XboxGameRules.BrandColor;

    public string? StoreId { get; init; }

    public DateTimeOffset InstalledAt { get; init; }

    public string LaunchTarget => PackageIdentity.LaunchTarget(FamilyName, AppId);
}

public sealed record XboxScanResult(IReadOnlyList<XboxInstalledGame> Games, XboxAvailability Availability);

/// <summary>
/// Procura jogos do app Xbox / Microsoft Store (STORE-004) sem contornar o pacote: pastas de jogos do app Xbox
/// (<c>.GamingRoot</c> de cada unidade, <c>&lt;pasta&gt;\&lt;jogo&gt;\Content\MicrosoftGame.config</c>) e pacotes
/// registrados para o usuário que são jogos (<see cref="XboxGameRules.IsGame"/>). Só lê arquivos; o jogo é aberto
/// pelo AUMID.
/// </summary>
public static class XboxLibraryScanner
{
    public static XboxScanResult Scan(IXboxSystem system)
    {
        ArgumentNullException.ThrowIfNull(system);
        var packages = Safe(system.UserPackages);
        var registered = new Dictionary<string, InstalledPackage>(StringComparer.OrdinalIgnoreCase);
        foreach (var package in packages ?? [])
        {
            if (PackageIdentity.TryFamilyFromFullName(package.FullName, out string family))
            {
                registered.TryAdd(family, package);
            }
        }

        var gamingServices = XboxGameRules.Families(Safe(system.GamingServicesPackages) ?? []);
        var context = new ScanContext(system, packages is null ? null : registered, gamingServices);
        var games = new Dictionary<string, XboxInstalledGame>(StringComparer.OrdinalIgnoreCase);

        // 1. Pastas de jogos do app Xbox em cada unidade.
        foreach (string drive in Safe(system.DriveRoots) ?? [])
        {
            foreach (string gamesFolder in GameFolders(drive))
            {
                foreach (string entry in Subfolders(gamesFolder))
                {
                    string content = Path.Combine(entry, "Content");
                    string folder = Directory.Exists(content) ? content : entry;
                    if (File.Exists(Path.Combine(folder, MicrosoftGameConfig.FileName)) && Read(context, folder, fullName: null) is { } game)
                    {
                        games.TryAdd(game.FamilyName, game);
                    }
                }
            }
        }

        // 2. Pacotes registrados para o usuário (jogos em WindowsApps ou em pastas fora do .GamingRoot).
        foreach (var (family, package) in registered)
        {
            if (!games.ContainsKey(family) && !XboxGameRules.IsSystemPackage(family) && Directory.Exists(package.Folder)
                && Read(context, package.Folder, package.FullName) is { } game)
            {
                games.TryAdd(game.FamilyName, game);
            }
        }

        var availability = XboxGameRules.Availability(packages is null ? null : registered.Keys, Safe(system.GamingServicesInstalled));
        return new XboxScanResult([.. games.Values.OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase)], availability);
    }

    /// <summary>Lê um pacote; nulo quando não é jogo, não está registrado para o usuário ou não dá para abrir.</summary>
    private static XboxInstalledGame? Read(ScanContext context, string folder, string? fullName)
    {
        var config = ReadFile(folder, MicrosoftGameConfig.FileName) is { } configXml ? MicrosoftGameConfig.Parse(configXml) : null;
        var manifest = ReadFile(folder, AppxManifest.FileName) is { } manifestXml ? AppxManifest.Parse(manifestXml) : null;
        string? family = manifest?.FamilyName ?? config?.FamilyName;
        if (family is null && !PackageIdentity.TryFamilyFromFullName(fullName, out family))
        {
            return null;
        }

        // Pasta sem pacote registrado (instalação de outro usuário, incompleta ou removida): o Windows não abriria.
        InstalledPackage? package = null;
        if (context.Registered is not null && !context.Registered.TryGetValue(family, out package))
        {
            return null;
        }

        fullName ??= package?.FullName;
        bool hasXboxServices = File.Exists(Path.Combine(folder, XboxGameRules.XboxServicesConfig));
        if (!XboxGameRules.IsGame(family, config is not null, hasXboxServices, context.GamingServices.Contains(family), manifest))
        {
            return null;
        }

        var app = manifest?.MainApplication;
        string? appId = app?.Id ?? (config is { Executables.Count: > 0 } ? config.Executables[0].Id : null);
        if (appId is null)
        {
            return null;
        }

        string packageName = PackageIdentity.NameOf(family);
        string? title = new[] { config?.DisplayName, manifest?.DisplayName, app?.DisplayName }
            .Select(t => Text(context.System, fullName, packageName, t))
            .FirstOrDefault(t => t is not null);
        string? gameFolderName = string.Equals(Path.GetFileName(folder), "Content", StringComparison.OrdinalIgnoreCase)
            ? Path.GetFileName(Path.GetDirectoryName(folder))
            : null;
        return new XboxInstalledGame
        {
            FamilyName = family,
            AppId = appId,
            Title = title ?? XboxGameRules.FallbackTitle(family, gameFolderName),
            Folder = folder,
            CoverPath = FirstAsset(folder, config?.Square480x480Logo, app?.Square310x310Logo, config?.Square150x150Logo, app?.Square150x150Logo, config?.StoreLogo, manifest?.Logo),
            HeroPath = FirstAsset(folder, config?.SplashScreenImage, app?.SplashScreen, app?.Wide310x150Logo),
            Accent = XboxGameRules.Accent(config?.BackgroundColor, app?.BackgroundColor),
            StoreId = config?.StoreId,
            InstalledAt = new DateTimeOffset(Directory.GetCreationTimeUtc(folder), TimeSpan.Zero),
        };
    }

    /// <summary>Texto do manifesto; <c>ms-resource:</c> é resolvido pelo Windows (sem nome completo, fica de fora).</summary>
    private static string? Text(IXboxSystem system, string? fullName, string packageName, string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        string? indirect = PackageIdentity.IndirectString(fullName ?? string.Empty, packageName, value);
        if (indirect is null)
        {
            return value.Trim();
        }

        if (fullName is null)
        {
            return null;
        }

        string? resolved = Safe(() => system.LoadIndirectString(indirect));
        return string.IsNullOrWhiteSpace(resolved) || resolved.StartsWith("ms-resource:", StringComparison.OrdinalIgnoreCase) ? null : resolved.Trim();
    }

    private static string? FirstAsset(string folder, params string?[] declared) =>
        declared.Select(d => PackageAssets.Resolve(folder, d)).FirstOrDefault(p => p is not null);

    private static IEnumerable<string> GameFolders(string drive)
    {
        byte[] data = [];
        try
        {
            string file = Path.Combine(drive, GamingRoot.FileName);
            if (File.Exists(file))
            {
                data = File.ReadAllBytes(file);
            }
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Sem acesso ao arquivo: tenta a pasta padrão.
        }

        return GamingRoot.Folders(drive, data).Where(Directory.Exists);
    }

    private static IEnumerable<string> Subfolders(string folder)
    {
        try
        {
            return [.. Directory.EnumerateDirectories(folder)];
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return [];
        }
    }

    private static string? ReadFile(string folder, string name)
    {
        try
        {
            string path = Path.Combine(folder, name);
            return File.Exists(path) ? File.ReadAllText(path) : null;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    // Uma fonte do Windows que falha não derruba a procura (o Steam e os adicionados continuam).
    private static T? Safe<T>(Func<T> read)
    {
        try
        {
            return read();
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or System.Security.SecurityException or System.ComponentModel.Win32Exception)
        {
            return default;
        }
    }

    private sealed record ScanContext(IXboxSystem System, Dictionary<string, InstalledPackage>? Registered, HashSet<string> GamingServices);
}
