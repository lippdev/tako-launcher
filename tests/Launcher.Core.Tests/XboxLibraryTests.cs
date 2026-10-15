using System.Buffers.Binary;
using System.Text;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Xbox;
using Launcher.Core.Presentation;
using Launcher.Core.Session;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Fonte Xbox / Microsoft Store (STORE-004) com manifestos de exemplo em <c>Fixtures/Xbox</c>.</summary>
public sealed class XboxLibraryTests : IDisposable
{
    private const string Microsoft = "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US";
    private const string Contoso = "CN=Contoso Games, O=Contoso, L=Seattle, S=Washington, C=US";
    private const string Fabrikam = "CN=Fabrikam, O=Fabrikam, C=US";

    private static readonly string Fixtures = Path.Combine(AppContext.BaseDirectory, "Fixtures", "Xbox");

    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-xbox-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    [Fact]
    public void Publisher_id_matches_the_one_windows_computes()
    {
        Assert.Equal("8wekyb3d8bbwe", PackageIdentity.PublisherId(Microsoft));
        Assert.Equal("Microsoft.GamingApp_8wekyb3d8bbwe", PackageIdentity.FamilyName("Microsoft.GamingApp", Microsoft));
        Assert.Equal(13, PackageIdentity.PublisherId(Contoso).Length);
        Assert.NotEqual(PackageIdentity.PublisherId(Contoso), PackageIdentity.PublisherId(Fabrikam));
    }

    [Fact]
    public void Full_names_give_the_family_and_launch_uses_the_aumid()
    {
        Assert.True(PackageIdentity.TryFamilyFromFullName("Microsoft.GamingApp_2509.1001.27.0_x64__8wekyb3d8bbwe", out string family));
        Assert.Equal("Microsoft.GamingApp_8wekyb3d8bbwe", family);
        Assert.False(PackageIdentity.TryFamilyFromFullName("Microsoft.GamingApp_8wekyb3d8bbwe", out _));
        Assert.False(PackageIdentity.TryFamilyFromFullName(null, out _));
        Assert.Equal("Microsoft.GamingApp", PackageIdentity.NameOf(family));
        Assert.Equal(@"shell:AppsFolder\Contoso.Racing_abc!Game", PackageIdentity.LaunchTarget("Contoso.Racing_abc", "Game"));
    }

    [Theory]
    [InlineData("ms-resource:AppName", "@{P_1_x64__abc?ms-resource://Contoso.Racing/resources/AppName}")]
    [InlineData("ms-resource:Strings/AppName", "@{P_1_x64__abc?ms-resource://Contoso.Racing/Strings/AppName}")]
    [InlineData("ms-resource:/Strings/AppName", "@{P_1_x64__abc?ms-resource://Contoso.Racing/Strings/AppName}")]
    [InlineData("ms-resource://Contoso.Racing/resources/AppName", "@{P_1_x64__abc?ms-resource://Contoso.Racing/resources/AppName}")]
    [InlineData("Contoso Racing", null)]
    [InlineData(null, null)]
    public void Resource_names_become_indirect_strings(string? value, string? expected) =>
        Assert.Equal(expected, PackageIdentity.IndirectString("P_1_x64__abc", "Contoso.Racing", value));

    [Fact]
    public void Microsoft_game_config_is_read_from_the_fixture()
    {
        var config = MicrosoftGameConfig.Parse(Fixture("MicrosoftGame.config"))!;

        Assert.Equal("Contoso.ContosoRacing", config.IdentityName);
        Assert.Equal(Contoso, config.Publisher);
        Assert.Equal("Contoso.ContosoRacing_" + PackageIdentity.PublisherId(Contoso), config.FamilyName);
        Assert.Equal([new GameExecutable("gamelaunchhelper.exe", "Game"), new GameExecutable("ContosoRacing_DX11.exe", "GameDX11")], config.Executables);
        Assert.Equal("Contoso Racing", config.DisplayName);
        Assert.Equal("Square480.png", config.Square480x480Logo);
        Assert.Equal("SplashScreen.png", config.SplashScreenImage);
        Assert.Equal("#1e3a5f", config.BackgroundColor);
        Assert.Equal("9NCONTOSO001", config.StoreId);
        Assert.Equal("1A2B3C4D", config.TitleId);
    }

