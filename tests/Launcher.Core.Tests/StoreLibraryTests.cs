using System.Text.Json;
using Launcher.Core.Library;
using Launcher.Core.Library.Epic;
using Launcher.Core.Library.Gog;
using Launcher.Core.Library.Sources;
using Launcher.Core.Library.Steam;
using Launcher.Core.Library.Xbox;
using Launcher.Core.Session;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Fontes de biblioteca (ADR-0019): Epic, GOG, agregação e jogos em duas lojas, com manifests e registro falsos.</summary>
public sealed class StoreLibraryTests
{
    private const string FarolFolder = @"C:\Program Files\Epic Games\Farol";
    private const string MovedCorridaFolder = @"E:\Jogos\CorridaNoturna";
    private const string CidadeFolder = @"D:\GOG Games\Cidade Submersa";
    private const string GalaxyFolder = @"C:\Program Files (x86)\GOG Galaxy";

    private static readonly string Fixtures = Path.Combine(AppContext.BaseDirectory, "Fixtures", "Stores");
    private static readonly string ProgramData = Path.Combine(Fixtures, "ProgramData");
    private static readonly string Manifests = Path.Combine(ProgramData, "Epic", "EpicGamesLauncher", "Data", "Manifests");

    // ------------------------------------------------------------------ Epic: manifests

    [Fact]
    public void Epic_manifest_is_read_with_launch_identity()
    {
        var install = EpicManifests.ParseManifest(File.ReadAllText(Path.Combine(Manifests, "0A1B2C3D.item")));

        Assert.NotNull(install);
        Assert.Equal("Farol", install.AppName);
        Assert.Equal("Farol do Fim do Mundo", install.DisplayName);
        Assert.Equal(FarolFolder, install.InstallLocation);
        Assert.Equal("Binaries/Win64/Farol.exe", install.LaunchExecutable);
        Assert.Equal("farolns", install.CatalogNamespace);
        Assert.Equal("a1b2c3d4e5f60718293a4b5c6d7e8f90", install.CatalogItemId);
        Assert.Equal(16106127360, install.InstallSize);
    }

    [Theory]
    [InlineData("1B2C3D4E.item")] // DLC: o jogo principal é outro
    [InlineData("2C3D4E5F.item")] // plugin do Unreal Engine (sem a categoria "games")
    [InlineData("3D4E5F60.item")] // instalação incompleta
    public void Epic_manifests_that_are_not_playable_games_are_skipped(string file) =>
        Assert.Null(EpicManifests.ParseManifest(File.ReadAllText(Path.Combine(Manifests, file))));

    [Fact]
    public void Epic_invalid_manifest_throws_json_error() =>
        Assert.ThrowsAny<JsonException>(() => EpicManifests.ParseManifest(File.ReadAllText(Path.Combine(Manifests, "5F607182.item"))));

    [Fact]
    public void Epic_installed_list_maps_app_to_current_folder()
    {
        var list = EpicManifests.ParseInstalledList(File.ReadAllText(Path.Combine(ProgramData, "Epic", "UnrealEngineLauncher", "LauncherInstalled.dat")));

        Assert.Equal(FarolFolder, list["Farol"]);
        Assert.Equal(MovedCorridaFolder, list["corrida noturna"]);
    }

    [Fact]
    public void Epic_games_open_through_the_official_protocol_never_the_exe()
    {
        var install = new EpicInstall("Corrida Noturna", "Corrida", @"C:\X", "Corrida.exe", "corrida", "c0ffee", 0);

        Assert.Equal("com.epicgames.launcher://apps/corrida%3Ac0ffee%3ACorrida%20Noturna?action=launch&silent=true", EpicManifests.LaunchUri(install));
    }

