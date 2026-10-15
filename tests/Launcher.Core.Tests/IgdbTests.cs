using System.Net;
using System.Text;
using System.Text.Json;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Igdb;
using Launcher.Core.Presentation;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>IGDB (ADR-0021) com respostas de exemplo escritas à mão: nenhuma chamada real à Twitch ou à IGDB.</summary>
public sealed class IgdbTests
{
    private static readonly string Fixtures = Path.Combine(AppContext.BaseDirectory, "Fixtures", "Igdb");
    private static readonly DateTimeOffset Now = new(2026, 10, 15, 12, 0, 0, TimeSpan.Zero);
    private static readonly IgdbSettings Key = new(true, "abcdefghijklmnopqrstuvwxyz0123", "secretsecretsecretsecretsecret");

    private static string Fixture(string name) => File.ReadAllText(Path.Combine(Fixtures, name));

    private static JsonElement Games() => JsonDocument.Parse(Fixture("games-lanterna.json")).RootElement;

    // ---------------------------------------------------------------- Consultas

    [Fact]
    public void Search_escapes_the_title_and_filters_by_platform()
    {
        string query = IgdbQueries.Search("Jogo \"citado\" \\ teste\n", IgdbPlatforms.IdsFor(null));

        Assert.StartsWith("search \"Jogo \\\"citado\\\" \\\\ teste\"; fields name, summary", query, StringComparison.Ordinal);
        Assert.Contains("where version_parent = null & platforms = (6); limit 10;", query, StringComparison.Ordinal);
        Assert.Contains("involved_companies.company.name", query, StringComparison.Ordinal);
        Assert.Contains("external_games.external_game_source", query, StringComparison.Ordinal);
        Assert.Contains("platforms = (19,58)", IgdbQueries.Search("Mario", IgdbPlatforms.IdsFor("snes")), StringComparison.Ordinal);
        Assert.DoesNotContain("platforms =", IgdbQueries.Search("Mario", IgdbPlatforms.IdsFor("desconhecida")), StringComparison.Ordinal);
    }

    [Fact]
    public void Batch_queries_list_ids_and_store_uids()
    {
        Assert.EndsWith("where id = (1,2,3); limit 3;", IgdbQueries.GamesById([3, 1, 2, 2]), StringComparison.Ordinal);
        Assert.Equal(
            "fields game, uid, external_game_source; where external_game_source = 1 & uid = (\"570\",\"730\"); limit 4;",
            IgdbQueries.ExternalGames(1, ["570", "730", "570"]));
        Assert.Equal(new Uri("https://images.igdb.com/igdb/image/upload/t_cover_big/abc.jpg"), IgdbQueries.Image("abc", IgdbParser.CoverSize));
    }

    [Fact]
    public void Platforms_map_console_keys_to_igdb_ids()
    {
        Assert.Equal([IgdbPlatforms.PcWindows], IgdbPlatforms.IdsFor(""));
        Assert.Equal([8], IgdbPlatforms.IdsFor("PS2"));
        Assert.Equal([29], IgdbPlatforms.IdsFor("megadrive"));
        Assert.Equal("SNES", IgdbPlatforms.NameOf(58));
        Assert.Null(IgdbPlatforms.NameOf(-1));
    }

    // ---------------------------------------------------------------- Respostas