    [Theory]
    [InlineData("")]
    [InlineData("<Game><ExecutableList /></Game>")]
    [InlineData("<Package><Identity Name=\"A\" Publisher=\"CN=A\" /></Package>")]
    [InlineData("<!DOCTYPE Game [<!ENTITY x \"y\">]><Game><Identity Name=\"&x;\" Publisher=\"CN=A\" /></Game>")]
    [InlineData("<Game><Identity Name=\"A\"")]
    public void Invalid_game_config_is_ignored(string xml) => Assert.Null(MicrosoftGameConfig.Parse(xml));

    [Fact]
    public void Appx_manifest_main_app_is_the_first_visible_one()
    {
        var manifest = AppxManifest.Parse(Fixture("AppxManifest.xml"))!;

        Assert.Equal("Contoso.ContosoRacing", manifest.IdentityName);
        Assert.Equal("ms-resource:AppDisplayName", manifest.DisplayName);
        Assert.False(manifest.IsFrameworkOrResource);
        Assert.Equal(["Game", "GameDX11"], manifest.Applications.Select(a => a.Id));
        Assert.True(manifest.Applications[1].Hidden);
        Assert.Equal("Game", manifest.MainApplication!.Id);
        Assert.Equal("SplashScreen.png", manifest.MainApplication.SplashScreen);

        var uwp = AppxManifest.Parse(Fixture("AppxManifest-Uwp.xml"))!;
        Assert.Equal(@"Assets\Square310x310Logo.png", uwp.MainApplication!.Square310x310Logo);
        Assert.Equal(@"Assets\Wide310x150Logo.png", uwp.MainApplication.Wide310x150Logo);
    }

    [Fact]
    public void Framework_packages_are_flagged()
    {
        var manifest = AppxManifest.Parse("""
            <Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
              <Identity Name="Microsoft.VCLibs.140.00" Publisher="CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US" />
              <Properties><Framework>true</Framework><DisplayName>VCLibs</DisplayName></Properties>
            </Package>
            """)!;

        Assert.True(manifest.IsFrameworkOrResource);
        Assert.Null(manifest.MainApplication);
        Assert.Equal("Microsoft.VCLibs.140.00_8wekyb3d8bbwe", manifest.FamilyName);
    }

    [Fact]
    public void Gaming_root_lists_the_folders_of_each_drive()
    {
        string drive = Path.Combine(_root, "D") + Path.DirectorySeparatorChar;
        byte[] data = GamingRootBytes(@"\XboxGames", @"E:\Jogos Xbox");

        Assert.Equal([@"\XboxGames", @"E:\Jogos Xbox"], GamingRoot.Parse(data));
        Assert.Equal([Path.Combine(drive, "XboxGames"), @"E:\Jogos Xbox"], GamingRoot.Folders(drive, data));

        // Arquivo ausente ou estranho: a pasta padrão do app Xbox.
        Assert.Empty(GamingRoot.Parse("XXXX\u0001\0\0\0"u8));
        Assert.Empty(GamingRoot.Parse([]));
        Assert.Equal([Path.Combine(drive, "XboxGames")], GamingRoot.Folders(drive, []));

        // Quantidade declarada maior que o conteúdo, sem o zero final: lê o que houver.
        byte[] truncated = [.. "RGBX"u8, 9, 0, 0, 0, .. Encoding.Unicode.GetBytes("Jogos")];
        Assert.Equal(["Jogos"], GamingRoot.Parse(truncated));
    }

    [Theory]
    [InlineData("Logo.png", "Logo.scale-200.png")]
    [InlineData("logo.PNG", "Logo.scale-200.png")]
    [InlineData("Icon.png", "Icon.targetsize-256.png")]
    [InlineData("Exact.png", "Exact.png")]
    [InlineData("Missing.png", null)]
    public void Package_assets_pick_the_largest_plain_variant(string declared, string? expected)
    {
        string[] files =
        [
            "Logo.scale-100.png", "Logo.scale-200.png", "Logo.contrast-black_scale-400.png", "Logo.altform-unplated_scale-400.png", "Logo.jpg",
            "Icon.targetsize-48.png", "Icon.targetsize-256.png", "Exact.png", "Exact.scale-400.png", "LogoExtra.png",
        ];

        Assert.Equal(expected, PackageAssets.Pick(declared, files));
    }

