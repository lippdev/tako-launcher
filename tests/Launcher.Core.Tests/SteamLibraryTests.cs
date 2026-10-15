using Launcher.Core.Library;
using Launcher.Core.Library.Steam;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class SteamLibraryTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    [Fact]
    public void Vdf_reads_nested_nodes_escapes_comments_and_conditionals()
    {
        var root = Vdf.Parse("""
            // comentário
            "AppState"
            {
                "appid"     "570"
                "name"      "Dota \"2\""
                "UserConfig" { "language" "brazilian" }
                "platform"  "win" [$WIN32]
            }
            """);

        var state = root.Node("appstate")!;
        Assert.Equal("570", state.Value("APPID"));
        Assert.Equal("Dota \"2\"", state.Value("name"));
        Assert.Equal("brazilian", state.Path("UserConfig")!.Value("language"));
        Assert.Equal("win", state.Value("platform"));
    }

    [Fact]
    public void Scanner_finds_installed_games_in_every_library_with_usage()
    {
        string steam = Path.Combine(_root, "Steam");
        string extra = Path.Combine(_root, "SteamLibrary");
        Write(steam, "steamapps/libraryfolders.vdf", $$"""
            "libraryfolders"
            {
                "0" { "path" "{{Escape(steam)}}" }
                "1" { "path" "{{Escape(extra)}}" }
            }
            """);
        Manifest(steam, 570, "Dota 2", flags: 4, size: 40L << 30);
        Manifest(extra, 1030300, "Hollow Knight: Silksong", flags: 4, size: 8L << 30);
        Manifest(extra, 228980, "Steamworks Common Redistributables", flags: 4, size: 1);
        Manifest(extra, 999, "Baixando", flags: 1026, size: 1);
        Write(steam, "userdata/123/config/localconfig.vdf", """
            "UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" {
                "570" { "LastPlayed" "1760000000" "Playtime" "125" }
            } } } } }
            """);

        var apps = SteamLibraryScanner.Scan(steam);

        Assert.Equal(["Dota 2", "Hollow Knight: Silksong"], apps.Select(a => a.Name));
        var dota = apps[0];
        Assert.Equal(TimeSpan.FromMinutes(125), dota.PlayTime);
        Assert.Equal(DateTimeOffset.FromUnixTimeSeconds(1760000000), dota.LastPlayed);
        Assert.Equal(Path.Combine(steam, "steamapps", "common", "Dota 2"), dota.InstallPath);

        var game = SteamGames.ToGame(dota);
        Assert.Equal("steam:570", game.Id);
        Assert.Equal("steam://rungameid/570", game.LaunchTarget);
        Assert.Equal("40 GB", game.SizeLabel);
        Assert.Equal(new Uri("steam-art://570/cover"), game.CoverUri);
    }

    [Fact]
    public void Scanner_accepts_old_libraryfolders_format_and_missing_files()
    {
        string steam = Path.Combine(_root, "Steam");
        string extra = Path.Combine(_root, "Old");
        Write(steam, "steamapps/libraryfolders.vdf", $$"""
            "LibraryFolders" { "TimeNextStatsReport" "1" "1" "{{Escape(extra)}}" }
            """);

        Assert.Equal([steam, extra], SteamLibraryScanner.LibraryFolders(steam));
        Assert.Empty(SteamLibraryScanner.Scan(Path.Combine(_root, "nao-existe")));
    }

    [Theory]
    [InlineData(0L, null)]
    [InlineData(500L << 20, "500 MB")]
    [InlineData((long)(2.5 * (1L << 30)), "2,5 GB")]
    [InlineData(120L << 30, "120 GB")]
    public void Size_label(long bytes, string? expected) => Assert.Equal(expected, SteamGames.SizeLabel(bytes));

    [Fact]
    public void Local_library_persists_manual_games_and_user_state()
    {
        var time = new FixedTimeProvider(new DateTimeOffset(2026, 10, 15, 12, 0, 0, TimeSpan.Zero));
        string file = Path.Combine(_root, "library.json");
        string exe = Write(_root, "Jogo/jogo.exe", "MZ");
        var steamGame = SteamGames.ToGame(new SteamApp(570, "Dota 2", "C:/x", 1, time.GetUtcNow(), null, TimeSpan.Zero));

        var library = new LocalGameLibrary(() => [steamGame], new LibraryStore(file), time);
        library.Load();
        var added = library.AddManual(ManualGameRecord.Create("Meu Jogo", exe, time.GetUtcNow()));
        library.ToggleFavorite("steam:570");
        library.RecordLaunch(added.Id);

        // Nova instância (próxima abertura do launcher) lê o mesmo arquivo.
        var reopened = new LocalGameLibrary(() => [SteamGames.ToGame(new SteamApp(570, "Dota 2", "C:/x", 1, time.GetUtcNow(), null, TimeSpan.Zero))], new LibraryStore(file), time);
        reopened.Load();

        Assert.Equal(2, reopened.Games.Count);
        Assert.True(reopened.Find("steam:570")!.IsFavorite);
        var manual = reopened.Find(added.Id)!;
        Assert.Equal(GameStore.Manual, manual.Store);
        Assert.True(manual.IsInstalled);
        Assert.Equal(exe, manual.LaunchTarget);
        Assert.Equal(time.GetUtcNow(), manual.LastPlayedAt);

        reopened.RemoveManual(added.Id);
        Assert.Null(reopened.Find(added.Id));
        Assert.Null(reopened.FindManual(added.Id));
    }

    [Fact]
    public void Corrupted_library_file_starts_empty_and_keeps_a_copy()
    {
        string file = Write(_root, "library.json", "{ isto não é json");
        var library = new LocalGameLibrary(() => [], new LibraryStore(file), TimeProvider.System);

        library.Load();

        Assert.Empty(library.Games);
        Assert.True(File.Exists(file + ".corrompido"));
    }

    private static string Escape(string path) => path.Replace("\\", "\\\\", StringComparison.Ordinal);

    private static void Manifest(string library, int appId, string name, int flags, long size) =>
        Write(library, $"steamapps/appmanifest_{appId}.acf", $$"""
            "AppState"
            {
                "appid"       "{{appId}}"
                "name"        "{{name}}"
                "StateFlags"  "{{flags}}"
                "installdir"  "{{name}}"
                "SizeOnDisk"  "{{size}}"
            }
            """);

    private static string Write(string root, string relative, string content)
    {
        string path = Path.Combine(root, relative.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, content);
        return path;
    }
}