    [Fact]
    public void Parses_a_game_with_portuguese_localization_companies_and_art()
    {
        var game = IgdbParser.Game(Games()[0], Now)!;

        Assert.Equal("IGDB", game.Source);
        Assert.Equal("Lanterna Distante: Edição Brasil", game.Title); // nome da região Brasil
        Assert.StartsWith("A lighthouse keeper", game.Description, StringComparison.Ordinal);
        Assert.Equal("en", game.DescriptionLanguage); // a IGDB não traduz resumos
        Assert.Equal(["Aventura", "Quebra-cabeça", "Indie"], game.Genres);
        Assert.Equal(["Fantasia", "Mistério"], game.Themes);
        Assert.Equal(["Um jogador"], game.GameModes);
        Assert.Equal("Maré Baixa Games", game.Developer);
        Assert.Equal("Estúdio Farol", game.Publisher);
        Assert.Equal(86, game.CriticScore);
        Assert.Equal("Lanterna", game.Franchise);
        Assert.Equal(new DateOnly(2024, 6, 1), game.ReleaseDate);
        Assert.Equal(new Uri("https://images.igdb.com/igdb/image/upload/t_cover_big/cobrfake.jpg"), game.CoverUrl);
        Assert.Equal(new Uri("https://images.igdb.com/igdb/image/upload/t_1080p/arfake01.jpg"), game.BackgroundUrl);
        Assert.Equal(4000001, game.SteamAppId);
        Assert.Equal("900001", game.IdIn("IGDB"));
        Assert.Equal("1400000001", game.IdIn("GOG"));
        Assert.Equal(3, game.SourceIds.Count); // fonte desconhecida (10) fica de fora
    }

    [Fact]
    public void Parses_sparse_games_with_screenshot_and_legacy_category()
    {
        var game = IgdbParser.Game(Games()[1], Now)!;

        Assert.Null(game.Description);
        Assert.Null(game.CoverUrl);
        Assert.Equal(new Uri("https://images.igdb.com/igdb/image/upload/t_screenshot_huge/scfake02.jpg"), game.BackgroundUrl);
        Assert.Equal("Série Lanterna", game.Franchise);
        Assert.Equal(4000002, game.SteamAppId); // "category" antigo ainda é lido
        Assert.Null(game.CriticScore);
        Assert.Null(IgdbParser.Game(JsonDocument.Parse("""{"name":"sem id"}""").RootElement, Now));
    }

    [Fact]
    public void Matches_show_year_and_platforms()
    {
        var first = IgdbParser.Match(Games()[0])!;
        var second = IgdbParser.Match(Games()[1])!;

        Assert.Equal(new MetadataMatch("IGDB", "900001", "Lanterna Distante") { Year = 2024, Platforms = "PC, Nintendo Switch" }, first);
        Assert.Equal("Lanterna Distante 2 (2026) · SNES · IGDB", GameText.MatchLabel(second));
        Assert.Equal("✓ Lanterna Distante (2024) · PC, Nintendo Switch · IGDB", GameText.MatchLabel(first with { IsExact = true }));
        Assert.Equal("Hades · Steam", GameText.MatchLabel(MetadataMatch.Steam("Hades", 1145360)));
    }

    [Fact]
    public void External_games_map_store_ids_to_igdb_games()
    {
        var map = IgdbParser.ExternalGames(JsonDocument.Parse(Fixture("external-games.json")).RootElement);

        Assert.Equal(900001, map["4000001"]);
        Assert.Equal(900002, map["4000002"]);
        Assert.Empty(IgdbParser.ExternalGames(JsonDocument.Parse("{}").RootElement));
    }

    [Theory]
    [InlineData("Role-playing (RPG)", "RPG")]
    [InlineData("Shooter", "Tiro")]
    [InlineData("Co-operative", "Cooperativo")]
    [InlineData("Gênero novo", "Gênero novo")]
    public void Translates_known_terms(string english, string portuguese) => Assert.Equal(portuguese, IgdbText.Translate(english));

    // ---------------------------------------------------------------- Limite, fila e cache

    [Fact]
    public void Rate_gate_allows_four_per_second()
    {
        var time = new FixedTimeProvider(Now);
        var gate = new RateGate(4, TimeSpan.FromSeconds(1), time);

        Assert.All(Enumerable.Range(0, 4), _ => Assert.Equal(TimeSpan.Zero, gate.Reserve()));
        Assert.Equal(TimeSpan.FromSeconds(1), gate.Reserve());
        time.Now += TimeSpan.FromSeconds(1.5);
        Assert.Equal(TimeSpan.Zero, gate.Reserve());
    }