    [Fact]
    public void Package_assets_resolve_inside_the_package_only()
    {
        string package = Path.Combine(_root, "pkg");
        Touch(package, "Assets/Square150x150Logo.scale-200.png");

        Assert.Equal(Path.Combine(package, "Assets", "Square150x150Logo.scale-200.png"), PackageAssets.Resolve(package, @"Assets\Square150x150Logo.png"));
        Assert.Null(PackageAssets.Resolve(package, @"..\outro\Logo.png"));
        Assert.Null(PackageAssets.Resolve(package, @"Nada\Logo.png"));
        Assert.Null(PackageAssets.Resolve(package, null));
    }

    [Fact]
    public void Games_are_told_apart_from_apps()
    {
        var game = AppxManifest.Parse(Fixture("AppxManifest.xml"));
        var app = AppxManifest.Parse(Fixture("AppxManifest-App.xml"));

        Assert.True(XboxGameRules.IsGame("Contoso.ContosoRacing_x", hasGameConfig: true, false, false, game));
        Assert.True(XboxGameRules.IsGame("Contoso.ContosoRacing_x", hasGameConfig: true, false, false, manifest: null));
        Assert.True(XboxGameRules.IsGame("Fabrikam.PuzzleQuest_x", false, hasXboxServicesConfig: true, false, app));
        Assert.True(XboxGameRules.IsGame("Fabrikam.Other_x", false, false, knownToGamingServices: true, app));
        Assert.False(XboxGameRules.IsGame("Fabrikam.NotesApp_x", false, false, false, app));
        // Apps do próprio Xbox têm configuração do Xbox Live, mas não são jogos.
        Assert.False(XboxGameRules.IsGame("Microsoft.GamingApp_8wekyb3d8bbwe", false, true, true, app));
        Assert.False(XboxGameRules.IsGame("Microsoft.XboxGamingOverlay_8wekyb3d8bbwe", true, true, true, game));
        // Sem app visível e sem MicrosoftGame.config: nada para abrir.
        Assert.False(XboxGameRules.IsGame("Fabrikam.Service_x", false, true, true, game! with { Applications = [] }));
    }

    [Fact]
    public void Rules_give_color_title_and_availability()
    {
        Assert.Equal("#1E3A5F", XboxGameRules.Accent("#1e3a5f"));
        Assert.Equal("#1E3A5F", XboxGameRules.Accent("transparent", null, "#1E3A5F"));
        Assert.Equal(XboxGameRules.BrandColor, XboxGameRules.Accent("transparent", "#12", "#GGGGGG"));

        Assert.Equal("Contoso Racing", XboxGameRules.FallbackTitle("Contoso.ContosoRacing_x", "Contoso Racing"));
        Assert.Equal("ContosoRacing", XboxGameRules.FallbackTitle("Contoso.ContosoRacing_x", "Content"));
        Assert.Equal("Solo", XboxGameRules.FallbackTitle("Solo_x", null));

        Assert.Equal(["Contoso.Racing_abcdefghjkmnp", "Fabrikam.Quest_abcdefghjkmnp"],
            XboxGameRules.Families(["Contoso.Racing_1.0.0.0_x64__abcdefghjkmnp", "Fabrikam.Quest_abcdefghjkmnp", "lixo", "a_b"]).Order());

        string[] ready = [XboxGameRules.XboxAppFamily];
        Assert.Equal(XboxAvailability.Ready, XboxGameRules.Availability(ready, gamingServicesInstalled: true));
        Assert.Equal(XboxAvailability.NoGamingServices, XboxGameRules.Availability(ready, gamingServicesInstalled: false));
        Assert.Equal(XboxAvailability.NoXboxApp, XboxGameRules.Availability([], gamingServicesInstalled: true));
        Assert.Equal(XboxAvailability.Unknown, XboxGameRules.Availability(null, gamingServicesInstalled: true));

        Assert.Equal("PC Game Pass e Microsoft Store · 2 jogos encontrados", XboxGameRules.Describe(true, XboxAvailability.Ready, 2));
        Assert.StartsWith("App Xbox não encontrado", XboxGameRules.Describe(true, XboxAvailability.NoXboxApp, 0), StringComparison.Ordinal);
        Assert.StartsWith("Serviços de Jogos não encontrados", XboxGameRules.Describe(true, XboxAvailability.NoGamingServices, 1), StringComparison.Ordinal);
        Assert.StartsWith("Desligado", XboxGameRules.Describe(false, XboxAvailability.NoXboxApp, 0), StringComparison.Ordinal);
    }

