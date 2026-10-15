using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Combinação por campo entre N provedores e o agente que escolhe a fonte por tipo de jogo (ADR-0021).</summary>
public sealed class MetadataMergeTests
{
    private static readonly DateTimeOffset Now = new(2026, 10, 15, 12, 0, 0, TimeSpan.Zero);

    private static readonly GameMetadata SteamPart = new()
    {
        Source = "Steam",
        SteamAppId = 4000001,
        SourceIds = new Dictionary<string, string> { ["Steam"] = "4000001" },
        Title = "Lanterna Distante",
        Description = "Um faroleiro explora um arquipélago.",
        DescriptionLanguage = "pt-BR",
        Genres = ["Aventura"],
        Developer = "Maré Baixa (Steam)",
        MetacriticScore = 88,
    };

    private static readonly GameMetadata IgdbPart = new()
    {
        Source = "IGDB",
        SteamAppId = 4000001,
        SourceIds = new Dictionary<string, string> { ["IGDB"] = "900001", ["Steam"] = "4000001", ["GOG"] = "1400000001" },
        Title = "Lanterna Distante",
        Description = "A lighthouse keeper explores an archipelago.",
        DescriptionLanguage = "en",
        Genres = ["Aventura", "Quebra-cabeça"],
        Themes = ["Fantasia"],
        GameModes = ["Um jogador"],
        Franchise = "Lanterna",
        ReleaseDate = new DateOnly(2024, 6, 1),
        Developer = "Maré Baixa (IGDB)",
        Publisher = "Estúdio Farol",
        CriticScore = 86,
        CoverUrl = new Uri("https://images.igdb.com/igdb/image/upload/t_cover_big/c.jpg"),
        BackgroundUrl = new Uri("https://images.igdb.com/igdb/image/upload/t_1080p/a.jpg"),
    };

    [Fact]
    public void Steam_games_keep_steam_data_and_fill_gaps_with_igdb()
    {
        var merged = MetadataMerge.Combine([IgdbPart, SteamPart], MetadataKind.Steam, Now);

        Assert.Equal("Steam + IGDB", merged.Source);
        Assert.Equal("Um faroleiro explora um arquipélago.", merged.Description);
        Assert.Equal(["Aventura"], merged.Genres);
        Assert.Equal("Maré Baixa (Steam)", merged.Developer);
        Assert.Equal("Estúdio Farol", merged.Publisher); // a Steam não trouxe
        Assert.Equal(["Fantasia"], merged.Themes);
        Assert.Equal("Lanterna", merged.Franchise);
        Assert.Equal(88, merged.MetacriticScore);
        Assert.Equal(86, merged.CriticScore);
        Assert.Equal(new Uri("steam-art://4000001/cover"), merged.CoverUrl);
        Assert.Equal(new Uri("steam-art://4000001/hero"), merged.BackgroundUrl);
        Assert.Equal("900001", merged.IdIn("IGDB"));
        Assert.Equal(2, merged.Parts.Count);
    }

    [Fact]
    public void Other_pc_games_use_igdb_first_but_keep_portuguese_text_and_bigger_steam_art()
    {
        var merged = MetadataMerge.Combine([SteamPart, IgdbPart], MetadataKind.Pc, Now);

        Assert.Equal("IGDB + Steam", merged.Source);
        Assert.Equal(["Aventura", "Quebra-cabeça"], merged.Genres);
        Assert.Equal("Maré Baixa (IGDB)", merged.Developer);
        Assert.Equal("Um faroleiro explora um arquipélago.", merged.Description); // português vence
        Assert.Equal("pt-BR", merged.DescriptionLanguage);
        Assert.Equal(new Uri("steam-art://4000001/cover"), merged.CoverUrl); // 600×900 da Steam
    }

    [Fact]
    public void Without_steam_the_igdb_art_and_english_summary_are_used()
    {
        var merged = MetadataMerge.Combine([IgdbPart with { SteamAppId = null, SourceIds = new Dictionary<string, string> { ["IGDB"] = "900001" } }], MetadataKind.Pc, Now);

        Assert.Equal(IgdbPart.CoverUrl, merged.CoverUrl);
        Assert.Equal(IgdbPart.BackgroundUrl, merged.BackgroundUrl);
        Assert.Equal("en", merged.DescriptionLanguage);
        Assert.Null(merged.SteamAppId);
    }

    [Fact]
    public void Emulated_games_prefer_the_rom_provider_and_unknown_providers_go_last()
    {
        var rom = new GameMetadata { Source = MetadataPriority.ScreenScraper, SourceIds = new Dictionary<string, string> { ["ScreenScraper"] = "77" }, Genres = ["Plataforma"], CoverUrl = new Uri("https://example.invalid/box.png") };
        var other = new GameMetadata { Source = "Outro", SourceIds = new Dictionary<string, string> { ["Outro"] = "1" }, Genres = ["X"], Publisher = "Só no outro" };

        var merged = MetadataMerge.Combine([other, IgdbPart, rom], MetadataKind.Emulated, Now);

        Assert.Equal("ScreenScraper + IGDB + Outro", merged.Source);
        Assert.Equal(["Plataforma"], merged.Genres);
        Assert.Equal(rom.CoverUrl, merged.CoverUrl);
        Assert.Equal("Estúdio Farol", merged.Publisher);
        Assert.Equal(["Steam", "IGDB", "ScreenScraper"], MetadataPriority.Order(MetadataKind.Steam));
        Assert.Equal(["IGDB", "Steam", "ScreenScraper"], MetadataPriority.Order(MetadataKind.Pc));
    }