    [Fact]
    public void Epic_source_lists_installed_games_and_follows_moved_folders()
    {
        var registry = new FakeRegistry().Set(EpicLibrarySource.ProtocolKey, string.Empty, "\"C:\\Epic\\EpicGamesLauncher.exe\" %1");
        var source = new EpicLibrarySource(registry, ProgramData, ExistingFolders(FarolFolder, MovedCorridaFolder));

        var scan = source.Scan();

        Assert.True(scan.ClientInstalled);
        Assert.Null(scan.Notice);
        Assert.Equal(["Corrida Noturna™", "Farol do Fim do Mundo"], scan.Games.Select(g => g.Title));
        var corrida = scan.Games[0];
        Assert.Equal("epic:Corrida Noturna", corrida.Id);
        Assert.Equal(GameStore.Epic, corrida.Store);
        Assert.Equal(MovedCorridaFolder, corrida.InstallPath); // o manifest aponta para a pasta antiga
        Assert.StartsWith("com.epicgames.launcher://apps/corrida%3Ac0ffee%3A", corrida.LaunchTarget, StringComparison.Ordinal);
        Assert.Equal("700 MB", corrida.SizeLabel);
        Assert.True(corrida.CanPlay);
        Assert.Null(corrida.CoverUri); // a capa vem do agente de metadados
    }

    [Fact]
    public void Epic_source_warns_when_the_launcher_is_missing()
    {
        var withGames = new EpicLibrarySource(new FakeRegistry(), ProgramData, ExistingFolders(FarolFolder)).Scan();
        Assert.False(withGames.ClientInstalled);
        Assert.Single(withGames.Games);
        Assert.Contains("Epic Games Launcher não encontrado", withGames.Notice, StringComparison.Ordinal);

        var nothing = new EpicLibrarySource(new FakeRegistry(), Path.Combine(Fixtures, "nao-existe")).Scan();
        Assert.Empty(nothing.Games);
        Assert.Equal("Epic Games Launcher não encontrado neste PC", nothing.Notice);
    }

    [Fact]
    public void Epic_games_are_tracked_by_install_folder()
    {
        var game = EpicLibrarySource.ToGame(new EpicInstall("Farol", "Farol", FarolFolder, null, "ns", "id", 0), DateTimeOffset.UnixEpoch);

        Assert.Equal(new TrackingPlan(GameTrackingStrategy.InstallFolder, FarolFolder), GameTracking.Plan(game, hasProcess: false));
    }

    // ------------------------------------------------------------------ GOG: registro e .info

    [Fact]
    public void Gog_info_picks_the_primary_game_task()
    {
        var info = GogGames.ParseInfo(File.ReadAllText(Path.Combine(Fixtures, "Gog", "goggame-1400000001.info")));

        Assert.NotNull(info);
        Assert.Equal("1400000001", info.GameId);
        Assert.Equal(@"bin\x64\cidade.exe", info.PlayPath);
        Assert.Equal("-skipintro", info.PlayArguments);
    }

    [Fact]
    public void Gog_dlc_is_not_a_library_item()
    {
        var dlcInfo = GogGames.ParseInfo(File.ReadAllText(Path.Combine(Fixtures, "Gog", "goggame-1400000002.info")));
        var entry = new GogRegistryEntry("1400000002", "Expansão", CidadeFolder);

        Assert.Equal("1400000002", dlcInfo!.GameId); // id numérico também vale
        Assert.False(GogGames.IsPlayable(entry, dlcInfo));
        Assert.False(GogGames.IsPlayable(entry with { DependsOn = "1400000001" }, null));
        Assert.True(GogGames.IsPlayable(entry, null));
    }

    [Fact]
    public void Gog_opens_through_galaxy_when_installed()
    {
        var entry = new GogRegistryEntry("1400000001", "Cidade Submersa", CidadeFolder + "\\", Exe: CidadeFolder + @"\cidade.exe");

        var launch = GogGames.Launch(entry, null, GalaxyFolder + @"\GalaxyClient.exe");

        Assert.NotNull(launch);
        Assert.True(launch.ViaGalaxy);
        Assert.Equal(GalaxyFolder + @"\GalaxyClient.exe", launch.Target);
        Assert.Equal($"/command=runGame /gameId=1400000001 /path=\"{CidadeFolder}\"", launch.Arguments);
    }

