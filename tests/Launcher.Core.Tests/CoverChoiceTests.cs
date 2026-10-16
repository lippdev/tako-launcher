using System.Text.Json;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Art;
using Launcher.Core.Library.Metadata.Igdb;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Library.Metadata.SteamGridDb;
using Launcher.Core.Persistence;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// ADR-0023: SteamGridDB (consultas, filtros, escolha automática, respostas escritas à mão), ids fixados por fonte
/// ("Fontes e IDs"), imagens fixadas por tipo e as imagens de cada fonte para o seletor. Nenhuma chamada de rede.
/// </summary>
public sealed class CoverChoiceTests : IDisposable
{
    private static readonly DateTimeOffset Now = new(2026, 10, 15, 12, 0, 0, TimeSpan.Zero);
    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-covers-" + Guid.NewGuid().ToString("N"));

    public CoverChoiceTests() => Directory.CreateDirectory(_root);

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    // ---------------------------------------------------------------- SteamGridDB

    private const string Grids = """
        {"success":true,"data":[
          {"id":1,"score":3,"style":"alternate","width":600,"height":900,"nsfw":false,"humor":false,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/a.png","thumb":"https://cdn2.steamgriddb.com/thumb/a.jpg","author":{"name":"Ana"}},
          {"id":2,"score":9,"style":"blurred","width":342,"height":482,"nsfw":false,"humor":false,"mime":"image/jpeg","url":"https://cdn2.steamgriddb.com/grid/b.jpg","thumb":"https://cdn2.steamgriddb.com/thumb/b.jpg","author":{"name":"Bia"}},
          {"id":3,"score":20,"style":"alternate","width":600,"height":900,"nsfw":true,"humor":false,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/c.png","author":{"name":"Caio"}},
          {"id":4,"score":15,"style":"material","width":920,"height":430,"nsfw":false,"humor":false,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/d.png","author":{"name":"Davi"}},
          {"id":5,"score":30,"style":"alternate","width":600,"height":900,"nsfw":false,"humor":false,"mime":"image/webp","url":"https://cdn2.steamgriddb.com/grid/e.webp","thumb":"https://cdn2.steamgriddb.com/thumb/e.webp","author":{"name":"Eva"}},
          {"id":6,"score":50,"style":"alternate","width":600,"height":900,"nsfw":false,"humor":true,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/f.png","author":{"name":"Fred"}},
          {"id":7,"score":1,"url":"http://inseguro.example/x.png"}
        ]}
        """;

    [Fact]
    public void Image_paths_carry_the_filter_for_each_kind()
    {
        Assert.Equal("grids/game/42?dimensions=600x900,342x482,660x930&types=static&nsfw=false&humor=false", SteamGridDbQueries.Images("42", new SteamGridDbFilter(ArtKind.Cover)));
        Assert.Equal("grids/game/42?dimensions=920x430,460x215&styles=material&types=static,animated&nsfw=false&humor=false",
            SteamGridDbQueries.Images("42", new SteamGridDbFilter(ArtKind.Cover) { Horizontal = true, Style = "material", Animated = true }));
        Assert.Equal("heroes/game/42?types=static&nsfw=false&humor=false", SteamGridDbQueries.Images("42", new SteamGridDbFilter(ArtKind.Hero) { Style = "no_logo" })); // estilo que heroes não tem sai
        Assert.StartsWith("logos/game/42?styles=white&", SteamGridDbQueries.Images("42", new SteamGridDbFilter(ArtKind.Logo) { Style = "white" }), StringComparison.Ordinal);
        Assert.StartsWith("icons/game/42?", SteamGridDbQueries.Images("42", new SteamGridDbFilter(ArtKind.Icon)), StringComparison.Ordinal);
        Assert.Equal("games/steam/570", SteamGridDbQueries.BySteamAppId(570));
        Assert.Equal("search/autocomplete/Lanterna%20Distante", SteamGridDbQueries.Autocomplete(" Lanterna Distante "));
    }