    [Fact]
    public void Kind_comes_from_store_and_platform()
    {
        Assert.Equal(MetadataKind.Steam, MetadataPriority.KindOf(Game(GameStore.Steam)));
        Assert.Equal(MetadataKind.Pc, MetadataPriority.KindOf(Game(GameStore.Epic)));
        Assert.Equal(MetadataKind.Emulated, MetadataPriority.KindOf(new Game { Id = "rom:1", Title = "R", Store = GameStore.Manual, Accent = "#000000", Platform = "snes" }));
    }

    [Fact]
    public void Store_ids_come_from_each_store()
    {
        Assert.Equal("570", ExternalGameIds.For(Game(GameStore.Steam, steamAppId: 570))["Steam"]);
        Assert.Empty(ExternalGameIds.For(Game(GameStore.Epic, steamAppId: 570))); // appid de correspondência pelo nome não vale como id
        Assert.Equal("1207658924", ExternalGameIds.For(new Game { Id = "gog:1207658924", Title = "G", Store = GameStore.Gog, Accent = "#000000" })["GOG"]);
        Assert.Equal("cat123", ExternalGameIds.For(Game(GameStore.Epic, product: "cat123"))["Epic"]);
        Assert.Equal("9NBLGGH", ExternalGameIds.For(Game(GameStore.Xbox, product: "9NBLGGH"))["Microsoft"]);
    }

    [Fact]
    public void Applying_igdb_art_fills_games_without_their_own_cover()
    {
        var game = Game(GameStore.Epic);
        var merged = MetadataMerge.Combine([IgdbPart with { SteamAppId = null, SourceIds = new Dictionary<string, string> { ["IGDB"] = "900001" } }], MetadataKind.Pc, Now);

        merged.ApplyTo(game);

        Assert.Equal(IgdbPart.CoverUrl, game.CoverUri);
        Assert.Equal(IgdbPart.BackgroundUrl, game.HeroUri);
        Assert.Equal(["Fantasia"], game.Themes);
        Assert.Equal(["Um jogador"], game.GameModes);
        Assert.Equal("Lanterna", game.Franchise);
        Assert.Equal(86, game.CriticScore);
    }

    // ---------------------------------------------------------------- Agente

    [Fact]
    public async Task Epic_game_matches_igdb_by_title_and_steam_by_the_id_igdb_reports()
    {
        var steam = new FakeProvider("Steam", SteamPart, exactKey: "Steam");
        var igdb = new FakeProvider("IGDB", IgdbPart) { SearchResults = [new MetadataMatch("IGDB", "900001", "Lanterna Distante")] };
        var agent = new MetadataAgent([steam, igdb], new FixedTimeProvider(Now));

        var update = await agent.UpdateAsync(Game(GameStore.Epic, title: "Lanterna Distante™"), null, force: false, TestContext.Current.CancellationToken);

        Assert.True(update.Queried);
        Assert.Equal("IGDB + Steam", update.Metadata!.Source);
        Assert.Equal(["IGDB", "Steam"], update.Metadata.Consulted);
        Assert.Equal(0, steam.Searches); // casou pelo appid que a IGDB informou, sem busca por nome
        Assert.Equal(["4000001"], steam.Fetched);
    }

    [Fact]
    public async Task Igdb_off_keeps_steam_only_and_turning_it_on_consults_only_igdb()
    {
        var steam = new FakeProvider("Steam", SteamPart, exactKey: "Steam");
        var igdb = new FakeProvider("IGDB", IgdbPart, exactKey: "Steam") { Enabled = false };
        var time = new FixedTimeProvider(Now);
        var agent = new MetadataAgent([steam, igdb], time);
        var game = Game(GameStore.Steam, steamAppId: 4000001);

        var first = (await agent.UpdateAsync(game, null, false, TestContext.Current.CancellationToken)).Metadata!;
        Assert.Equal("Steam", first.Source);
        Assert.Equal("Steam", agent.EnabledNames);
        Assert.False(agent.NeedsUpdate(game, first));

        igdb.Enabled = true;
        time.Now += TimeSpan.FromDays(1);
        Assert.True(agent.NeedsUpdate(game, first));
        var second = (await agent.UpdateAsync(game, first, false, TestContext.Current.CancellationToken)).Metadata!;

        Assert.Equal("Steam + IGDB", second.Source);
        Assert.Single(steam.Fetched); // a parte da Steam foi reaproveitada
        Assert.Equal(first.FetchedAt, second.FetchedAt); // o prazo de 30 dias continua o da rodada completa
        Assert.Equal("Steam e IGDB", agent.EnabledNames);
    }