    [Fact]
    public void Gog_without_galaxy_opens_the_game_itself()
    {
        var entry = new GogRegistryEntry("1", "Cidade", CidadeFolder);

        var fromRegistry = GogGames.Launch(entry with { Exe = CidadeFolder + @"\cidade.exe", LaunchParam = "-x", WorkingDir = CidadeFolder + @"\bin" }, null, null);
        Assert.Equal(new GogLaunch(CidadeFolder + @"\cidade.exe", "-x", CidadeFolder + @"\bin", ViaGalaxy: false), fromRegistry);

        var fromInfo = GogGames.Launch(entry, new GogInfo("1", "1", "Cidade", @"bin\x64\cidade.exe", "-skipintro", null), null);
        Assert.Equal(new GogLaunch(CidadeFolder + @"\bin\x64\cidade.exe", "-skipintro", CidadeFolder, ViaGalaxy: false), fromInfo);

        var fromCommand = GogGames.Launch(entry with { LaunchCommand = $"\"{CidadeFolder}\\cidade.exe\" -windowed" }, null, null);
        Assert.Equal(new GogLaunch(CidadeFolder + @"\cidade.exe", "-windowed", CidadeFolder, ViaGalaxy: false), fromCommand);

        Assert.Null(GogGames.Launch(entry, null, null));
    }

    [Theory]
    [InlineData("\"C:\\Jogo\\jogo.exe\" -a -b", "C:\\Jogo\\jogo.exe", "-a -b")]
    [InlineData("C:\\Jogo Sem Aspas\\jogo.exe -a", "C:\\Jogo Sem Aspas\\jogo.exe", "-a")]
    [InlineData("\"C:\\Jogo\\jogo.exe\"", "C:\\Jogo\\jogo.exe", null)]
    [InlineData("  ", null, null)]
    public void Gog_launch_command_splits_into_exe_and_arguments(string command, string? exe, string? arguments) =>
        Assert.Equal((exe, arguments), GogGames.SplitCommand(command));

    [Fact]
    public void Gog_source_reads_registry_and_info_and_skips_dlc_and_uninstalled()
    {
        var source = new GogLibrarySource(GogRegistry(), GogInfoFiles, path => path.StartsWith(CidadeFolder, StringComparison.Ordinal));

        var scan = source.Scan();

        var game = Assert.Single(scan.Games);
        Assert.Equal("gog:1400000001", game.Id);
        Assert.Equal("Cidade Submersa", game.Title);
        Assert.Equal(GameStore.Gog, game.Store);
        Assert.Equal(CidadeFolder + @"\bin\x64\cidade.exe", game.LaunchTarget);
        Assert.Equal("-skipintro", game.LaunchArguments);
        Assert.Equal(CidadeFolder, game.InstallPath);
        Assert.False(scan.ClientInstalled);
        Assert.Equal("Sem GOG Galaxy: os jogos abrem direto pelo executável", scan.Notice);
    }

    [Fact]
    public void Gog_source_prefers_galaxy_when_its_executable_exists()
    {
        var registry = GogRegistry()
            .Set(GogLibrarySource.GalaxyKey + @"\paths", "client", GalaxyFolder)
            .Set(GogLibrarySource.GalaxyKey, "clientExecutable", "GalaxyClient.exe");
        var source = new GogLibrarySource(registry, GogInfoFiles, path => path.StartsWith(CidadeFolder, StringComparison.Ordinal) || path == GalaxyFolder + @"\GalaxyClient.exe");

        var scan = source.Scan();

        Assert.True(scan.ClientInstalled);
        Assert.Null(scan.Notice);
        Assert.Equal(GalaxyFolder + @"\GalaxyClient.exe", Assert.Single(scan.Games).LaunchTarget);
    }

    // ------------------------------------------------------------------ Agregação e lojas ligadas/desligadas