    [Fact]
    public void Parser_reads_author_size_style_and_animation_and_drops_insecure_urls()
    {
        var images = SteamGridDbParser.Images(SteamGridDbClient.Data(Grids)!.Value, ArtKind.Cover);

        Assert.Equal(6, images.Count);
        var first = images[0];
        Assert.Equal("Ana", first.Author);
        Assert.Equal(600, first.Width);
        Assert.Equal("alternate", first.Style);
        Assert.Equal(new Uri("https://cdn2.steamgriddb.com/thumb/a.jpg"), first.Preview);
        Assert.Equal("Alternativo · 600 × 900 · por Ana", first.Caption);
        Assert.True(images.Single(i => i.Url.AbsolutePath.EndsWith("e.webp", StringComparison.Ordinal)).IsAnimated);
        Assert.Null(SteamGridDbClient.Data("""{"success":false,"errors":["Unauthorized"]}"""));
    }

    [Fact]
    public void Filter_keeps_vertical_static_safe_images_by_score_and_auto_choice_prefers_600x900()
    {
        var images = SteamGridDbParser.Images(SteamGridDbClient.Data(Grids)!.Value, ArtKind.Cover);

        var vertical = SteamGridDbQueries.Apply(images, new SteamGridDbFilter(ArtKind.Cover));
        Assert.Equal(["b.jpg", "a.png"], vertical.Select(i => Path.GetFileName(i.Url.AbsolutePath))); // sem NSFW, humor, animada nem horizontal
        Assert.Equal(["e.webp", "b.jpg", "a.png"], SteamGridDbQueries.Apply(images, new SteamGridDbFilter(ArtKind.Cover) { Animated = true }).Select(i => Path.GetFileName(i.Url.AbsolutePath)));
        Assert.Equal(["d.png"], SteamGridDbQueries.Apply(images, new SteamGridDbFilter(ArtKind.Cover) { Horizontal = true }).Select(i => Path.GetFileName(i.Url.AbsolutePath)));
        Assert.Equal(["a.png"], SteamGridDbQueries.Apply(images, new SteamGridDbFilter(ArtKind.Cover) { Style = "alternate" }).Select(i => Path.GetFileName(i.Url.AbsolutePath)));

        Assert.Equal("a.png", Path.GetFileName(SteamGridDbQueries.BestAutomatic(images, ArtKind.Cover)!.Url.AbsolutePath)); // 600×900 vence a mais votada 342×482
        Assert.Null(SteamGridDbQueries.BestAutomatic([], ArtKind.Hero));
    }

    [Fact]
    public void Games_come_from_autocomplete_and_steam_lookup()
    {
        var list = SteamGridDbParser.Games(SteamGridDbClient.Data("""{"success":true,"data":[{"id":5249,"name":"Lanterna Distante","release_date":1717200000,"types":["steam","egs"],"verified":true},{"id":7,"name":"Lanterna 2"}]}""")!.Value);
        var single = SteamGridDbParser.Games(SteamGridDbClient.Data("""{"success":true,"data":{"id":5249,"name":"Lanterna Distante"}}""")!.Value);

        Assert.Equal(["5249", "7"], list.Select(m => m.Id));
        Assert.Equal(2024, list[0].Year);
        Assert.Equal("Steam, Epic", list[0].Platforms);
        Assert.Equal("SteamGridDB", list[0].Source);
        Assert.Equal("Lanterna Distante", Assert.Single(single).Title);
    }

    [Fact]
    public void Settings_validate_the_key_and_never_print_it()
    {
        Assert.Contains("Cole", new SteamGridDbSettings(true, string.Empty).Problem(), StringComparison.Ordinal);
        Assert.Contains("incompleta", new SteamGridDbSettings(true, "abc").Problem(), StringComparison.Ordinal);
        var ok = new SteamGridDbSettings(true, "0123456789abcdef0123456789abcdef");
        Assert.Null(ok.Problem());
        Assert.True(ok.IsActive);
        Assert.False(ok.AutoFill); // desligado por padrão
        Assert.DoesNotContain("0123456789abcdef", ok.ToString(), StringComparison.Ordinal);
        Assert.Equal("abc", SteamGridDbSettings.Clean(" a b\nc "));
    }