    [Fact]
    public async Task Transient_failures_are_retried_and_misses_wait_a_week()
    {
        var steam = new FakeProvider("Steam", null);
        var time = new FixedTimeProvider(Now);
        var agent = new MetadataAgent([steam], time);
        var game = Game(GameStore.Manual, title: "Nada parecido");

        var miss = (await agent.UpdateAsync(game, null, false, TestContext.Current.CancellationToken)).Metadata!;
        Assert.True(miss.IsEmpty);
        Assert.False(agent.NeedsUpdate(game, miss));
        time.Now += TimeSpan.FromDays(8);
        Assert.True(agent.NeedsUpdate(game, miss));

        var flaky = new FakeProvider("Steam", null, exactKey: "Steam") { FailFetch = true };
        var transient = await new MetadataAgent([flaky], time).UpdateAsync(Game(GameStore.Steam, steamAppId: 1), null, false, TestContext.Current.CancellationToken);
        Assert.True(transient.Queried);
        Assert.Null(transient.Metadata); // nada gravado: tenta de novo na próxima rodada
    }

    [Fact]
    public async Task Pinning_an_igdb_result_brings_the_steam_data_and_is_kept_on_refresh()
    {
        var steam = new FakeProvider("Steam", SteamPart, exactKey: "Steam") { SearchResults = [MetadataMatch.Steam("Errado", 1)] };
        var igdb = new FakeProvider("IGDB", IgdbPart);
        var time = new FixedTimeProvider(Now);
        var agent = new MetadataAgent([steam, igdb], time);
        var game = Game(GameStore.Manual, title: "lanterna.exe");

        var pinned = await agent.PinAsync(game, new MetadataMatch("IGDB", "900001", "Lanterna Distante"), TestContext.Current.CancellationToken);

        Assert.True(pinned!.IsPinned);
        Assert.Equal("900001", pinned.IdIn("IGDB"));
        Assert.Equal(4000001, pinned.SteamAppId);
        Assert.Equal(0, steam.Searches);

        time.Now += TimeSpan.FromDays(40);
        var refreshed = (await agent.UpdateAsync(game, pinned, false, TestContext.Current.CancellationToken)).Metadata!;
        Assert.True(refreshed.IsPinned);
        Assert.Equal("900001", refreshed.IdIn("IGDB"));
        Assert.Equal(0, steam.Searches + igdb.Searches); // fixado: sem busca por nome
    }

    [Fact]
    public async Task Fix_match_lists_results_from_every_enabled_source_in_priority_order()
    {
        var steam = new FakeProvider("Steam", SteamPart) { SearchResults = [MetadataMatch.Steam("Lanterna Distante", 4000001)] };
        var igdb = new FakeProvider("IGDB", IgdbPart) { SearchResults = [new MetadataMatch("IGDB", "900001", "Lanterna Distante")] };
        var agent = new MetadataAgent([steam, igdb], new FixedTimeProvider(Now));

        var forEpic = await agent.SearchAsync(Game(GameStore.Epic, title: "Lanterna Distante"), TestContext.Current.CancellationToken);
        var forSteam = await agent.SearchAsync(Game(GameStore.Steam, title: "Lanterna Distante"), TestContext.Current.CancellationToken);

        Assert.Equal(["IGDB", "Steam"], forEpic.Select(m => m.Source));
        Assert.Equal(["Steam", "IGDB"], forSteam.Select(m => m.Source));
    }

    [Fact]
    public void Legacy_steam_only_cache_becomes_a_one_part_combination()
    {
        var legacy = new GameMetadata { Source = "Steam", SteamAppId = 570, FetchedAt = Now, Description = "Antigo" };

        var normalized = MetadataAgent.Normalize(legacy)!;

        Assert.Equal(["Steam"], normalized.Consulted);
        Assert.Single(normalized.Parts);
        Assert.Equal("570", normalized.IdIn("Steam"));
        Assert.Equal(["Steam"], MetadataAgent.Normalize(new GameMetadata { Source = "Steam", FetchedAt = Now })!.Consulted);
    }

    private static Game Game(GameStore store, string title = "Lanterna Distante", int? steamAppId = null, string? product = null) => new()
    {
        Id = $"{store.ToString().ToLowerInvariant()}:1",
        Title = title,
        Store = store,
        Accent = "#000000",
        SteamAppId = steamAppId,
        StoreProductId = product,
    };

    private sealed class FakeProvider(string name, GameMetadata? result, string? exactKey = null) : IMetadataProvider
    {
        public string Name => name;

        public bool Enabled { get; set; } = true;

        public bool IsEnabled => Enabled;

        public bool FailFetch { get; init; }

        public IReadOnlyList<MetadataMatch> SearchResults { get; init; } = [];

        public int Searches { get; private set; }

        public List<string> Fetched { get; } = [];

        public Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken)
        {
            Searches++;
            return Task.FromResult(SearchResults);
        }

        public Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken) =>
            Task.FromResult(exactKey is null ? null : lookup.ExternalIds.GetValueOrDefault(exactKey));

        public Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken)
        {
            Fetched.Add(id);
            return Task.FromResult(FailFetch ? null : result);
        }
    }
}