    [Fact]
    public void Sources_skip_disabled_stores_and_isolate_failures()
    {
        var failures = new List<GameStore>();
        var sources = new LibrarySources(
            [
                new StubSource(GameStore.Steam, new LibraryScan([StoreGame("steam:1", "Um", GameStore.Steam)], ClientInstalled: true)),
                new StubSource(GameStore.Epic, null),
                new StubSource(GameStore.Gog, new LibraryScan([StoreGame("gog:2", "Dois", GameStore.Gog), StoreGame("gog:2", "Dois de novo", GameStore.Gog)], ClientInstalled: false, "Sem GOG Galaxy: os jogos abrem direto pelo executável")),
                new StubSource(GameStore.Xbox, new LibraryScan([StoreGame("xbox:3", "Três", GameStore.Xbox)], ClientInstalled: true)),
            ],
            store => store != GameStore.Xbox,
            (store, _) => failures.Add(store));

        var games = sources.Discover();

        Assert.Equal(["steam:1", "gog:2"], games.Select(g => g.Id));
        Assert.Equal([GameStore.Epic], failures);
        Assert.Equal("1 jogo instalado", sources.Reports[GameStore.Steam].Summary);
        Assert.Equal("Não foi possível ler os jogos da Epic Games; veja a pasta de logs", sources.Reports[GameStore.Epic].Summary);
        Assert.Equal("Sem GOG Galaxy: os jogos abrem direto pelo executável · 1 jogo instalado", sources.Reports[GameStore.Gog].Summary);
        Assert.Equal("Desligada: os jogos do Xbox não aparecem na biblioteca", sources.Reports[GameStore.Xbox].Summary);
        Assert.Equal([GameStore.Steam, GameStore.Epic, GameStore.Gog, GameStore.Xbox], sources.Stores);
    }

    [Fact]
    public void Steam_source_reports_missing_client()
    {
        var scan = new SteamLibrarySource(null).Scan();

        Assert.Empty(scan.Games);
        Assert.Equal("Steam não encontrada neste PC", scan.Notice);
    }

    [Theory]
    [InlineData(false, false, false, "App Xbox não encontrado")]
    [InlineData(true, false, false, "Serviços de Jogos não encontrados")]
    [InlineData(true, true, true, null)]
    public void Xbox_source_reports_missing_xbox_app_or_gaming_services(bool xboxApp, bool gamingServices, bool clientInstalled, string? notice)
    {
        var scan = new XboxLibrarySource(new BareXboxSystem(xboxApp, gamingServices)).Scan();

        Assert.Empty(scan.Games);
        Assert.Equal(clientInstalled, scan.ClientInstalled);
        if (notice is null)
        {
            Assert.Null(scan.Notice);
        }
        else
        {
            Assert.StartsWith(notice, scan.Notice, StringComparison.Ordinal);
            Assert.StartsWith(notice, new LibrarySourceReport(GameStore.Xbox, true, 0, scan.ClientInstalled, scan.Notice, false).Summary, StringComparison.Ordinal);
        }
    }

    // ------------------------------------------------------------------ Mesmo jogo em duas lojas (STORE-010)

    [Fact]
    public void Same_game_in_two_stores_keeps_both_items_linked()
    {
        var steam = SteamGames.ToGame(new SteamApp(4242, "Corrida Noturna", @"C:\Steam\common\Corrida", 1, DateTimeOffset.UnixEpoch, null, TimeSpan.Zero));
        var epic = StoreGame("epic:Corrida Noturna", "Corrida Noturna™", GameStore.Epic);
        var other = StoreGame("gog:9", "Cidade Submersa", GameStore.Gog);

        StoreDuplicates.Link([steam, epic, other]);

        Assert.Equal([GameStore.Epic], steam.AlsoInstalledIn);
        Assert.Equal([GameStore.Steam], epic.AlsoInstalledIn);
        Assert.Empty(other.AlsoInstalledIn);
        Assert.Equal(4242, epic.SteamAppId); // capa e dados da cópia da Steam, sem esperar a busca por nome
        Assert.Equal(new Uri("steam-art://4242/cover"), epic.CoverUri);
        Assert.Null(other.SteamAppId);
    }

    [Fact]
    public void Same_title_in_one_store_is_not_linked()
    {
        var a = StoreGame("epic:A", "Farol", GameStore.Epic);
        var b = StoreGame("epic:B", "Farol", GameStore.Epic);

        StoreDuplicates.Link([a, b]);

        Assert.Empty(a.AlsoInstalledIn);
        Assert.Empty(b.AlsoInstalledIn);
    }

