using System.Xml;
using Launcher.Core.Library.Sources;

namespace Launcher.Core.Library.Ea;

/// <summary>
/// Fonte de biblioteca do EA app (STORE-006, ADR-0019). Os jogos instalados vêm do registro que os instaladores da EA
/// gravam (<see cref="GameKeys"/>, valor <c>Install Dir</c>) e das chaves de desinstalação com o editor "Electronic
/// Arts"; cada pasta precisa do <c>__Installer\installerdata.xml</c>, que dá os ids de conteúdo e o nome. Abre sempre
/// pelo EA app (<see cref="EaGames.LaunchUri"/>), que cuida de login, DRM e atualização; o fim da sessão vem dos
/// processos na pasta do jogo. Só o Origin antigo instalado conta como cliente desatualizado (STORE-012).
/// </summary>
public sealed class EaLibrarySource(IRegistryReader registry, Func<string, string?>? readFile = null, Func<string, bool>? exists = null) : ILibrarySource
{
    /// <summary>Chaves com uma subchave por jogo (nome do jogo), gravadas pelos instaladores da EA.</summary>
    public static readonly IReadOnlyList<string> GameKeys =
    [
        @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\EA Games",
        @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Electronic Arts",
        @"HKEY_LOCAL_MACHINE\SOFTWARE\EA Games",
        @"HKEY_LOCAL_MACHINE\SOFTWARE\Electronic Arts",
    ];

    public const string Origin2ProtocolKey = @"HKEY_CLASSES_ROOT\origin2\shell\open\command";

    public const string Link2EaProtocolKey = @"HKEY_CLASSES_ROOT\link2ea\shell\open\command";

    public const string MissingNotice = "EA app não encontrado: instale-o para abrir estes jogos";

    public const string OutdatedNotice = "Só o Origin está instalado e ele não abre mais os jogos: instale o EA app";

    private const string Accent = "#D9302B"; // store-ea (DESIGN.md)

    private static readonly string[] InstallDirValues = ["Install Dir", "InstallDir", "Install Location"];

    private readonly Func<string, string?> _readFile = readFile ?? (path => File.Exists(path) ? File.ReadAllText(path) : null);
    private readonly Func<string, bool> _exists = exists ?? Directory.Exists;

    public GameStore Store => GameStore.Ea;

    public EaClientKind Client() => EaGames.ClientKind(
        registry.GetString(Origin2ProtocolKey, string.Empty),
        registry.GetString(Link2EaProtocolKey, string.Empty));

    public LibraryScan Scan()
    {
        var client = Client();
        var games = new List<Game>();
        foreach (var (folder, fallbackTitle) in CandidateFolders())
        {
            string path = WindowsPath.Normalize(folder);
            if (!_exists(path) || games.Exists(g => string.Equals(g.InstallPath, path, StringComparison.OrdinalIgnoreCase)))
            {
                continue;
            }

            EaInstallerData data;
            try
            {
                string? xml = _readFile(WindowsPath.Combine(path, "__Installer", "installerdata.xml"));
                if (xml is null)
                {
                    continue; // sem o arquivo do instalador não há como pedir ao EA app para abrir
                }

                data = EaGames.Parse(xml);
            }
            catch (XmlException)
            {
                continue;
            }

            if (data.ContentIds.Count == 0 || games.Exists(g => g.Id == IdFor(data.ContentIds[0])))
            {
                continue;
            }

            games.Add(ToGame(data, data.Title ?? fallbackTitle ?? WindowsPath.LastName(path), path, InstallDates.Of(path)));
        }

        if (games.Count == 0 && client == EaClientKind.Missing)
        {
            return LibraryScan.Missing("EA app não encontrado neste PC");
        }

        return new LibraryScan(
            [.. games.OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase)],
            client == EaClientKind.EaApp,
            client switch
            {
                EaClientKind.LegacyOrigin => OutdatedNotice,
                EaClientKind.Missing => MissingNotice,
                _ => null,
            });
    }

    public static string IdFor(string contentId) => $"ea:{contentId}";

    public static Game ToGame(EaInstallerData data, string title, string folder, DateTimeOffset addedAt = default)
    {
        ArgumentNullException.ThrowIfNull(data);
        return new Game
        {
            Id = IdFor(data.ContentIds[0]),
            Title = title.Trim(),
            Store = GameStore.Ea,
            Accent = Accent,
            IsInstalled = true,
            AddedAt = addedAt,
            LaunchTarget = EaGames.LaunchUri(data.ContentIds),
            InstallPath = folder,
            StoreProductId = data.ContentIds[0],
        };
    }

    /// <summary>Pastas candidatas: o registro dos jogos da EA e os programas de "Electronic Arts" instalados.</summary>
    private IEnumerable<(string Folder, string? Title)> CandidateFolders()
    {
        foreach (string root in GameKeys)
        {
            foreach (string name in registry.SubKeyNames(root))
            {
                string key = $@"{root}\{name}";
                string? folder = InstallDirValues.Select(v => registry.GetString(key, v)).FirstOrDefault(v => !string.IsNullOrWhiteSpace(v));
                if (folder is not null)
                {
                    yield return (folder, name);
                }
            }
        }

        foreach (var entry in UninstallEntries.Read(registry))
        {
            if (entry.InstallLocation is { } folder && entry.Publisher?.Contains("Electronic Arts", StringComparison.OrdinalIgnoreCase) == true)
            {
                yield return (folder, entry.DisplayName);
            }
        }
    }
}