    [Fact]
    public void Backoff_doubles_and_honors_retry_after()
    {
        Assert.Equal(TimeSpan.FromSeconds(1), IgdbClient.Backoff(1));
        Assert.Equal(TimeSpan.FromSeconds(4), IgdbClient.Backoff(3));
        Assert.Equal(TimeSpan.FromSeconds(7), IgdbClient.Backoff(1, TimeSpan.FromSeconds(7)));
        Assert.Equal(TimeSpan.FromSeconds(30), IgdbClient.Backoff(1, TimeSpan.FromMinutes(5)));
    }

    [Fact]
    public async Task Client_sends_headers_waits_on_429_and_caches()
    {
        var http = new FakeHttp();
        http.Igdb.Enqueue((HttpStatusCode.TooManyRequests, "[]"));
        http.Igdb.Enqueue((HttpStatusCode.OK, Fixture("games-lanterna.json")));
        var delays = new List<TimeSpan>();
        var cache = new MemoryCache();
        using var client = Client(http, new FakeAuth(), cache, delays);

        var first = await client.QueryAsync("games", "fields name; limit 1;", TestContext.Current.CancellationToken);
        var again = await client.QueryAsync("games", "fields name; limit 1;", TestContext.Current.CancellationToken);

        Assert.Equal(2, first.GetArrayLength());
        Assert.Equal(2, again.GetArrayLength());
        Assert.Equal(2, client.NetworkRequests); // 429 + sucesso; a repetição veio do cache
        Assert.Contains(TimeSpan.FromSeconds(1), delays);
        var request = http.Requests[^1];
        Assert.Equal("https://api.igdb.com/v4/games", request.Uri);
        Assert.Equal("cliente", request.ClientId);
        Assert.Equal("Bearer token-1", request.Authorization);
        Assert.Equal("fields name; limit 1;", request.Body);
    }

    [Fact]
    public async Task Client_renews_the_token_once_on_401_and_reports_failures()
    {
        var http = new FakeHttp();
        http.Igdb.Enqueue((HttpStatusCode.Unauthorized, "{}"));
        http.Igdb.Enqueue((HttpStatusCode.OK, "[]"));
        var auth = new FakeAuth();
        using var client = Client(http, auth, null, []);

        await client.QueryAsync("games", IgdbQueries.Ping, TestContext.Current.CancellationToken);

        Assert.Equal(1, auth.Invalidated);
        Assert.Equal("Bearer token-2", http.Requests[^1].Authorization);

        http.Igdb.Enqueue((HttpStatusCode.Unauthorized, "{}"));
        http.Igdb.Enqueue((HttpStatusCode.Unauthorized, "{}"));
        var denied = await Assert.ThrowsAsync<IgdbException>(() => client.QueryAsync("games", "fields id;", TestContext.Current.CancellationToken));
        Assert.Equal(IgdbFailure.Unauthorized, denied.Failure);

        for (int i = 0; i < 4; i++)
        {
            http.Igdb.Enqueue((HttpStatusCode.TooManyRequests, "[]"));
        }

        var limited = await Assert.ThrowsAsync<IgdbException>(() => client.QueryAsync("games", "fields slug;", TestContext.Current.CancellationToken));
        Assert.Equal(IgdbFailure.RateLimited, limited.Failure);
    }

    [Fact]
    public void File_cache_expires_after_its_max_age()
    {
        string folder = Path.Combine(Path.GetTempPath(), $"tako-igdb-{Guid.NewGuid():N}");
        try
        {
            var time = new FixedTimeProvider(DateTimeOffset.UtcNow);
            var cache = new IgdbFileCache(folder, time, TimeSpan.FromDays(30));
            cache.Write("games\nfields name;", "[1]");

            Assert.Equal("[1]", cache.TryRead("games\nfields name;"));
            Assert.Null(cache.TryRead("games\nfields id;"));
            time.Now += TimeSpan.FromDays(31);
            Assert.Null(cache.TryRead("games\nfields name;"));
        }
        finally
        {
            if (Directory.Exists(folder))
            {
                Directory.Delete(folder, recursive: true);
            }
        }
    }

    // ---------------------------------------------------------------- Chave da Twitch

