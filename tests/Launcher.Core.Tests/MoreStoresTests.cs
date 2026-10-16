using System.Text;
using Launcher.Core.Library;
using Launcher.Core.Library.Amazon;
using Launcher.Core.Library.BattleNet;
using Launcher.Core.Library.Ea;
using Launcher.Core.Library.Sources;
using Launcher.Core.Library.Ubisoft;
using Launcher.Core.Presentation;
using Microsoft.Data.Sqlite;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Ubisoft Connect, EA app, Battle.net e Amazon Games (STORE-005 a 008, 011 e 012) com registro e arquivos falsos.</summary>
public sealed class MoreStoresTests
{
    private const string Uninstall = @"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
    private static readonly string EaFixtures = Path.Combine(AppContext.BaseDirectory, "Fixtures", "Stores", "Ea");

    // ------------------------------------------------------------------ Ubisoft Connect

    [Fact]
    public void Ubisoft_reads_installs_with_uninstall_name_or_folder_and_launches_by_protocol()
    {
        var registry = new FakeRegistry()
            .Set(UbisoftLibrarySource.InstallsKeys[0] + @"\635", "InstallDir", "D:/Ubisoft/Far Horizon 5/")
            .Set(UbisoftLibrarySource.InstallsKeys[0] + @"\4311", "InstallDir", @"D:\Ubisoft\Sem Nome")
            .Set(UbisoftLibrarySource.InstallsKeys[0] + @"\999", "InstallDir", @"D:\Ubisoft\Sumiu")
            .Set(UbisoftLibrarySource.InstallsKeys[0] + @"\launcher", "InstallDir", @"D:\Ubisoft\Nao e jogo")
            .Set(Uninstall + @"\Uplay Install 635", "DisplayName", "Far Horizon 5")
            .Set(UbisoftLibrarySource.ProtocolKey, string.Empty, "\"C:\\Ubisoft\\upc.exe\" \"%1\"");
        var source = new UbisoftLibrarySource(registry, Folders(@"D:\Ubisoft\Far Horizon 5", @"D:\Ubisoft\Sem Nome", @"D:\Ubisoft\Nao e jogo"));

        var scan = source.Scan();

        Assert.True(scan.ClientInstalled);
        Assert.Null(scan.Notice);
        Assert.Equal(["Far Horizon 5", "Sem Nome"], scan.Games.Select(g => g.Title));
        var game = scan.Games[0];
        Assert.Equal("ubisoft:635", game.Id);
        Assert.Equal(GameStore.Ubisoft, game.Store);
        Assert.Equal("uplay://launch/635/0", game.LaunchTarget);
        Assert.Equal(@"D:\Ubisoft\Far Horizon 5", game.InstallPath); // pasta normalizada: a sessão acompanha por ela
        Assert.Equal("635", game.StoreProductId);
    }

    [Fact]
    public void Ubisoft_without_client_keeps_games_and_warns()
    {
        var registry = new FakeRegistry().Set(UbisoftLibrarySource.InstallsKeys[1] + @"\5", "InstallDir", @"E:\Jogo");

        var scan = new UbisoftLibrarySource(registry, Folders(@"E:\Jogo")).Scan();

        Assert.Single(scan.Games);
        Assert.False(scan.ClientInstalled);
        Assert.Equal(UbisoftLibrarySource.MissingNotice, scan.Notice);
        Assert.Equal("Ubisoft Connect não encontrada neste PC", new UbisoftLibrarySource(new FakeRegistry()).Scan().Notice);
    }