    [Fact]
    public void Library_links_duplicates_and_keeps_each_store_identity()
    {
        string file = Path.Combine(Path.GetTempPath(), $"tako-stores-{Guid.NewGuid():N}.json");
        try
        {
            var library = new LocalGameLibrary(
                () => [StoreGame("steam:7", "Farol", GameStore.Steam, steamAppId: 7), StoreGame("epic:Farol", "Farol do Fim", GameStore.Epic), StoreGame("gog:7", "FAROL", GameStore.Gog)],
                new LibraryStore(file),
                TimeProvider.System);
            library.Load();

            Assert.Equal(3, library.Games.Count);
            Assert.Equal([GameStore.Steam], library.Find("gog:7")!.AlsoInstalledIn);
            Assert.Empty(library.Find("epic:Farol")!.AlsoInstalledIn);
        }
        finally
        {
            File.Delete(file);
        }
    }

    // ------------------------------------------------------------------ Apoio

    private static FakeRegistry GogRegistry() => new FakeRegistry()
        .Set(GogLibrarySource.GamesKey + @"\1400000001", "gameID", "1400000001")
        .Set(GogLibrarySource.GamesKey + @"\1400000001", "gameName", "Cidade Submersa")
        .Set(GogLibrarySource.GamesKey + @"\1400000001", "path", CidadeFolder)
        .Set(GogLibrarySource.GamesKey + @"\1400000002", "gameName", "Cidade Submersa - Expansão")
        .Set(GogLibrarySource.GamesKey + @"\1400000002", "path", CidadeFolder)
        .Set(GogLibrarySource.GamesKey + @"\1207000000", "gameName", "Desinstalado")
        .Set(GogLibrarySource.GamesKey + @"\1207000000", "path", @"F:\Sumiu");

    /// <summary>Os <c>goggame-*.info</c> de exemplo, como se estivessem na pasta do jogo.</summary>
    private static string? GogInfoFiles(string path) => path.StartsWith(CidadeFolder + "\\goggame-", StringComparison.Ordinal)
        ? File.ReadAllText(Path.Combine(Fixtures, "Gog", path[(CidadeFolder.Length + 1)..]))
        : null;

    private static Func<string, bool> ExistingFolders(params string[] folders) => path => folders.Contains(path, StringComparer.OrdinalIgnoreCase);

    private static Game StoreGame(string id, string title, GameStore store, int? steamAppId = null) => new()
    {
        Id = id,
        Title = title,
        Store = store,
        Accent = "#000000",
        IsInstalled = true,
        SteamAppId = steamAppId,
    };

    private sealed class StubSource(GameStore store, LibraryScan? scan) : ILibrarySource
    {
        public GameStore Store => store;

        public LibraryScan Scan() => scan ?? throw new IOException("disco indisponível");
    }

    /// <summary>Windows sem jogos do Xbox; com ou sem o app Xbox e os Serviços de Jogos.</summary>
    private sealed class BareXboxSystem(bool xboxApp, bool gamingServices) : IXboxSystem
    {
        public IReadOnlyList<string> DriveRoots() => [];

        public IReadOnlyList<InstalledPackage>? UserPackages() =>
            xboxApp ? [new InstalledPackage("Microsoft.GamingApp_2510.1001.1.0_x64__8wekyb3d8bbwe", "/nao-existe")] : [];

        public IReadOnlyCollection<string> GamingServicesPackages() => [];

        public bool GamingServicesInstalled() => gamingServices;

        public string? LoadIndirectString(string indirect) => null;
    }

    /// <summary>Registro em memória: chave → valores (o valor padrão da chave é o nome vazio).</summary>
    private sealed class FakeRegistry : IRegistryReader
    {
        private readonly Dictionary<string, Dictionary<string, string>> _keys = new(StringComparer.OrdinalIgnoreCase);

        public FakeRegistry Set(string key, string name, string value)
        {
            if (!_keys.TryGetValue(key, out var values))
            {
                _keys[key] = values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            }

            values[name] = value;
            return this;
        }

        public IReadOnlyList<string> SubKeyNames(string keyPath) =>
            [.. _keys.Keys
                .Where(k => k.StartsWith(keyPath + "\\", StringComparison.OrdinalIgnoreCase))
                .Select(k => k[(keyPath.Length + 1)..].Split('\\')[0])
                .Distinct(StringComparer.OrdinalIgnoreCase)];

        public string? GetString(string keyPath, string valueName) =>
            _keys.TryGetValue(keyPath, out var values) ? values.GetValueOrDefault(valueName) : null;
    }
}
