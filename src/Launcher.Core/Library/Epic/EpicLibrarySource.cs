using System.Text.Json;
using Launcher.Core.Library.Sources;
using Launcher.Core.Library.Steam;

namespace Launcher.Core.Library.Epic;

/// <summary>
/// Fonte de biblioteca da Epic Games (STORE-002, ADR-0019): lê os manifests da Epic sem a Epic aberta e abre os jogos
/// pelo protocolo oficial (<see cref="EpicManifests.LaunchUri"/>), nunca pelo executável direto, para respeitar login e
/// DRM. O fim da sessão é reconhecido pelos processos na pasta do jogo.
/// </summary>
public sealed class EpicLibrarySource(IRegistryReader registry, string programData, Func<string, bool>? directoryExists = null) : ILibrarySource
{
    /// <summary>Onde a Epic guarda os dados (inclusive a pasta de manifests), quando instalada.</summary>
    public const string LauncherKey = @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Epic Games\EpicGamesLauncher";

    /// <summary>Protocolo <c>com.epicgames.launcher://</c>: sem ele, nada abre os jogos (o cliente não está instalado).</summary>
    public const string ProtocolKey = @"HKEY_CLASSES_ROOT\com.epicgames.launcher\shell\open\command";

    private const string Accent = "#2A2A2A"; // store-epic (DESIGN.md), até a cor sair da capa

    private readonly Func<string, bool> _directoryExists = directoryExists ?? Directory.Exists;

    public GameStore Store => GameStore.Epic;

    public LibraryScan Scan()
    {
        bool client = registry.GetString(ProtocolKey, string.Empty) is not null;
        string manifests = ManifestsFolder();
        if (!Directory.Exists(manifests))
        {
            return client ? new LibraryScan([], ClientInstalled: true) : LibraryScan.Missing("Epic Games Launcher não encontrado neste PC");
        }

        var locations = ReadInstalledList();
        var games = new List<Game>();
        foreach (string file in Directory.EnumerateFiles(manifests, "*.item").Order(StringComparer.OrdinalIgnoreCase))
        {
            EpicInstall? install;
            try
            {
                install = EpicManifests.ParseManifest(File.ReadAllText(file));
            }
            catch (JsonException)
            {
                continue; // manifest corrompido ou sendo gravado: os outros seguem
            }

            if (install is null)
            {
                continue;
            }

            // O jogo pode ter sido movido pela Epic: a lista de instalados tem a pasta atual.
            if (!_directoryExists(install.InstallLocation) && locations.TryGetValue(install.AppName, out string? moved))
            {
                install = install with { InstallLocation = moved };
            }

            if (_directoryExists(install.InstallLocation) && !games.Exists(g => g.Id == IdFor(install)))
            {
                games.Add(ToGame(install, new DateTimeOffset(File.GetCreationTimeUtc(file), TimeSpan.Zero)));
            }
        }

        return new LibraryScan(
            [.. games.OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase)],
            client,
            client ? null : "Epic Games Launcher não encontrado: instale-o para abrir estes jogos");
    }

    public static string IdFor(EpicInstall install)
    {
        ArgumentNullException.ThrowIfNull(install);
        return $"epic:{install.AppName}";
    }

    public static Game ToGame(EpicInstall install, DateTimeOffset addedAt)
    {
        ArgumentNullException.ThrowIfNull(install);
        return new Game
        {
            Id = IdFor(install),
            Title = install.DisplayName,
            Store = GameStore.Epic,
            SizeLabel = SteamGames.SizeLabel(install.InstallSize),
            Accent = Accent,
            IsInstalled = true,
            AddedAt = addedAt,
            LaunchTarget = EpicManifests.LaunchUri(install),
            InstallPath = install.InstallLocation,
        };
    }

    private string ManifestsFolder()
    {
        // A Epic grava o caminho dos dados no registro; o padrão fica em %ProgramData%.
        string? data = registry.GetString(LauncherKey, "AppDataPath");
        return string.IsNullOrWhiteSpace(data)
            ? Path.Combine(programData, "Epic", "EpicGamesLauncher", "Data", "Manifests")
            : Path.Combine(data, "Manifests");
    }

    private IReadOnlyDictionary<string, string> ReadInstalledList()
    {
        string file = Path.Combine(programData, "Epic", "UnrealEngineLauncher", "LauncherInstalled.dat");
        try
        {
            return File.Exists(file) ? EpicManifests.ParseInstalledList(File.ReadAllText(file)) : new Dictionary<string, string>();
        }
        catch (JsonException)
        {
            return new Dictionary<string, string>();
        }
    }
}
