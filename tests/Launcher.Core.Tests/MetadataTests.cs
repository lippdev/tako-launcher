using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using System.Net;
using System.Text;
using System.Text.Json;
using Xunit;

namespace Launcher.Core.Tests;

public class MetadataTests
{
    [Theory]
    [InlineData("Hollow Knight: Silksong", "Hollow Knight Silksong", 1.0)]
    [InlineData("God of War III", "God of War 3", 1.0)]
    [InlineData("Pokémon", "Pokemon", 1.0)]
    [InlineData("The Witcher 3: Wild Hunt GOTY", "The Witcher® 3: Wild Hunt", 1.0)]
    [InlineData("eldenring", "ELDEN RING", 0.95)]
    public void Equivalent_titles_match_exactly(string local, string candidate, double expected) =>
        Assert.Equal(expected, TitleMatcher.Score(local, candidate), 2);

    [Fact]
    public void Search_terms_add_a_clean_title_when_it_differs()
    {
        Assert.Equal(["The Witcher 3 Wild Hunt GOTY", "witcher 3 wild hunt"], TitleMatcher.SearchTerms("The Witcher 3 Wild Hunt GOTY"));
        Assert.Equal(["hades 2"], TitleMatcher.SearchTerms("hades 2"));
    }

    [Fact]
    public void Picks_the_closest_result_and_rejects_weak_ones()
    {
        MetadataMatch[] results =
        [
            new("Hollow Knight", 367520),
            new("Hollow Knight: Silksong", 1030300),
            new("Hollow Knight Soundtrack", 1),
        ];

        Assert.Equal(1030300, TitleMatcher.Best("silksong hollow knight", results)?.SteamAppId);
        Assert.Equal(367520, TitleMatcher.Best("Hollow Knight", results)?.SteamAppId);
        Assert.Null(TitleMatcher.Best("Forza Horizon 5", results));
        Assert.Null(TitleMatcher.Best("Minecraft Launcher", [new MetadataMatch("Minecraft Dungeons II", 1912410), new MetadataMatch("Minecraft Legends", 1928870)]));
    }

    [Fact]
    public void Metadata_fills_game_and_gives_manual_games_steam_art()
    {
        var game = new Game { Id = "manual:1", Title = "Silksong", Store = GameStore.Manual, Accent = "#000000", Description = "" };
        var metadata = new GameMetadata
        {
            Source = "Steam",
            SteamAppId = 1030300,
            Description = "Descubra um reino vasto",
            Genres = ["Ação", "Aventura"],
            ReleaseDate = new DateOnly(2025, 9, 4),
            Developer = "Team Cherry",
            MetacriticScore = 91,
        };

        metadata.ApplyTo(game);

        Assert.Equal("Descubra um reino vasto", game.Description);
        Assert.Equal(["Ação", "Aventura"], game.Genres);
        Assert.Equal(91, game.MetacriticScore);
        Assert.Equal(new Uri("steam-art://1030300/cover"), game.CoverUri);
    }

    [Fact]
    public void Metadata_keeps_a_cover_chosen_by_the_user()
    {
        var cover = new Uri("file:///C:/capas/silksong.png");
        var game = new Game { Id = "manual:1", Title = "Silksong", Store = GameStore.Manual, Accent = "#000000", CoverUri = cover };

        new GameMetadata { Source = "Steam", SteamAppId = 1030300 }.ApplyTo(game);

        Assert.Equal(cover, game.CoverUri);
    }

    [Theory]
    [InlineData("4/set./2025", 2025, 9, 4)]
    [InlineData("24 de out. de 2026", 2026, 10, 24)]
    [InlineData("9 Jul, 2013", 2013, 7, 9)]
    [InlineData("Jul 9, 2013", 2013, 7, 9)]
    public void Parses_store_dates(string text, int year, int month, int day) =>
        Assert.Equal(new DateOnly(year, month, day), SteamStoreProvider.ParseDate(text));

    [Theory]
    [InlineData("Em breve")]
    [InlineData("")]
    [InlineData(null)]
    public void Unknown_dates_are_null(string? text) => Assert.Null(SteamStoreProvider.ParseDate(text));

    [Fact]
    public void Parses_appdetails_in_portuguese()
    {
        using var json = JsonDocument.Parse("""
            {"1030300":{"success":true,"data":{"name":"Hollow Knight: Silksong",
              "short_description":"Descubra um reino &quot;vasto&quot;",
              "genres":[{"id":"1","description":"Ação"},{"id":"25","description":"Aventura"}],
              "release_date":{"coming_soon":false,"date":"4/set./2025"},
              "developers":["Team Cherry"],"publishers":["Team Cherry"],
              "metacritic":{"score":90,"url":"https://www.metacritic.com/"}}}}
            """);

        var metadata = SteamStoreProvider.Parse(json.RootElement, 1030300, DateTimeOffset.UnixEpoch)!;

        Assert.Equal("Descubra um reino \"vasto\"", metadata.Description);
        Assert.Equal(["Ação", "Aventura"], metadata.Genres);
        Assert.Equal(new DateOnly(2025, 9, 4), metadata.ReleaseDate);
        Assert.Equal("Team Cherry", metadata.Developer);
        Assert.Equal(90, metadata.MetacriticScore);
        Assert.Null(SteamStoreProvider.Parse(JsonDocument.Parse("""{"1":{"success":false}}""").RootElement, 1, DateTimeOffset.UnixEpoch));
    }

    [Fact]
    public async Task Search_keeps_only_apps()
    {
        using var http = new HttpClient(new StubHandler("""
            {"total":2,"items":[{"type":"app","name":"Hollow Knight: Silksong","id":1030300},{"type":"sub","name":"Pacote","id":5}]}
            """));

        var results = await new SteamStoreProvider(http).SearchAsync("silksong", TestContext.Current.CancellationToken);

        Assert.Equal([new MetadataMatch("Hollow Knight: Silksong", 1030300)], results);
    }

    [Fact]
    public async Task Rate_limited_requests_return_nothing()
    {
        using var http = new HttpClient(new StubHandler("{}", HttpStatusCode.TooManyRequests));

        Assert.Null(await new SteamStoreProvider(http).FetchAsync(570, TestContext.Current.CancellationToken));
    }

    private sealed class StubHandler(string body, HttpStatusCode status = HttpStatusCode.OK) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken) =>
            Task.FromResult(new HttpResponseMessage(status) { Content = new StringContent(body, Encoding.UTF8, "application/json") });
    }
}