    [Fact]
    public void Settings_store_round_trips_through_the_protector()
    {
        var store = new SteamGridDbSettingsStore(Path.Combine(_root, "sgdb.dat"), new ReverseProtector());
        store.Save(new SteamGridDbSettings(true, "0123456789abcdef0123456789abcdef") { AutoFill = true, Animated = true });

        var reopened = new SteamGridDbSettingsStore(Path.Combine(_root, "sgdb.dat"), new ReverseProtector()).Current;
        Assert.True(reopened.AutoFill);
        Assert.True(reopened.Animated);
        Assert.DoesNotContain("0123456789abcdef", File.ReadAllText(Path.Combine(_root, "sgdb.dat")), StringComparison.Ordinal);
        Assert.Same(SteamGridDbSettings.Off, new SteamGridDbSettingsStore(Path.Combine(_root, "nada.dat"), new ReverseProtector()).Current);
    }

    // ---------------------------------------------------------------- Ids colados

    [Theory]
    [InlineData("Steam", "1145360", "1145360", null)]
    [InlineData("Steam", "https://store.steampowered.com/app/1145360/Hades/", "1145360", null)]
    [InlineData("Steam", "steamdb.info/app/570/", "570", null)]
    [InlineData("IGDB", "113112", "113112", null)]
    [InlineData("IGDB", "https://www.igdb.com/games/Hades", null, "hades")]
    [InlineData("ScreenScraper", "https://www.screenscraper.fr/gameinfos.php?plateforme=4&gameid=3", "3", null)]
    [InlineData("SteamGridDB", "https://www.steamgriddb.com/game/5249/grids", "5249", null)]
    [InlineData("SteamGridDB", " 005249 ", "5249", null)]
    public void Pasted_ids_and_page_addresses_become_ids(string source, string text, string? id, string? slug)
    {
        var parsed = SourceIdInput.Parse(source, text);

        Assert.True(parsed.IsValid, parsed.Problem);
        Assert.Equal(id, parsed.Id);
        Assert.Equal(slug, parsed.Slug);
    }

    [Theory]
    [InlineData("Steam", "")]
    [InlineData("Steam", "igdb.com/games/hades")]
    [InlineData("SteamGridDB", "store.steampowered.com/app/570")]
    [InlineData("IGDB", "0")]
    public void Unknown_text_explains_with_an_example(string source, string text)
    {
        var parsed = SourceIdInput.Parse(source, text);

        Assert.False(parsed.IsValid);
        Assert.NotNull(parsed.Problem);
    }

    // ---------------------------------------------------------------- Ids fixados por fonte e escolha automática

    [Fact]
    public async Task Pinned_id_wins_over_the_store_and_is_not_replaced_by_a_title_search()
    {
        var steam = new FakeProvider("Steam", id => Part("Steam", id), exactKey: "Steam") { SearchResults = [MetadataMatch.Steam("Errado", 1)] };
        var igdb = new FakeProvider("IGDB", id => Part("IGDB", id)) { SearchResults = [new MetadataMatch("IGDB", "111", "Outro")] };
        var time = new FixedTimeProvider(Now);
        var agent = new MetadataAgent([steam, igdb], time);
        var game = Game(GameStore.Steam, steamAppId: 4000001);
        var pinned = new Dictionary<string, string> { ["IGDB"] = "900001" };

        var first = (await agent.UpdateAsync(game, null, false, TestContext.Current.CancellationToken, pinned)).Metadata!;
        Assert.Equal("900001", first.IdIn("IGDB"));
        Assert.Equal(0, igdb.Searches);
        Assert.False(agent.NeedsUpdate(game, first, pinned: pinned));

        // Trocar o id fixado pede uma rodada nova; o da loja (Steam) continua.
        var changed = new Dictionary<string, string> { ["IGDB"] = "900002" };
        Assert.True(agent.NeedsUpdate(game, first, pinned: changed));
        var second = (await agent.UpdateAsync(game, first, false, TestContext.Current.CancellationToken, changed)).Metadata!;
        Assert.Equal("900002", second.IdIn("IGDB"));
        Assert.Equal("4000001", second.IdIn("Steam"));

        // "Nenhum": a fonte fica de fora do jogo.
        var none = new Dictionary<string, string> { ["IGDB"] = string.Empty };
        Assert.True(agent.NeedsUpdate(game, second, pinned: none));
        var third = (await agent.UpdateAsync(game, second, false, TestContext.Current.CancellationToken, none)).Metadata!;
        Assert.Null(third.IdIn("IGDB"));
        Assert.DoesNotContain(third.Parts, p => p.Source == "IGDB");
    }

