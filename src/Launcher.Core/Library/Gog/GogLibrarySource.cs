using System.Text.Json;
using Launcher.Core.Library.Sources;

namespace Launcher.Core.Library.Gog;

/// <summary>
/// Fonte de biblioteca do GOG (STORE-003, ADR-0019): jogos instalados pelo GOG Galaxy ou pelos instaladores offline,
/// lidos do registro (<see cref="GamesKey"/>) e do <c>goggame-&lt;id&gt;.info</c> na pasta. Abre pelo Galaxy quando
/// instalado; senão, pelo executável do jogo (sem DRM). O fim da sessão é reconhecido pelos processos na pasta do jogo.
/// </summary>
public sealed class GogLibrarySource(IRegistryReader registry, Func<string, string?>? readFile = null, Func<string, bool>? exists = null) : ILibrarySource
{
    public const string GamesKey = @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\Games";

    public const string GalaxyKey = @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\GalaxyClient";

    private const string Accent = "#7A3FD1"; // store-gog (DESIGN.md), até a cor sair da capa

    private readonly Func<string, string?> _readFile = readFile ?? (path => File.Exists(path) ? File.ReadAllText(path) : null);
    private readonly Func<string, bool> _exists = exists ?? (path => Directory.Exists(path) || File.Exists(path));

    public GameStore Store => GameStore.Gog;

    /// <summary><c>GalaxyClient.exe</c> quando o GOG Galaxy está instalado.</summary>
    public string? GalaxyExecutable()
    {
        string? folder = registry.GetString(GalaxyKey + @"\paths", "client");
        string exe = registry.GetString(GalaxyKey, "clientExecutable") ?? "GalaxyClient.exe";
        if (string.IsNullOrWhiteSpace(folder))
        {
            return null;
        }

        string path = folder.TrimEnd('\\', '/') + "\\" + exe;
        return _exists(path) ? path : null;
    }

    public LibraryScan Scan()
    {
        string? galaxy = GalaxyExecutable();
        var games = new List<Game>();
        foreach (string id in registry.SubKeyNames(GamesKey))
        {
            string key = $@"{GamesKey}\{id}";
            var entry = new GogRegistryEntry(
                registry.GetString(key, "gameID") ?? id,
                registry.GetString(key, "gameName") ?? string.Empty,
                registry.GetString(key, "path") ?? string.Empty,
                registry.GetString(key, "exe"),
                registry.GetString(key, "launchParam"),
                registry.GetString(key, "workingDir"),
                registry.GetString(key, "launchCommand"),
                registry.GetString(key, "dependsOn"));
            if (string.IsNullOrWhiteSpace(entry.Path) || !_exists(entry.Path))
            {
                continue; // desinstalado sem limpar o registro
            }

            var info = ReadInfo(entry);
            if (GogGames.IsPlayable(entry, info) && GogGames.Launch(entry, info, galaxy) is { } launch && !games.Exists(g => g.Id == IdFor(entry.GameId)))
            {
                games.Add(ToGame(entry, launch, new DateTimeOffset(Directory.GetCreationTimeUtc(entry.Path), TimeSpan.Zero)));
            }
        }

        // Sem Galaxy os jogos abrem direto pelo executável; o aviso só explica por que nuvem e conquistas não sincronizam.
        return new LibraryScan(
            [.. games.OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase)],
            ClientInstalled: galaxy is not null,
            galaxy is null && games.Count > 0 ? "Sem GOG Galaxy: os jogos abrem direto pelo executável" : null);
    }

    public static string IdFor(string gameId) => $"gog:{gameId}";

    public static Game ToGame(GogRegistryEntry entry, GogLaunch launch, DateTimeOffset addedAt = default)
    {
        ArgumentNullException.ThrowIfNull(entry);
        ArgumentNullException.ThrowIfNull(launch);
        return new Game
        {
            Id = IdFor(entry.GameId),
            Title = entry.Name.Trim(),
            Store = GameStore.Gog,
            Accent = Accent,
            IsInstalled = true,
            AddedAt = addedAt,
            LaunchTarget = launch.Target,
            LaunchArguments = launch.Arguments,
            WorkingDirectory = launch.WorkingDirectory,
            InstallPath = entry.Path,
        };
    }

    private GogInfo? ReadInfo(GogRegistryEntry entry)
    {
        try
        {
            string? json = _readFile(entry.Path.TrimEnd('\\', '/') + $"\\goggame-{entry.GameId}.info");
            return json is null ? null : GogGames.ParseInfo(json);
        }
        catch (JsonException)
        {
            return null;
        }
    }
}