    [Fact]
    public async Task Twitch_token_is_reused_and_renewed_before_expiring_or_when_the_key_changes()
    {
        var http = new FakeHttp();
        var settings = Key;
        var time = new FixedTimeProvider(Now);
        using var auth = new TwitchAppAuthenticator(http.Client, () => settings, time);

        var first = await auth.GetAccessAsync(TestContext.Current.CancellationToken);
        var cached = await auth.GetAccessAsync(TestContext.Current.CancellationToken);
        time.Now += TimeSpan.FromSeconds(3600);
        var renewed = await auth.GetAccessAsync(TestContext.Current.CancellationToken);
        settings = settings with { ClientSecret = "outrosegredooutrosegredooutros" };
        var newKey = await auth.GetAccessAsync(TestContext.Current.CancellationToken);

        Assert.Equal(new IgdbAccess(Key.ClientId, "twitch-1"), first);
        Assert.Equal(first, cached);
        Assert.Equal("twitch-2", renewed.AccessToken);
        Assert.Equal("twitch-3", newKey.AccessToken);
        Assert.Equal(3, http.TokenRequests.Count);
        Assert.Contains("grant_type=client_credentials", http.TokenRequests[0], StringComparison.Ordinal);
        Assert.Contains($"client_id={Key.ClientId}", http.TokenRequests[0], StringComparison.Ordinal);
    }

    [Theory]
    [InlineData(HttpStatusCode.BadRequest, """{"status":400,"message":"invalid client"}""", IgdbFailure.InvalidClientId)]
    [InlineData(HttpStatusCode.Forbidden, """{"status":403,"message":"invalid client secret"}""", IgdbFailure.InvalidSecret)]
    [InlineData(HttpStatusCode.TooManyRequests, "{}", IgdbFailure.RateLimited)]
    [InlineData(HttpStatusCode.BadGateway, "", IgdbFailure.Unavailable)]
    public void Twitch_errors_become_clear_reasons(HttpStatusCode status, string body, IgdbFailure expected)
    {
        var failure = TwitchAppAuthenticator.TokenFailure(status, body).Failure;

        Assert.Equal(expected, failure);
        Assert.False(string.IsNullOrWhiteSpace(IgdbErrors.Message(failure)));
    }

    [Fact]
    public async Task Without_a_key_nothing_is_requested()
    {
        var http = new FakeHttp();
        using var auth = new TwitchAppAuthenticator(http.Client, () => IgdbSettings.Off with { ClientId = Key.ClientId, ClientSecret = Key.ClientSecret }, TimeProvider.System);
        using var client = new IgdbClient(http.Client, auth, TimeProvider.System);
        var provider = new IgdbProvider(client, TimeProvider.System);

        Assert.False(provider.IsEnabled);
        Assert.Empty(await provider.SearchAsync(new MetadataLookup("Lanterna", new Dictionary<string, string>()), TestContext.Current.CancellationToken));
        Assert.Null(await provider.FetchAsync("900001", TestContext.Current.CancellationToken));
        var error = await Assert.ThrowsAsync<IgdbException>(() => auth.GetAccessAsync(TestContext.Current.CancellationToken));
        Assert.Equal(IgdbFailure.NotConfigured, error.Failure);
        Assert.Empty(http.Requests);
        Assert.Empty(http.TokenRequests);
    }