    [Fact]
    public async Task Steamgriddb_only_fills_games_left_without_cover_and_only_when_turned_on()
    {
        var igdb = new FakeProvider("IGDB", id => Part("IGDB", id) with { CoverUrl = null }, exactKey: "GOG");
        var sgdb = new FakeArtProvider(new Uri("https://cdn2.steamgriddb.com/grid/a.png")) { AutoFill = false };
        var agent = new MetadataAgent([igdb, sgdb], new FixedTimeProvider(Now));
        var game = new Game { Id = "gog:1", Title = "Lanterna", Store = GameStore.Gog, Accent = "#000000" };

        var off = (await agent.UpdateAsync(game, null, false, TestContext.Current.CancellationToken)).Metadata!;
        Assert.Null(off.CoverUrl);
        Assert.Equal(0, sgdb.Fetches);

        sgdb.AutoFill = true;
        Assert.True(agent.NeedsUpdate(game, off));
        var on = (await agent.UpdateAsync(game, off, false, TestContext.Current.CancellationToken)).Metadata!;
        Assert.Equal(new Uri("https://cdn2.steamgriddb.com/grid/a.png"), on.CoverUrl);

        // Jogo da Steam já tem capa: o SteamGridDB nem é consultado.
        var steamGame = Game(GameStore.Steam, steamAppId: 4000001);
        var steamAgent = new MetadataAgent([new FakeProvider("Steam", id => Part("Steam", id), exactKey: "Steam"), sgdb], new FixedTimeProvider(Now));
        int before = sgdb.Fetches;
        var steamResult = (await steamAgent.UpdateAsync(steamGame, null, false, TestContext.Current.CancellationToken)).Metadata!;
        Assert.Equal(before, sgdb.Fetches);
        Assert.Contains("SteamGridDB", steamResult.Consulted);
        Assert.Equal(new Uri("steam-art://4000001/cover"), steamResult.CoverUrl);

        // Fora da busca de todas as fontes, mas na lista de "Fontes e IDs".
        Assert.DoesNotContain(await agent.SearchAsync(game, TestContext.Current.CancellationToken), m => m.Source == "SteamGridDB");
        Assert.Contains(agent.Sources(game), p => p.Name == "SteamGridDB");
    }

    [Fact]
    public async Task Validation_resolves_slugs_and_returns_the_name()
    {
        var igdb = new SlugProvider(id => id == "113112" ? Part("IGDB", id) with { Title = "Hades" } : null);
        var agent = new MetadataAgent([igdb], new FixedTimeProvider(Now));

        var bySlug = await agent.ValidateAsync("IGDB", SourceIdInput.Parse("IGDB", "igdb.com/games/hades"), TestContext.Current.CancellationToken);
        var unknown = await agent.ValidateAsync("IGDB", SourceIdInput.Parse("IGDB", "999"), TestContext.Current.CancellationToken);

        Assert.Equal("113112", bySlug!.Value.Id);
        Assert.Equal("Hades", bySlug.Value.Metadata.Title);
        Assert.Null(unknown);
    }

    // ---------------------------------------------------------------- Imagens fixadas por tipo