    [Fact]
    public void Scanner_finds_gdk_and_uwp_games_and_skips_apps()
    {
        var system = BuildMachine();

        var result = XboxLibraryScanner.Scan(system);

        Assert.Equal(XboxAvailability.Ready, result.Availability);
        Assert.Equal(["Contoso Racing", "Fabrikam Puzzle Quest"], result.Games.Select(g => g.Title));

        var racing = result.Games[0];
        Assert.Equal(ContosoFamily, racing.FamilyName);
        Assert.Equal("Game", racing.AppId);
        Assert.Equal($@"shell:AppsFolder\{ContosoFamily}!Game", racing.LaunchTarget);
        Assert.Equal(Path.Combine(_root, "D", "XboxGames", "Contoso Racing", "Content"), racing.Folder);
        Assert.EndsWith("Square480.scale-200.png", racing.CoverPath, StringComparison.Ordinal);
        Assert.EndsWith("SplashScreen.scale-100.png", racing.HeroPath, StringComparison.Ordinal);
        Assert.Equal("#1E3A5F", racing.Accent);
        Assert.Equal("9NCONTOSO001", racing.StoreId);

        var puzzle = result.Games[1];
        Assert.Equal("App", puzzle.AppId);
        Assert.EndsWith("Square310x310Logo.scale-100.png", puzzle.CoverPath, StringComparison.Ordinal);
        Assert.Null(puzzle.HeroPath);
        Assert.Equal(XboxGameRules.BrandColor, puzzle.Accent);
    }

    [Fact]
    public void Scanner_resolves_resource_titles_and_skips_unregistered_folders()
    {
        var system = BuildMachine();
        string config = Path.Combine(_root, "D", "XboxGames", "Contoso Racing", "Content", MicrosoftGameConfig.FileName);
        File.WriteAllText(config, File.ReadAllText(config).Replace("DefaultDisplayName=\"Contoso Racing\"", "DefaultDisplayName=\"ms-resource:GameTitle\"", StringComparison.Ordinal));
        system.Resources[$"@{{{ContosoFullName}?ms-resource://Contoso.ContosoRacing/resources/GameTitle}}"] = "Corrida Contoso";

        var titles = XboxLibraryScanner.Scan(system).Games.Select(g => g.Title).ToList();
        Assert.Contains("Corrida Contoso", titles);

        // Sem o recurso: o próximo nome do manifesto também é ms-resource; fica o nome da pasta.
        system.Resources.Clear();
        Assert.Contains("Contoso Racing", XboxLibraryScanner.Scan(system).Games.Select(g => g.Title));

        // Pacote desinstalado para este usuário: a pasta ainda existe, mas o Windows não abriria o jogo.
        system.Packages!.RemoveAll(p => p.FullName == ContosoFullName);
        Assert.DoesNotContain(XboxLibraryScanner.Scan(system).Games, g => g.FamilyName == ContosoFamily);
    }

    [Fact]
    public void Scanner_without_package_list_still_reads_game_folders()
    {
        var system = BuildMachine();
        system.Packages = null;

        var result = XboxLibraryScanner.Scan(system);

        // Sem a lista não dá para conferir o registro: vale o que está nas pastas do app Xbox.
        Assert.Equal(XboxAvailability.Unknown, result.Availability);
        Assert.Equal([ContosoFamily, PackageIdentity.FamilyName("Contoso.Orphan", Contoso)], result.Games.Select(g => g.FamilyName).Order(StringComparer.Ordinal));
    }

    [Fact]
    public void Scanner_survives_a_failing_system()
    {
        var result = XboxLibraryScanner.Scan(new FakeXboxSystem { Fail = true });

        Assert.Empty(result.Games);
        Assert.Equal(XboxAvailability.Unknown, result.Availability);
    }