    [Fact]
    public void Settings_are_stored_protected_and_validated()
    {
        string path = Path.Combine(Path.GetTempPath(), $"tako-igdb-{Guid.NewGuid():N}.dat");
        try
        {
            var protector = new ReversingProtector();
            new IgdbSettingsStore(path, protector).Save(Key with { ClientId = $" {Key.ClientId} " });

            Assert.DoesNotContain(Key.ClientSecret, File.ReadAllText(path), StringComparison.Ordinal);
            var loaded = new IgdbSettingsStore(path, protector).Current;
            Assert.Equal(Key, loaded);
            Assert.True(loaded.IsActive);
            Assert.DoesNotContain(Key.ClientSecret, loaded.ToString(), StringComparison.Ordinal);

            File.WriteAllText(path, "corrompido");
            Assert.Equal(IgdbSettings.Off, new IgdbSettingsStore(path, new ReversingProtector()).Current);
        }
        finally
        {
            File.Delete(path);
        }

        Assert.Equal("abc123", IgdbSettings.Clean(" abc\n12 3\t"));
        Assert.Null(Key.Problem());
        Assert.Contains("Client ID", (Key with { ClientId = "" }).Problem(), StringComparison.Ordinal);
        Assert.Contains("Client Secret", (Key with { ClientSecret = "" }).Problem(), StringComparison.Ordinal);
        Assert.Contains("incompleto", (Key with { ClientId = "abc123" }).Problem(), StringComparison.Ordinal);
        Assert.False((Key with { Enabled = false }).IsActive);
    }

    // ---------------------------------------------------------------- Provedor

