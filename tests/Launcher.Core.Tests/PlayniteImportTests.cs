using Launcher.Core.Library;
using Launcher.Core.Library.Playnite;
using Launcher.Tests;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Importação da biblioteca do Playnite (STORE-009): leitor do LiteDB 4, prévia, casamento com as lojas e gravação.</summary>
public sealed class PlayniteImportTests
{
    private static readonly DateTimeOffset Now = new(2026, 10, 15, 12, 0, 0, TimeSpan.Zero);

    // ------------------------------------------------------------------ Leitor do LiteDB 4 (formato v7)

    [Fact]
    public void LiteDb_v7_reader_returns_every_document_including_extended_ones()
    {
        var reader = new LiteDbV7Reader(LiteDbV7Writer.Write(new Dictionary<string, IReadOnlyList<Dictionary<string, object?>>> { ["Game"] = PlayniteSample.Games() }));

        Assert.Equal(["Game"], reader.Collections);
        var documents = reader.Documents("Game");
        Assert.Equal(8, documents.Count);
        var longo = documents.Single(d => (string?)d["Name"] == "Descrição Longa");
        Assert.Equal(9000, ((string)longo["Description"]!).Length); // documento em páginas de extensão
        var lanterna = documents.Single(d => (Guid)d["_id"]! == PlayniteSample.Lanterna);
        Assert.Equal(7200L, lanterna["Playtime"]);
        Assert.Equal(new DateTime(2025, 3, 1, 12, 0, 0, DateTimeKind.Utc), lanterna["Added"]);
        Assert.Empty(reader.Documents("Inexistente"));
    }

    [Fact]
    public void LiteDb_reader_rejects_other_formats()
    {
        Assert.Throws<InvalidDataException>(() => new LiteDbV7Reader(new byte[100]));
        var v5 = LiteDbV7Writer.Write(new Dictionary<string, IReadOnlyList<Dictionary<string, object?>>> { ["Game"] = [] });
        v5[25 + 27] = 8; // versão do arquivo do LiteDB 5
        Assert.Throws<InvalidDataException>(() => new LiteDbV7Reader(v5));
    }