    [Fact]
    public void Ubisoft_client_found_by_executable_in_launcher_folder()
    {
        var registry = new FakeRegistry().Set(UbisoftLibrarySource.LauncherKey, "InstallDir", @"C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\");

        var source = new UbisoftLibrarySource(registry, path => path == @"C:\Program Files (x86)\Ubisoft\Ubisoft Game Launcher\UbisoftConnect.exe");

        Assert.True(source.ClientInstalled());
        Assert.True(source.Scan().ClientInstalled);
    }

    // ------------------------------------------------------------------ EA app

    [Fact]
    public void Ea_installer_data_gives_content_ids_and_localized_title()
    {
        var data = EaGames.Parse(File.ReadAllText(Path.Combine(EaFixtures, "installerdata-eaapp.xml")));

        Assert.Equal(["1077777", "1077778"], data.ContentIds);
        Assert.Equal("Arena Estelar", data.Title); // pt_BR antes de en_US

        var origin = EaGames.Parse(File.ReadAllText(Path.Combine(EaFixtures, "installerdata-origin.xml")));
        Assert.Equal(["OFB-EAST:55555"], origin.ContentIds);
        Assert.Equal("Deep Sea Station", origin.Title); // sem pt_BR, en_US
    }

    [Fact]
    public void Ea_launch_uri_uses_the_ea_app_protocol_with_every_content_id() =>
        Assert.Equal("origin2://game/launch?offerIds=1077777,OFB-EAST%3A55555&autoDownload=1", EaGames.LaunchUri(["1077777", "OFB-EAST:55555"]));

    [Theory]
    [InlineData(null, null, EaClientKind.Missing)]
    [InlineData("\"C:\\Program Files (x86)\\Origin\\Origin.exe\" \"%1\"", null, EaClientKind.LegacyOrigin)]
    [InlineData("\"C:\\Program Files\\Electronic Arts\\EA Desktop\\EA Desktop\\EALauncher.exe\" \"%1\"", null, EaClientKind.EaApp)]
    [InlineData(null, "\"C:\\Program Files\\Electronic Arts\\EA Desktop\\EA Desktop\\EALaunchHelper.exe\" \"%1\"", EaClientKind.EaApp)]
    public void Ea_client_kind_tells_ea_app_from_old_origin(string? origin2, string? link2ea, EaClientKind expected) =>
        Assert.Equal(expected, EaGames.ClientKind(origin2, link2ea));

    [Fact]
    public void Ea_source_reads_registry_and_uninstall_folders_with_installer_data()
    {
        const string arena = @"C:\Program Files\EA Games\Arena Estelar";
        const string deep = @"D:\Origin Games\Deep Sea Station";
        var registry = new FakeRegistry()
            .Set(EaLibrarySource.GameKeys[0] + @"\Arena Estelar", "Install Dir", arena + "\\")
            .Set(EaLibrarySource.GameKeys[1] + @"\EA Desktop", "InstallLocation", @"C:\Program Files\Electronic Arts\EA Desktop")
            .Set(Uninstall + @"\{DEEP-SEA}", "DisplayName", "Deep Sea Station")
            .Set(Uninstall + @"\{DEEP-SEA}", "Publisher", "Electronic Arts")
            .Set(Uninstall + @"\{DEEP-SEA}", "InstallLocation", deep)
            .Set(Uninstall + @"\{ARENA}", "Publisher", "Electronic Arts, Inc.")
            .Set(Uninstall + @"\{ARENA}", "InstallLocation", arena) // mesma pasta: não duplica
            .Set(EaLibrarySource.Link2EaProtocolKey, string.Empty, "EALaunchHelper.exe \"%1\"");
        var files = new Dictionary<string, string>
        {
            [arena + @"\__Installer\installerdata.xml"] = File.ReadAllText(Path.Combine(EaFixtures, "installerdata-eaapp.xml")),
            [deep + @"\__Installer\installerdata.xml"] = File.ReadAllText(Path.Combine(EaFixtures, "installerdata-origin.xml")),
        };
        var source = new EaLibrarySource(registry, files.GetValueOrDefault, Folders(arena, deep, @"C:\Program Files\Electronic Arts\EA Desktop"));

        var scan = source.Scan();

        Assert.True(scan.ClientInstalled);
        Assert.Null(scan.Notice);
        Assert.Equal(["ea:1077777", "ea:OFB-EAST:55555"], scan.Games.Select(g => g.Id));
        var game = scan.Games[0];
        Assert.Equal("Arena Estelar", game.Title);
        Assert.Equal(GameStore.Ea, game.Store);
        Assert.Equal(arena, game.InstallPath);
        Assert.StartsWith("origin2://game/launch?offerIds=1077777", game.LaunchTarget, StringComparison.Ordinal);
    }

    [Fact]
    public void Ea_with_only_old_origin_is_outdated()
    {
        var registry = new FakeRegistry().Set(EaLibrarySource.Origin2ProtocolKey, string.Empty, "\"C:\\Origin\\Origin.exe\" \"%1\"");

        var scan = new EaLibrarySource(registry, _ => null, _ => false).Scan();

        Assert.False(scan.ClientInstalled);
        Assert.Equal(EaLibrarySource.OutdatedNotice, scan.Notice);
        Assert.StartsWith("Só o Origin", new LibrarySourceReport(GameStore.Ea, true, 0, scan.ClientInstalled, scan.Notice, false).Summary, StringComparison.Ordinal);
    }

    // ------------------------------------------------------------------ Battle.net

    [Fact]
    public void Battle_net_product_db_reads_uid_code_path_and_state()
    {
        byte[] db = ProductDbBytes(
            Install("prometheus", "pro", @"C:\Program Files (x86)\Overwatch", installed: true),
            Install("battle.net", "bna", @"C:\Program Files (x86)\Battle.net", installed: true),
            Install("fenris", "fen", @"D:\Diablo IV", installed: false));

        var installs = ProductDb.Parse(db);

        Assert.Equal(3, installs.Count);
        Assert.Equal(new BattleNetInstall("prometheus", "pro", @"C:\Program Files (x86)\Overwatch", true), installs[0]);
        Assert.False(installs[2].Installed);
    }

    [Fact]
    public void Battle_net_product_db_skips_unknown_fields_and_rejects_truncated_files()
    {
        var unknown = new List<byte>();
        unknown.AddRange(Field(9, Encoding.UTF8.GetBytes("ignorado"))); // campo desconhecido, pulado
        unknown.AddRange([0x18, 0x96, 0x01]); // varint no campo 3
        unknown.AddRange(Field(1, Install("s2", "s2", @"C:\StarCraft II", installed: null)));

        var installs = ProductDb.Parse([.. unknown]);

        Assert.Equal("s2", Assert.Single(installs).Uid);
        Assert.Null(installs[0].Installed);
        Assert.Throws<InvalidDataException>(() => ProductDb.Parse(ProductDbBytes(Install("wow", "wow", @"C:\WoW", true)).AsSpan(..^3).ToArray()));
    }

    [Theory]
    [InlineData("prometheus", "Pro")]
    [InlineData("hs_beta", "WTCG")]
    [InlineData("diablo3_enus", "D3")]
    [InlineData("fenris", "Fen")]
    public void Battle_net_products_map_uid_to_launch_code(string uid, string code) =>
        Assert.Equal(code, BattleNetProducts.Find(uid)!.LaunchCode);

    [Theory]
    [InlineData("wow_classic")]
    [InlineData("wow_beta")]
    [InlineData("desconhecido")]
    public void Battle_net_variants_do_not_inherit_another_game_code(string uid) => Assert.Null(BattleNetProducts.Find(uid));

    [Fact]
    public void Battle_net_source_launches_through_client_and_opens_client_for_unknown_games()
    {
        const string client = @"C:\Program Files (x86)\Battle.net";
        var registry = new FakeRegistry()
            .Set(Uninstall + @"\Battle.net", "InstallLocation", client)
            .Set(Uninstall + @"\Battle.net", "UninstallString", "\"C:\\ProgramData\\Battle.net\\Agent\\Blizzard Uninstaller.exe\" --lang=enUS --uid=battle.net")
            .Set(Uninstall + @"\Classic", "DisplayName", "World of Warcraft Classic")
            .Set(Uninstall + @"\Classic", "UninstallString", "\"C:\\ProgramData\\Battle.net\\Agent\\Blizzard Uninstaller.exe\" --lang=enUS --uid=wow_classic --displayname=\"World of Warcraft Classic\"");
        byte[] db = ProductDbBytes(
            Install("prometheus", "pro", @"C:\Jogos\Overwatch", installed: true),
            Install("wow_classic", "wow_classic", @"C:\Jogos\WoW Classic", installed: true),
            Install("battle.net", "bna", client, installed: true),
            Install("fenris", "fen", @"D:\Diablo IV", installed: false));
        var source = new BattleNetLibrarySource(
            registry,
            @"C:\ProgramData",
            path => path == @"C:\ProgramData\Battle.net\Agent\product.db" ? db : null,
            Folders(@"C:\Jogos\Overwatch", @"C:\Jogos\WoW Classic", client, @"D:\Diablo IV", client + @"\Battle.net.exe"));

        var scan = source.Scan();

        Assert.True(scan.ClientInstalled);
        Assert.Equal(["Overwatch 2", "World of Warcraft Classic"], scan.Games.Select(g => g.Title));
        var overwatch = scan.Games[0];
        Assert.Equal("battlenet:prometheus", overwatch.Id);
        Assert.Equal(client + @"\Battle.net.exe", overwatch.LaunchTarget);
        Assert.Equal("--exec=\"launch Pro\"", overwatch.LaunchArguments);
        Assert.Equal(@"C:\Jogos\Overwatch", overwatch.InstallPath);
        var classic = scan.Games[1];
        Assert.Equal(client + @"\Battle.net.exe", classic.LaunchTarget); // abre o cliente para o usuário escolher
        Assert.Null(classic.LaunchArguments);
    }

    [Fact]
    public void Battle_net_falls_back_to_uninstall_keys_without_product_db()
    {
        var registry = new FakeRegistry()
            .Set(Uninstall + @"\Hearthstone", "DisplayName", "Hearthstone")
            .Set(Uninstall + @"\Hearthstone", "InstallLocation", @"E:\Hearthstone")
            .Set(Uninstall + @"\Hearthstone", "UninstallString", "\"C:\\ProgramData\\Battle.net\\Agent\\Blizzard Uninstaller.exe\" --lang=ptBR --uid=hs_beta --displayname=\"Hearthstone\"")
            .Set(Uninstall + @"\PTR", "DisplayName", "Overwatch Test")
            .Set(Uninstall + @"\PTR", "InstallLocation", @"E:\PTR")
            .Set(Uninstall + @"\PTR", "UninstallString", "Battle.net Uninstaller.exe --uid=prometheus_test");

        var scan = new BattleNetLibrarySource(registry, @"C:\ProgramData", _ => null, Folders(@"E:\Hearthstone", @"E:\PTR")).Scan();

        var game = Assert.Single(scan.Games);
        Assert.Equal("Hearthstone", game.Title);
        Assert.False(scan.ClientInstalled);
        Assert.Equal(BattleNetLibrarySource.MissingNotice, scan.Notice);
        Assert.Equal("battlenet://WTCG", game.LaunchTarget); // sem cliente: falha ao abrir e mostra o aviso
    }

    [Fact]
    public void Battle_net_unreadable_product_db_uses_registry()
    {
        var scan = new BattleNetLibrarySource(new FakeRegistry(), @"C:\ProgramData", _ => [0x0A, 0xFF], _ => false).Scan();

        Assert.Empty(scan.Games);
        Assert.Equal("Battle.net não encontrado neste PC", scan.Notice);
    }

    // ------------------------------------------------------------------ Amazon Games

    [Fact]
    public void Amazon_reads_installed_rows_from_sqlite_and_launches_by_protocol()
    {
        string root = Path.Combine(Path.GetTempPath(), $"tako-amazon-{Guid.NewGuid():N}");
        try
        {
            var source = new AmazonLibrarySource(new FakeRegistry().Set(AmazonLibrarySource.ProtocolKey, string.Empty, "x"), root, Folders(@"C:\Amazon Games\Library\Ilha", @"C:\Amazon Games\Library\Sem Titulo"));
            Directory.CreateDirectory(Path.GetDirectoryName(source.DatabasePath)!);
            using (var connection = new SqliteConnection($"Data Source={source.DatabasePath};Pooling=False"))
            {
                connection.Open();
                using var command = connection.CreateCommand();
                command.CommandText = """
                    CREATE TABLE DbSet (Id TEXT, ProductTitle TEXT, InstallDirectory TEXT, Installed INTEGER, ProductAsin TEXT);
                    INSERT INTO DbSet VALUES ('amzn1.adg.product.ilha', 'Ilha Perdida', 'C:\Amazon Games\Library\Ilha', 1, 'B0');
                    INSERT INTO DbSet VALUES ('amzn1.adg.product.semtitulo', NULL, 'C:\Amazon Games\Library\Sem Titulo\', 1, 'B1');
                    INSERT INTO DbSet VALUES ('amzn1.adg.product.velho', 'Desinstalado', 'C:\Amazon Games\Library\Velho', 0, 'B2');
                    INSERT INTO DbSet VALUES ('amzn1.adg.product.sumiu', 'Sumiu', 'C:\Amazon Games\Library\Sumiu', 1, 'B3');
                    """;
                command.ExecuteNonQuery();
            }

            var scan = source.Scan();

            Assert.True(scan.ClientInstalled);
            Assert.Equal(["Ilha Perdida", "Sem Titulo"], scan.Games.Select(g => g.Title));
            Assert.Equal("amazon:amzn1.adg.product.ilha", scan.Games[0].Id);
            Assert.Equal("amazon-games://play/amzn1.adg.product.ilha", scan.Games[0].LaunchTarget);
            Assert.Equal(@"C:\Amazon Games\Library\Sem Titulo", scan.Games[1].InstallPath);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }

    [Fact]
    public void Amazon_invalid_database_is_an_io_failure_and_missing_client_warns()
    {
        string root = Path.Combine(Path.GetTempPath(), $"tako-amazon-{Guid.NewGuid():N}");
        try
        {
            var source = new AmazonLibrarySource(new FakeRegistry(), root, _ => false);
            Assert.Equal("Amazon Games não encontrado neste PC", source.Scan().Notice);

            Directory.CreateDirectory(Path.GetDirectoryName(source.DatabasePath)!);
            File.WriteAllText(source.DatabasePath, "não é um banco");
            Assert.ThrowsAny<IOException>(() => source.Scan());
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }

    // ------------------------------------------------------------------ Abrir pelo cliente (STORE-011/012) e textos

    [Theory]
    [InlineData(GameStore.Steam, "steam://rungameid/1", " pela Steam")]
    [InlineData(GameStore.Ubisoft, "uplay://launch/1/0", " pela Ubisoft Connect")]
    [InlineData(GameStore.Ea, "origin2://game/launch?offerIds=1", " pelo EA app")]
    [InlineData(GameStore.BattleNet, @"C:\Battle.net\Battle.net.exe", " pelo Battle.net")]
    [InlineData(GameStore.Amazon, "amazon-games://play/1", " pela Amazon Games")]
    [InlineData(GameStore.Gog, @"C:\GOG Galaxy\GalaxyClient.exe", " pelo GOG Galaxy")]
    [InlineData(GameStore.Gog, @"C:\GOG Games\x\x.exe", "")]
    [InlineData(GameStore.Manual, @"C:\x.exe", "")]
    public void Launch_via_names_the_official_client(GameStore store, string target, string expected) =>
        Assert.Equal(expected, LibraryText.LaunchVia(new Game { Id = "x", Title = "X", Store = store, Accent = "#000000", LaunchTarget = target }));

    [Fact]
    public void Launch_failure_shows_the_missing_or_outdated_client_notice_only_for_client_stores()
    {
        var ea = new Game { Id = "ea:1", Title = "Arena", Store = GameStore.Ea, Accent = "#000000" };
        var gog = new Game { Id = "gog:1", Title = "Cidade", Store = GameStore.Gog, Accent = "#000000" };

        Assert.Equal($"{EaLibrarySource.OutdatedNotice} (Arena)", LibraryText.ClientMissing(ea, new LibrarySourceReport(GameStore.Ea, true, 1, false, EaLibrarySource.OutdatedNotice, false)));
        Assert.Null(LibraryText.ClientMissing(ea, new LibrarySourceReport(GameStore.Ea, true, 1, true, null, false)));
        Assert.Null(LibraryText.ClientMissing(gog, new LibrarySourceReport(GameStore.Gog, true, 1, false, "Sem GOG Galaxy: os jogos abrem direto pelo executável", false)));
    }

    [Fact]
    public void New_stores_have_names_for_settings_and_scan_summary()
    {
        Assert.Equal("do EA app", GameText.StoreOf(GameStore.Ea));
        Assert.Equal("do Battle.net", GameText.StoreOf(GameStore.BattleNet));
        Assert.Equal("da Ubisoft Connect", GameText.StoreOf(GameStore.Ubisoft));
        Assert.Equal("pela Amazon Games", GameText.StoreVia(GameStore.Amazon));
        Assert.Equal("2 jogos da Ubisoft Connect e 1 do Battle.net encontrados", LibraryText.ScanSummary([(GameStore.Ubisoft, 2), (GameStore.Ea, 0), (GameStore.BattleNet, 1)]));
        Assert.Equal("Desligada: os jogos da Amazon Games não aparecem na biblioteca", new LibrarySourceReport(GameStore.Amazon, false, 0, false, null, false).Summary);
    }

    [Fact]
    public void Windows_paths_are_normalized_without_the_host_os()
    {
        Assert.Equal(@"D:\Jogos\Farol", WindowsPath.Normalize("D:/Jogos/Farol/"));
        Assert.Equal(@"C:\", WindowsPath.Normalize(@"C:\"));
        Assert.Equal(@"C:\x\y.exe", WindowsPath.Combine(@"C:\", "x", "y.exe"));
        Assert.Equal("Farol", WindowsPath.LastName(@"D:\Jogos\Farol\"));
    }

    // ------------------------------------------------------------------ Apoio

    private static Func<string, bool> Folders(params string[] folders) => path => folders.Contains(path, StringComparer.OrdinalIgnoreCase);

    private static byte[] Install(string uid, string code, string path, bool? installed)
    {
        var message = new List<byte>();
        message.AddRange(Field(1, Encoding.UTF8.GetBytes(uid)));
        message.AddRange(Field(2, Encoding.UTF8.GetBytes(code)));
        message.AddRange(Field(3, [.. Field(1, Encoding.UTF8.GetBytes(path)), .. Field(2, Encoding.UTF8.GetBytes("us"))]));
        if (installed is bool on)
        {
            byte[] state = [0x08, on ? (byte)1 : (byte)0, 0x10, 0x01]; // installed (1) e playable (2)
            message.AddRange(Field(4, Field(1, state)));
        }

        return [.. message];
    }

    private static byte[] ProductDbBytes(params byte[][] installs) => [.. installs.SelectMany(i => Field(1, i))];

    /// <summary>Campo protobuf de tamanho+bytes (tipo 2).</summary>
    private static byte[] Field(int number, byte[] value)
    {
        var bytes = new List<byte>();
        Varint(bytes, (ulong)((number << 3) | 2));
        Varint(bytes, (ulong)value.Length);
        bytes.AddRange(value);
        return [.. bytes];
    }

    private static void Varint(List<byte> bytes, ulong value)
    {
        while (value >= 0x80)
        {
            bytes.Add((byte)(value | 0x80));
            value >>= 7;
        }

        bytes.Add((byte)value);
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