    [Fact]
    public void Pinned_art_is_shown_per_kind_and_survives_metadata_updates()
    {
        var game = Game(GameStore.Epic);
        new GameMetadata { Source = "IGDB", SourceIds = new Dictionary<string, string> { ["IGDB"] = "1" }, CoverUrl = new Uri("https://images.igdb.com/a.jpg") }.ApplyTo(game);
        var chosen = new Uri("https://cdn2.steamgriddb.com/grid/a.png");

        game.PinnedArt = ArtPins.With(game.PinnedArt, ArtKind.Cover, chosen);
        new GameMetadata { Source = "IGDB", SourceIds = new Dictionary<string, string> { ["IGDB"] = "1" }, CoverUrl = new Uri("https://images.igdb.com/b.jpg") }.ApplyTo(game);

        Assert.Equal(chosen, game.ShownCover);
        Assert.Equal(new Uri("https://images.igdb.com/b.jpg"), ArtPins.Automatic(game, ArtKind.Cover)); // a automática segue atualizando por baixo
        Assert.Equal(game.HeroUri, game.ShownHero);

        game.PinnedArt = ArtPins.With(game.PinnedArt, ArtKind.Cover, null); // "Voltar para a automática"
        Assert.Equal(new Uri("https://images.igdb.com/b.jpg"), game.ShownCover);
    }

    [Fact]
    public void Choices_are_saved_in_the_database()
    {
        var store = new GameChoicesStore(new TakoDatabase(Path.Combine(_root, "tako.db")));
        store.SetArt("epic:1", ArtKind.Cover, new Uri("https://cdn2.steamgriddb.com/grid/a.png"));
        store.SetArt("epic:1", ArtKind.Logo, new Uri("https://cdn2.steamgriddb.com/logo/l.png"));
        store.SetArt("epic:1", ArtKind.Logo, null);
        store.SetId("epic:1", "IGDB", "900001");
        store.SetId("epic:1", "SteamGridDB", string.Empty);
        store.SetId("epic:1", "Steam", "1");
        store.SetId("epic:1", "Steam", null);

        var reopened = new GameChoicesStore(new TakoDatabase(Path.Combine(_root, "tako.db")));
        var art = reopened.LoadAllArt()["epic:1"];
        Assert.Equal([ArtKind.Cover], art.Keys);
        Assert.Equal(new Dictionary<string, string> { ["IGDB"] = "900001", ["SteamGridDB"] = string.Empty }, reopened.LoadIds("epic:1"));
    }

    // ---------------------------------------------------------------- Imagens de cada fonte

    [Fact]
    public void Each_source_lists_its_images_for_the_picker()
    {
        var steam = SteamArtCandidates.For(570);
        Assert.Equal([ArtKind.Cover, ArtKind.Hero, ArtKind.Logo, ArtKind.Icon], steam.Select(c => c.Kind));
        Assert.Equal(new Uri("steam-art://570/logo"), steam[2].Url);

        using var igdb = JsonDocument.Parse("""
            {"id":1,"cover":{"image_id":"capa"},"artworks":[{"image_id":"arte"}],"screenshots":[{"image_id":"tela"}],
             "game_localizations":[{"region":{"identifier":"br"},"cover":{"image_id":"capabr"}}]}
            """);
        var images = IgdbParser.Images(igdb.RootElement);
        Assert.Equal(["Capa", "Capa (Brasil)", "Arte oficial", "Captura"], images.Select(i => i.Label));
        Assert.Contains("t_cover_big_2x/capa.jpg", images[0].Url.AbsoluteUri, StringComparison.Ordinal);
        Assert.Contains("t_cover_big/capa.jpg", images[0].Preview.AbsoluteUri, StringComparison.Ordinal);
        Assert.Equal(ArtKind.Hero, images[2].Kind);

        using var jeu = JsonDocument.Parse("""
            {"medias":[
              {"type":"box-2D","region":"jp","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?devid=x&devpassword=y&ssid=u&sspassword=p&media=box-2D(jp)"},
              {"type":"box-2D","region":"us","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?devid=x&media=box-2D(us)"},
              {"type":"wheel-hd","region":"wor","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?media=wheel-hd(wor)"},
              {"type":"fanart","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?media=fanart"},
              {"type":"video","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?media=video"}
            ]}
            """);
        var media = ScreenScraperParser.MediaCandidates(jeu.RootElement);
        Assert.Equal(["Caixa 2D · EUA", "Caixa 2D · Japão", "Fanart", "Logo (HD) · Mundo"], media.Select(m => m.Label));
        Assert.All(media, m => Assert.DoesNotContain("password", m.Url.AbsoluteUri, StringComparison.Ordinal));
    }