    [Fact]
    public void Xbox_games_open_by_aumid_and_track_the_package_folder()
    {
        var game = XboxGames.ToGame(new XboxInstalledGame
        {
            FamilyName = ContosoFamily,
            AppId = "Game",
            Title = "Contoso Racing",
            Folder = @"C:\XboxGames\Contoso Racing\Content",
            CoverPath = Path.Combine(_root, "cover.png"),
            Accent = "#1E3A5F",
        });

        Assert.Equal("xbox:" + ContosoFamily, game.Id);
        Assert.Equal(GameStore.Xbox, game.Store);
        Assert.Equal($@"shell:AppsFolder\{ContosoFamily}!Game", game.LaunchTarget);
        Assert.True(game.CanPlay);
        Assert.True(game.HasFallbackArt);
        Assert.Equal(game.CoverUri, game.HeroUri);

        // O shell não devolve processo: acompanha pelos processos dentro da pasta do pacote.
        var plan = GameTracking.Plan(game, hasProcess: false);
        Assert.Equal(new TrackingPlan(GameTrackingStrategy.InstallFolder, @"C:\XboxGames\Contoso Racing\Content"), plan);
        var tracker = new GameRunTracker(plan, DateTimeOffset.UnixEpoch, exitGrace: TimeSpan.FromSeconds(4));
        tracker.Update([new ProcessEntry(10, 1, @"C:\XboxGames\Contoso Racing\Content\ContosoRacing.exe")], DateTimeOffset.UnixEpoch.AddSeconds(5));
        Assert.Equal(GameRunStatus.Running, tracker.Status);
        Assert.Equal(GameRunStatus.Exited, tracker.Update([], DateTimeOffset.UnixEpoch.AddSeconds(10)));
    }

    [Fact]
    public void Metadata_art_replaces_the_package_logo()
    {
        var game = XboxGames.ToGame(new XboxInstalledGame
        {
            FamilyName = ContosoFamily,
            AppId = "Game",
            Title = "Contoso Racing",
            Folder = _root,
            CoverPath = Path.Combine(_root, "cover.png"),
        });

        new GameMetadata { Source = "Steam Store", SteamAppId = 570 }.ApplyTo(game);

        Assert.Equal(new Uri("steam-art://570/cover"), game.CoverUri);
        Assert.Equal(new Uri("steam-art://570/hero"), game.HeroUri);
    }

    [Theory]
    [InlineData(0, null, "Nenhum jogo da Steam encontrado neste PC")]
    [InlineData(0, 0, "Nenhum jogo da Steam ou do Xbox encontrado neste PC")]
    [InlineData(12, null, "12 jogos da Steam encontrados")]
    [InlineData(12, 0, "12 jogos da Steam encontrados")]
    [InlineData(0, 1, "1 jogo do Xbox encontrado")]
    [InlineData(12, 3, "12 jogos da Steam e 3 do Xbox encontrados")]
    public void Scan_summary_mentions_each_store(int steam, int? xbox, string expected)
    {
        List<(GameStore, int)> enabled = [(GameStore.Steam, steam)];
        if (xbox is int count)
        {
            enabled.Add((GameStore.Xbox, count));
        }

        Assert.Equal(expected, LibraryText.ScanSummary(enabled));
    }

    [Fact]
    public void Scan_summary_lists_every_enabled_store()
    {
        Assert.Equal("12 jogos da Steam, 2 da Epic Games e 1 do GOG encontrados", LibraryText.ScanSummary([(GameStore.Steam, 12), (GameStore.Epic, 2), (GameStore.Xbox, 0), (GameStore.Gog, 1)]));
        Assert.Equal("Nenhum jogo da Steam, da Epic Games ou do GOG encontrado neste PC", LibraryText.ScanSummary([(GameStore.Steam, 0), (GameStore.Epic, 0), (GameStore.Gog, 0)]));
        Assert.Equal("Todas as lojas estão desligadas nas Configurações", LibraryText.ScanSummary([]));
    }

    private static string ContosoFamily => PackageIdentity.FamilyName("Contoso.ContosoRacing", Contoso);

    private static string ContosoFullName => $"Contoso.ContosoRacing_1.4.2.0_x64__{PackageIdentity.PublisherId(Contoso)}";