    [Fact]
    public void Library_folder_is_read_with_platforms_categories_and_actions()
    {
        string folder = Path.Combine(Path.GetTempPath(), $"tako-playnite-{Guid.NewGuid():N}");
        try
        {
            PlayniteSample.Write(folder);
            long before = new FileInfo(Path.Combine(folder, "games.db")).Length;
            var stamp = File.GetLastWriteTimeUtc(Path.Combine(folder, "games.db"));

            var library = PlayniteLibrary.Read(folder);

            Assert.Equal(8, library.Games.Count);
            var lanterna = library.Games.Single(g => g.Id == PlayniteSample.Lanterna);
            Assert.True(lanterna.Favorite);
            Assert.Equal(TimeSpan.FromHours(2), lanterna.Playtime);
            Assert.Equal(PlayniteActionType.File, lanterna.Actions[0].Type);
            Assert.Equal("PC (Windows)", library.PlatformOf(lanterna)!.Name);
            Assert.Equal("Jogar no sofá", library.Categories[PlayniteSample.Sofa]);
            Assert.Equal(PlayniteActionType.Url, library.Games.Single(g => g.Id == PlayniteSample.Navegador).Actions[0].Type);
            Assert.Equal("Steam", library.Sources.Values.Single());
            // Só leitura: o arquivo do Playnite não muda.
            Assert.Equal(before, new FileInfo(Path.Combine(folder, "games.db")).Length);
            Assert.Equal(stamp, File.GetLastWriteTimeUtc(Path.Combine(folder, "games.db")));
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }

    [Fact]
    public void Missing_library_is_a_file_not_found() =>
        Assert.Throws<FileNotFoundException>(() => PlayniteLibrary.Read(Path.Combine(Path.GetTempPath(), $"tako-sem-playnite-{Guid.NewGuid():N}")));

    [Fact]
    public void Locator_prefers_the_database_path_from_config()
    {
        var candidates = PlayniteLocator.Candidates("/appdata", "{\"DatabasePath\":\"%TAKO%\\\\biblioteca\"}", path => path.Replace("%TAKO%", "E:", StringComparison.Ordinal));

        Assert.Equal(["E:\\biblioteca", Path.Combine("/appdata", "Playnite", "library")], candidates);
        Assert.Single(PlayniteLocator.Candidates("/appdata", "{ inválido"));
        Assert.Single(PlayniteLocator.Candidates("/appdata", "{\"DatabasePath\":\"{PlayniteDir}\\\\library\"}"));
    }

    // ------------------------------------------------------------------ Prévia

    [Fact]
    public void Plan_imports_new_games_matches_store_games_and_skips_the_rest()
    {
        var library = Sample();
        var steam = Game("steam:7", "Farol", GameStore.Steam);

        var plan = PlayniteImport.Plan(library, [steam], ["Zerados"], Now);

        Assert.Equal(["Lanterna Distante", "Cartucho Azul", "Jogo no Navegador", "Descrição Longa"], plan.New.Select(g => g.Record.Title));
        var matched = Assert.Single(plan.Matched);
        Assert.Equal("steam:7", matched.TakoId);
        Assert.Equal(["Jogar no sofá"], matched.Collections);
        Assert.Equal(
            [(PlayniteSample.Corrida, PlayniteSkipReason.StoreGame), (PlayniteSample.Roteiro, PlayniteSkipReason.PlayniteOnly), (PlayniteSample.Ausente, PlayniteSkipReason.NotPlayable)],
            plan.Skipped.Select(s => (s.Source.Id, s.Reason)));
        Assert.Equal(["Jogar no sofá"], plan.CollectionNames); // "Zerados" já existe no Tako
        Assert.Equal(
            "4 jogos novos, 1 jogo que o Tako já tem (favoritos e tempo completados) e 1 coleção. Ficam de fora 3 jogos (lojas que o Tako já lê, não instalados ou só pelo Playnite)",
            plan.Summary);
    }

    [Fact]
    public void New_game_records_resolve_install_dir_and_keep_rom_platform()
    {
        var plan = PlayniteImport.Plan(Sample(), [], [], Now);

        var lanterna = plan.New.Single(g => g.Source.Id == PlayniteSample.Lanterna);
        Assert.Equal("manual:playnite-11111111111111111111111111111111", lanterna.Record.Id);
        Assert.Equal(@"C:\Jogos\Lanterna\lanterna.exe", lanterna.Record.Path);
        Assert.Equal("-tela-cheia", lanterna.Record.Arguments);
        Assert.Equal(@"C:\Jogos\Lanterna", lanterna.Record.WorkingDirectory);
        Assert.Equal(new DateTimeOffset(2025, 3, 1, 12, 0, 0, TimeSpan.Zero), lanterna.Record.AddedAt);
        Assert.Equal("PC (Windows)", lanterna.PlatformName);
        Assert.Equal(["Jogar no sofá", "Zerados"], lanterna.Collections);

        var rom = plan.New.Single(g => g.Source.Id == PlayniteSample.Cartucho).Record;
        Assert.Equal(@"D:\ROMs\SNES\azul.sfc", rom.Path);
        Assert.Equal("snes", rom.Platform);

        var url = plan.New.Single(g => g.Source.Id == PlayniteSample.Navegador).Record;
        Assert.Equal("https://example.com/jogo", url.Path);
        Assert.Null(url.WorkingDirectory);
    }

    [Fact]
    public void Matching_never_duplicates_store_games_or_a_previous_import()
    {
        var library = Sample();
        var corrida = library.Games.Single(g => g.Id == PlayniteSample.Corrida);
        var lanterna = library.Games.Single(g => g.Id == PlayniteSample.Lanterna);

        Assert.Equal("epic:CorridaNoturna", PlayniteImport.Match(corrida, [Game("epic:CorridaNoturna", "Corrida", GameStore.Epic)])!.Id);
        Assert.Equal("gog:1", PlayniteImport.Match(lanterna, [Game("gog:1", "Outro nome", GameStore.Gog, installPath: @"C:\Jogos\Lanterna\")])!.Id); // mesma pasta
        Assert.Equal("manual:playnite-11111111111111111111111111111111", PlayniteImport.Match(lanterna, [Game("manual:playnite-11111111111111111111111111111111", "Lanterna", GameStore.Manual)])!.Id);
        Assert.Equal("manual:abc", PlayniteImport.Match(lanterna, [Game("manual:abc", "Lanterna Distante™", GameStore.Manual)])!.Id); // adicionado à mão nos dois
        Assert.Null(PlayniteImport.Match(corrida, [Game("epic:Outro", "Corrida Noturna", GameStore.Epic)])); // loja casa por id ou pasta, não por nome

        var battle = new PlayniteGame { Id = Guid.NewGuid(), Name = "Overwatch 2", PluginId = new Guid("e3c26a3d-d695-4cb7-a769-5ff7612c7edd"), GameId = "Pro" };
        var bnet = Game("battlenet:prometheus", "Overwatch 2", GameStore.BattleNet);
        Assert.Equal("battlenet:prometheus", PlayniteImport.Match(battle, [bnet])!.Id);
    }

    [Fact]
    public void Unknown_playnite_variables_are_not_imported()
    {
        var source = new PlayniteGame
        {
            Id = Guid.NewGuid(),
            Name = "Variável",
            IsInstalled = true,
            InstallDirectory = @"C:\Jogo",
            Actions = [new PlayniteAction(PlayniteActionType.File, "{PlayniteDir}\\x.exe", null, null, true)],
        };

        Assert.Equal((null, PlayniteSkipReason.PlayniteOnly), PlayniteImport.ToRecord(source, null, Now));
        Assert.Equal(@"C:\Jogo\bin\jogo.exe", PlayniteImport.ToRecord(source with { Actions = [new PlayniteAction(PlayniteActionType.File, @"bin\jogo.exe", null, null, true)] }, null, Now).Record!.Path);
    }

    [Fact]
    public void Merge_keeps_favorite_and_the_larger_time_but_not_for_steam()
    {
        var source = new PlayniteGame { Id = Guid.NewGuid(), Name = "X", Favorite = true, Playtime = TimeSpan.FromMinutes(90), LastActivity = Now };
        var current = new GameStateRecord { PlayMinutes = 30, LastPlayedAt = Now.AddDays(-3) };

        Assert.Equal(new GameStateRecord { IsFavorite = true, PlayMinutes = 90, LastPlayedAt = Now }, PlayniteImport.MergeState(current, source, GameStore.Gog));
        Assert.Equal(30, PlayniteImport.MergeState(current, source, GameStore.Steam).PlayMinutes); // a Steam soma o próprio tempo
        Assert.Equal(120, PlayniteImport.MergeState(current with { PlayMinutes = 120 }, source, GameStore.Gog).PlayMinutes);
    }

    [Fact]
    public void Empty_plan_explains_why()
    {
        var empty = PlayniteImport.Plan(PlayniteLibrary.FromDocuments([], [], [], []), [], [], Now);
        Assert.True(empty.IsEmpty);
        Assert.Equal("A biblioteca do Playnite está vazia", empty.Summary);
    }

    // ------------------------------------------------------------------ Gravação

    [Fact]
    public void Library_import_adds_games_and_state_in_one_save_and_is_idempotent()
    {
        string file = Path.Combine(Path.GetTempPath(), $"tako-import-{Guid.NewGuid():N}.json");
        try
        {
            var library = new LocalGameLibrary(() => [Game("steam:7", "Farol", GameStore.Steam)], new LibraryStore(file), TimeProvider.System);
            library.Load();
            var plan = PlayniteImport.Plan(Sample(), library.Games, [], Now);

            Apply(library, plan);
            Apply(library, PlayniteImport.Plan(Sample(), library.Games, [], Now)); // de novo: nada duplica

            Assert.Equal(5, library.Games.Count);
            var lanterna = library.Find("manual:playnite-11111111111111111111111111111111")!;
            Assert.True(lanterna.IsFavorite);
            Assert.Equal(TimeSpan.FromHours(2), lanterna.PlayTime);
            Assert.Equal("snes", library.Find("manual:playnite-66666666666666666666666666666666")!.Platform);
            Assert.True(library.Find("steam:7")!.IsFavorite);

            var reloaded = new LocalGameLibrary(() => [], new LibraryStore(file), TimeProvider.System);
            reloaded.Load();
            Assert.Equal(4, reloaded.Games.Count); // os importados ficam gravados
        }
        finally
        {
            File.Delete(file);
        }
    }

    private static void Apply(LocalGameLibrary library, PlayniteImportPlan plan)
    {
        var state = new Dictionary<string, Func<GameStateRecord?, GameStateRecord>>();
        foreach (var added in plan.New)
        {
            state[added.Record.Id] = current => PlayniteImport.MergeState(current, added.Source, GameStore.Manual);
        }

        foreach (var match in plan.Matched)
        {
            var store = library.Find(match.TakoId)!.Store;
            state[match.TakoId] = current => PlayniteImport.MergeState(current, match.Source, store);
        }

        library.Import([.. plan.New.Select(g => g.Record)], state);
    }

    private static PlayniteLibrary Sample()
    {
        var reader = new LiteDbV7Reader(LiteDbV7Writer.Write(new Dictionary<string, IReadOnlyList<Dictionary<string, object?>>> { ["Game"] = PlayniteSample.Games() }));
        return PlayniteLibrary.FromDocuments(
            reader.Documents("Game"),
            [
                new() { ["_id"] = PlayniteSample.Pc, ["Name"] = "PC (Windows)", ["SpecificationId"] = "pc_windows" },
                new() { ["_id"] = PlayniteSample.Snes, ["Name"] = "Super Nintendo", ["SpecificationId"] = "nintendo_super_nes" },
            ],
            [
                new() { ["_id"] = PlayniteSample.Sofa, ["Name"] = "Jogar no sofá" },
                new() { ["_id"] = PlayniteSample.Zerados, ["Name"] = "Zerados" },
            ],
            []);
    }

    private static Game Game(string id, string title, GameStore store, string? installPath = null) => new()
    {
        Id = id,
        Title = title,
        Store = store,
        Accent = "#000000",
        IsInstalled = true,
        InstallPath = installPath,
        StoreProductId = store == GameStore.BattleNet ? "Pro" : null,
    };
}