    [Fact]
    public void Local_images_are_the_newest_files_of_the_common_folders()
    {
        string pictures = Directory.CreateDirectory(Path.Combine(_root, "Imagens")).FullName;
        string sub = Directory.CreateDirectory(Path.Combine(pictures, "Capas")).FullName;
        File.WriteAllBytes(Path.Combine(pictures, "velha.png"), [1]);
        File.SetLastWriteTimeUtc(Path.Combine(pictures, "velha.png"), Now.UtcDateTime.AddDays(-2));
        File.WriteAllBytes(Path.Combine(sub, "nova.jpg"), [1]);
        File.SetLastWriteTimeUtc(Path.Combine(sub, "nova.jpg"), Now.UtcDateTime);
        File.WriteAllBytes(Path.Combine(pictures, "notas.txt"), [1]);

        var found = LocalImages.Find([pictures, Path.Combine(_root, "nao-existe")]);

        Assert.Equal(["nova.jpg", "velha.png"], found.Select(Path.GetFileName));
    }

    [Fact]
    public void Sizes_are_written_for_people()
    {
        Assert.Equal("vazio", ByteSize.Format(0));
        Assert.Equal("512 B", ByteSize.Format(512));
        Assert.Equal("12 KB", ByteSize.Format(12 * 1024));
        Assert.Equal("12,4 MB", ByteSize.Format((long)(12.4 * 1024 * 1024)));
        Assert.Equal("1,5 GB", ByteSize.Format(3L * 512 * 1024 * 1024));
    }

    // ---------------------------------------------------------------- Apoio

    private static GameMetadata Part(string source, string id) => new()
    {
        Source = source,
        SourceIds = new Dictionary<string, string> { [source] = id },
        SteamAppId = source == "Steam" ? int.Parse(id, System.Globalization.CultureInfo.InvariantCulture) : null,
        Title = "Lanterna Distante",
        CoverUrl = source == "IGDB" ? new Uri("https://images.igdb.com/c.jpg") : null,
    };

    private static Game Game(GameStore store, int? steamAppId = null) => new()
    {
        Id = $"{store.ToString().ToLowerInvariant()}:1",
        Title = "Lanterna Distante",
        Store = store,
        Accent = "#000000",
        SteamAppId = steamAppId,
    };

    private sealed class ReverseProtector : ConsoleMode.Runtime.Tv.ISecretProtector
    {
        public string Protect(string secret) => new(secret.Reverse().ToArray());

        public bool TryUnprotect(string stored, out string secret)
        {
            secret = new string(stored.Reverse().ToArray());
            return true;
        }
    }

    private class FakeProvider(string name, Func<string, GameMetadata?> fetch, string? exactKey = null) : IMetadataProvider
    {
        public string Name => name;

        public bool IsEnabled => true;

        public IReadOnlyList<MetadataMatch> SearchResults { get; init; } = [];

        public int Searches { get; private set; }

        public int Fetches { get; private set; }

        public Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken)
        {
            Searches++;
            return Task.FromResult(SearchResults);
        }

        public virtual Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken) =>
            Task.FromResult(exactKey is null ? null : lookup.ExternalIds.GetValueOrDefault(exactKey) ?? (exactKey == "GOG" ? "gog-1" : null));

        public Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken)
        {
            Fetches++;
            return Task.FromResult(fetch(id));
        }
    }

    private sealed class SlugProvider(Func<string, GameMetadata?> fetch) : FakeProvider("IGDB", fetch), ISlugResolver
    {
        public Task<string?> IdFromSlugAsync(string slug, CancellationToken cancellationToken) => Task.FromResult<string?>(slug == "hades" ? "113112" : null);
    }

    private sealed class FakeArtProvider(Uri cover) : FakeProvider("SteamGridDB", id => new GameMetadata
    {
        Source = "SteamGridDB",
        SourceIds = new Dictionary<string, string> { ["SteamGridDB"] = id },
        CoverUrl = cover,
    }), IArtFallbackProvider
    {
        public bool AutoFill { get; set; }

        public override Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken) => Task.FromResult<string?>("5249");
    }
}