    /// <summary>
    /// Máquina de exemplo: unidade D com <c>.GamingRoot</c> e um jogo do GDK, um jogo UWP antigo e um app comum em
    /// WindowsApps, o app Xbox e uma pasta de jogo sem pacote registrado.
    /// </summary>
    private FakeXboxSystem BuildMachine()
    {
        string drive = Path.Combine(_root, "D");
        Directory.CreateDirectory(drive);
        File.WriteAllBytes(Path.Combine(drive, GamingRoot.FileName), GamingRootBytes("XboxGames"));

        string racing = Path.Combine(drive, "XboxGames", "Contoso Racing", "Content");
        Copy("MicrosoftGame.config", racing, MicrosoftGameConfig.FileName);
        Copy("AppxManifest.xml", racing, AppxManifest.FileName);
        Touch(racing, "Square480.scale-100.png");
        Touch(racing, "Square480.scale-200.png");
        Touch(racing, "SplashScreen.scale-100.png");
        Touch(racing, "gamelaunchhelper.exe");

        // Jogo de outro usuário (ou instalação incompleta): pasta sem pacote registrado.
        string orphan = Path.Combine(drive, "XboxGames", "Orphan", "Content");
        Directory.CreateDirectory(orphan);
        File.WriteAllText(Path.Combine(orphan, MicrosoftGameConfig.FileName), Fixture("MicrosoftGame.config").Replace("Contoso.ContosoRacing", "Contoso.Orphan", StringComparison.Ordinal));

        string apps = Path.Combine(_root, "WindowsApps");
        string puzzle = Path.Combine(apps, "Fabrikam.PuzzleQuest_3.1.0.0_x64__x");
        Copy("AppxManifest-Uwp.xml", puzzle, AppxManifest.FileName);
        Touch(puzzle, XboxGameRules.XboxServicesConfig);
        Touch(puzzle, "Assets/Square310x310Logo.scale-100.png");
        string notes = Path.Combine(apps, "Fabrikam.NotesApp_2.0.0.0_x64__x");
        Copy("AppxManifest-App.xml", notes, AppxManifest.FileName);
        string xboxApp = Path.Combine(apps, "Microsoft.GamingApp");
        Touch(xboxApp, XboxGameRules.XboxServicesConfig);

        string fabrikam = PackageIdentity.PublisherId(Fabrikam);
        return new FakeXboxSystem
        {
            Drives = [drive],
            Packages =
            [
                new InstalledPackage(ContosoFullName, racing),
                new InstalledPackage($"Fabrikam.PuzzleQuest_3.1.0.0_x64__{fabrikam}", puzzle),
                new InstalledPackage($"Fabrikam.NotesApp_2.0.0.0_x64__{fabrikam}", notes),
                new InstalledPackage("Microsoft.GamingApp_2509.1001.27.0_x64__8wekyb3d8bbwe", xboxApp),
                new InstalledPackage($"Fabrikam.Removed_1.0.0.0_x64__{fabrikam}", Path.Combine(apps, "nao-existe")),
            ],
            GamingServices = [ContosoFullName],
        };
    }

    private static byte[] GamingRootBytes(params string[] folders)
    {
        var bytes = new List<byte>("RGBX"u8.ToArray());
        var count = new byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(count, (uint)folders.Length);
        bytes.AddRange(count);
        foreach (string folder in folders)
        {
            bytes.AddRange(Encoding.Unicode.GetBytes(folder + "\0"));
        }

        return [.. bytes];
    }

    private static string Fixture(string name) => File.ReadAllText(Path.Combine(Fixtures, name));

    private static void Copy(string fixture, string folder, string name)
    {
        Directory.CreateDirectory(folder);
        File.Copy(Path.Combine(Fixtures, fixture), Path.Combine(folder, name));
    }

    private static void Touch(string folder, string relative)
    {
        string path = Path.Combine(folder, relative);
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllBytes(path, []);
    }

    private sealed class FakeXboxSystem : IXboxSystem
    {
        public bool Fail { get; init; }

        public List<string> Drives { get; init; } = [];

        public List<InstalledPackage>? Packages { get; set; } = [];

        public List<string> GamingServices { get; init; } = [];

        public Dictionary<string, string> Resources { get; } = [];

        public IReadOnlyList<string> DriveRoots() => Fail ? throw new IOException("falha") : Drives;

        public IReadOnlyList<InstalledPackage>? UserPackages() => Fail ? throw new UnauthorizedAccessException() : Packages;

        public IReadOnlyCollection<string> GamingServicesPackages() => Fail ? throw new System.Security.SecurityException() : GamingServices;

        public bool GamingServicesInstalled() => !Fail;

        public string? LoadIndirectString(string indirect) => Resources.GetValueOrDefault(indirect);
    }
}