    [Fact]
    public async Task Provider_matches_store_ids_exactly_and_reuses_what_it_already_fetched()
    {
        var http = new FakeHttp();
        http.Igdb.Enqueue((HttpStatusCode.OK, Fixture("external-games.json")));
        http.Igdb.Enqueue((HttpStatusCode.OK, Fixture("games-lanterna.json")));
        using var client = Client(http, new FakeAuth(), null, []);
        var provider = new IgdbProvider(client, new FixedTimeProvider(Now));
        var steam = new Dictionary<string, string> { ["Steam"] = "4000001" };

        string? id = await provider.FindExactAsync(new MetadataLookup("Outro nome", steam), TestContext.Current.CancellationToken);
        var metadata = await provider.FetchAsync(id!, TestContext.Current.CancellationToken);
        var gog = await provider.FindExactAsync(new MetadataLookup("x", new Dictionary<string, string> { ["GOG"] = "1400000001" }), TestContext.Current.CancellationToken);

        Assert.Equal("900001", id);
        Assert.Equal("Lanterna Distante: Edição Brasil", metadata?.Title);
        Assert.Equal("900001", gog); // aprendido dos ids que o jogo trouxe, sem nova consulta
        Assert.Equal(2, client.NetworkRequests);
        Assert.Contains("where external_game_source = 1 & uid = (\"4000001\")", http.Requests[0].Body, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Provider_prefetches_the_library_in_batches()
    {
        var http = new FakeHttp();
        http.Igdb.Enqueue((HttpStatusCode.OK, Fixture("external-games.json")));
        http.Igdb.Enqueue((HttpStatusCode.OK, Fixture("games-lanterna.json")));
        using var client = Client(http, new FakeAuth(), null, []);
        var provider = new IgdbProvider(client, new FixedTimeProvider(Now));
        MetadataLookup[] lookups =
        [
            new("A", new Dictionary<string, string> { ["Steam"] = "4000001" }),
            new("B", new Dictionary<string, string> { ["Steam"] = "4000002" }),
            new("C", new Dictionary<string, string> { ["Steam"] = "4000009" }),
        ];

        await provider.PrefetchAsync(lookups, TestContext.Current.CancellationToken);
        foreach (var lookup in lookups)
        {
            _ = await provider.FindExactAsync(lookup, TestContext.Current.CancellationToken);
        }

        Assert.NotNull(await provider.FetchAsync("900002", TestContext.Current.CancellationToken));
        Assert.Equal(2, client.NetworkRequests); // um external_games e um games para a biblioteca toda
        Assert.Contains("uid = (\"4000001\",\"4000002\",\"4000009\")", http.Requests[0].Body, StringComparison.Ordinal);
        Assert.Contains("where id = (900001,900002)", http.Requests[1].Body, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Provider_searches_by_title_and_platform_and_flags_exact_matches()
    {
        var http = new FakeHttp();
        http.Igdb.Enqueue((HttpStatusCode.OK, Fixture("games-lanterna.json")));
        using var client = Client(http, new FakeAuth(), null, []);
        var provider = new IgdbProvider(client, new FixedTimeProvider(Now));

        var results = await provider.SearchAsync(new MetadataLookup("Lanterna Distante", new Dictionary<string, string>(), "snes"), TestContext.Current.CancellationToken);

        Assert.Equal(["900001", "900002"], results.Select(r => r.Id));
        Assert.All(results, r => Assert.False(r.IsExact));
        Assert.Contains("platforms = (19,58)", http.Requests[0].Body, StringComparison.Ordinal);
    }

    [Fact]
    public async Task Provider_failures_do_not_throw_during_background_updates()
    {
        var http = new FakeHttp();
        http.Igdb.Enqueue((HttpStatusCode.InternalServerError, "erro"));
        using var client = Client(http, new FakeAuth(), null, []);
        var provider = new IgdbProvider(client, new FixedTimeProvider(Now));

        Assert.Empty(await provider.SearchAsync(new MetadataLookup("Lanterna", new Dictionary<string, string>()), TestContext.Current.CancellationToken));
    }

    // ---------------------------------------------------------------- Apoio

    private static IgdbClient Client(FakeHttp http, IIgdbAuthenticator auth, IIgdbResponseCache? cache, List<TimeSpan> delays) =>
        new(http.Client, auth, TimeProvider.System, cache, (wait, _) =>
        {
            delays.Add(wait);
            return Task.CompletedTask;
        });

    private sealed class FakeAuth : IIgdbAuthenticator
    {
        private int _token = 1;

        public int Invalidated { get; private set; }

        public bool IsConfigured => true;

        public Task<IgdbAccess> GetAccessAsync(CancellationToken cancellationToken) => Task.FromResult(new IgdbAccess("cliente", $"token-{_token}"));

        public void Invalidate()
        {
            Invalidated++;
            _token++;
        }
    }

    private sealed class MemoryCache : IIgdbResponseCache
    {
        private readonly Dictionary<string, string> _items = [];

        public string? TryRead(string key) => _items.GetValueOrDefault(key);

        public void Write(string key, string json) => _items[key] = json;
    }

    /// <summary>"Cifra" de teste: inverte o texto, para provar que o arquivo não guarda o segredo em claro.</summary>
    private sealed class ReversingProtector : ISecretProtector
    {
        public string Protect(string secret) => "rev:" + new string([.. secret.Reverse()]);

        public bool TryUnprotect(string stored, out string secret)
        {
            secret = stored.StartsWith("rev:", StringComparison.Ordinal) ? new string([.. stored[4..].Reverse()]) : string.Empty;
            return secret.Length > 0;
        }
    }

    internal sealed record SentRequest(string Uri, string? ClientId, string? Authorization, string Body);

    /// <summary>Twitch e IGDB de mentira: respostas em fila para a IGDB e tokens numerados para a Twitch.</summary>
    internal sealed class FakeHttp : HttpMessageHandler
    {
        private int _tokens;

        public FakeHttp() => Client = new HttpClient(this);

        public HttpClient Client { get; }

        public Queue<(HttpStatusCode Status, string Body)> Igdb { get; } = new();

        public List<SentRequest> Requests { get; } = [];

        public List<string> TokenRequests { get; } = [];

        protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            string body = request.Content is null ? string.Empty : await request.Content.ReadAsStringAsync(cancellationToken);
            if (request.RequestUri == TwitchAppAuthenticator.TokenEndpoint)
            {
                TokenRequests.Add(body);
                return Json(HttpStatusCode.OK, $$"""{"access_token":"twitch-{{++_tokens}}","expires_in":3600,"token_type":"bearer"}""");
            }

            Requests.Add(new SentRequest(
                request.RequestUri!.ToString(),
                request.Headers.TryGetValues("Client-ID", out var id) ? id.Single() : null,
                request.Headers.Authorization?.ToString(),
                body));
            var (status, text) = Igdb.Count > 0 ? Igdb.Dequeue() : (HttpStatusCode.OK, "[]");
            return Json(status, text);
        }

        private static HttpResponseMessage Json(HttpStatusCode status, string body) =>
            new(status) { Content = new StringContent(body, Encoding.UTF8, "application/json") };
    }
}
