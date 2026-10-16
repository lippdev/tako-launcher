using System.IO.Compression;
using System.Net;
using System.Reflection;
using System.Text;
using System.Text.Json;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Igdb;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// ScreenScraper (ADR-0022) com respostas JSON escritas à mão e ROMs falsas geradas no teste: nenhuma chamada real ao
/// ScreenScraper e nenhum jogo de verdade no repositório.
/// </summary>
public sealed class ScreenScraperTests : IDisposable
{
    private static readonly string Fixtures = Path.Combine(AppContext.BaseDirectory, "Fixtures", "ScreenScraper");
    private static readonly DateTimeOffset Now = new(2026, 10, 15, 12, 0, 0, TimeSpan.Zero);
    private static readonly ScreenScraperSoftware Software = new("dev", "devpass", "Tako");
    private static readonly ScreenScraperSettings Account = new(true, "jogador", "senha secreta");

    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-ss-" + Guid.NewGuid().ToString("N"));

    public ScreenScraperTests() => Directory.CreateDirectory(_root);

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    private static string Fixture(string name) => File.ReadAllText(Path.Combine(Fixtures, name));

    private static JsonElement Response(string name) => ScreenScraperParser.Envelope(Fixture(name))!.Value;

    // ---------------------------------------------------------------- Sistemas

    [Theory]
    [InlineData("nes", 3)]
    [InlineData("snes", 4)]
    [InlineData("n64", 14)]
    [InlineData("gamecube", 13)]
    [InlineData("wii", 16)]
    [InlineData("gb", 9)]
    [InlineData("gbc", 10)]
    [InlineData("gba", 12)]
    [InlineData("nds", 15)]
    [InlineData("3ds", 17)]
    [InlineData("switch", 225)]
    [InlineData("mastersystem", 2)]
    [InlineData("megadrive", 1)]
    [InlineData("saturn", 22)]
    [InlineData("dreamcast", 23)]
    [InlineData("ps1", 57)]
    [InlineData("ps2", 58)]
    [InlineData("psp", 61)]
    [InlineData("ps3", 59)]
    [InlineData("xbox", 32)]
    [InlineData("arcade", 75)]
    [InlineData("neogeo", 142)]
    [InlineData("pcengine", 31)]
    [InlineData("atari2600", 26)]
    public void Systems_map_platform_keys_to_screenscraper_ids(string key, int id)
    {
        Assert.Equal(id, ScreenScraperSystems.ForKey(key)!.ScreenScraperId);
        Assert.Equal(id, ScreenScraperSystems.ForKey(key.ToUpperInvariant())!.ScreenScraperId);
        Assert.Equal(key, ScreenScraperSystems.ForScreenScraperId(id)!.Key);
    }

    [Fact]
    public void Systems_share_keys_with_igdb_and_pc_is_not_a_system()
    {
        Assert.Null(ScreenScraperSystems.ForKey(null));
        Assert.Null(ScreenScraperSystems.ForKey("pc"));
        Assert.Null(ScreenScraperSystems.ForScreenScraperId(-1));
        Assert.Equal(ScreenScraperSystems.All.Count, ScreenScraperSystems.All.Select(s => s.Key).Distinct().Count());
        foreach (var system in ScreenScraperSystems.All)
        {
            Assert.NotEmpty(IgdbPlatforms.IdsFor(system.Key)); // a IGDB complementa com o filtro da mesma plataforma
            Assert.All(system.Extensions, e => Assert.StartsWith(".", e, StringComparison.Ordinal));
        }
    }

    [Fact]
    public void Candidates_come_from_the_extension()
    {
        Assert.Equal(["snes"], ScreenScraperSystems.CandidatesFor(@"D:\roms\Lanterna (USA).SFC").Select(s => s.Key));
        Assert.Equal(["gba"], ScreenScraperSystems.CandidatesFor("lanterna.gba").Select(s => s.Key));
        var zip = ScreenScraperSystems.CandidatesFor("lanterna.zip").Select(s => s.Key).ToList();
        Assert.Equal(["arcade", "neogeo"], zip.Take(2));
        Assert.Contains("snes", zip);
        Assert.DoesNotContain("ps2", zip);
        Assert.Contains("ps1", ScreenScraperSystems.CandidatesFor("lanterna.cue").Select(s => s.Key));
        Assert.Empty(ScreenScraperSystems.CandidatesFor("lanterna.exe"));
        Assert.Empty(ScreenScraperSystems.CandidatesFor("lanterna"));

        Assert.True(ScreenScraperSystems.LooksLikeRom("lanterna.sfc"));
        Assert.True(ScreenScraperSystems.LooksLikeRom("lanterna.chd"));
        Assert.False(ScreenScraperSystems.LooksLikeRom("lanterna.exe"));
        Assert.False(ScreenScraperSystems.LooksLikeRom("lanterna.lnk"));
        Assert.False(ScreenScraperSystems.LooksLikeRom("leia-me.txt"));
        Assert.Contains("*.sfc", ScreenScraperSystems.FileDialogPattern(), StringComparison.Ordinal);
    }

    [Theory]
    [InlineData("Super Lanterna World (USA).sfc", "Super Lanterna World")]
    [InlineData("Lanterna (Europe) (Rev 1) [!].zip", "Lanterna")]
    [InlineData("lanterna_distante_v1.gba", "lanterna distante v1")]
    [InlineData("(Beta).nes", "(Beta)")]
    public void Rom_titles_drop_dat_tags(string file, string title) => Assert.Equal(title, RomTitle.FromFileName(file));

    [Fact]
    public void Rom_records_fill_platform_and_never_launch()
    {
        string path = WriteRom("Lanterna Distante (Brazil).sfc", "123456789");
        var record = ManualGameRecord.CreateRom(path, "snes", Now);
        var game = record.ToGame();

        Assert.Equal("Lanterna Distante", record.Title);
        Assert.Equal("snes", game.Platform);
        Assert.Equal(path, game.RomPath);
        Assert.True(game.IsRom);
        Assert.True(game.IsInstalled);
        Assert.True(game.CanPlay); // abre pelo emulador do sistema (ADR-0024)
        Assert.Equal(MetadataKind.Emulated, MetadataPriority.KindOf(game));
        Assert.Equal(path, MetadataLookup.For(game).RomPath);
        Assert.Equal("ROM no disco", Presentation.GameText.Status(game));
        Assert.Null(ManualGameRecord.Create("Jogo", path, Now).ToGame().RomPath);
    }

    // ---------------------------------------------------------------- Hashes

    [Fact]
    public void Hashes_match_the_standard_test_vectors()
    {
        byte[] data = Encoding.ASCII.GetBytes("123456789");
        Assert.Equal("CBF43926", Crc32.Compute(data));

        var full = RomHasher.Hash(new MemoryStream(data), "a.sfc", data.Length, full: true, TestContext.Current.CancellationToken);
        Assert.Equal(new RomIdentity("a.sfc", 9, "CBF43926", "25F9E794323B453885F5181F1B624D0B", "F7C3BC1D808E04732ADF679965CCC34CA7AE3441"), full);

        // Arquivo grande (ISO): só CRC, que o ScreenScraper aceita com nome e tamanho.
        var crcOnly = RomHasher.Hash(new MemoryStream(data), "a.iso", data.Length, full: false, TestContext.Current.CancellationToken);
        Assert.Equal(new RomIdentity("a.iso", 9, "CBF43926"), crcOnly);
    }

    [Fact]
    public void Zip_hashes_the_inner_rom_except_for_arcade_sets()
    {
        string zip = Path.Combine(_root, "lanterna.zip");
        using (var archive = ZipFile.Open(zip, ZipArchiveMode.Create))
        {
            using var stream = archive.CreateEntry("Lanterna (USA).sfc").Open();
            stream.Write(Encoding.ASCII.GetBytes("123456789"));
        }

        var inner = RomHasher.Identify(zip, ScreenScraperSystems.ForKey("snes"), TestContext.Current.CancellationToken);
        Assert.Equal("Lanterna (USA).sfc", inner.FileName);
        Assert.Equal(9, inner.Size);
        Assert.Equal("CBF43926", inner.Crc32);

        var set = RomHasher.Identify(zip, ScreenScraperSystems.ForKey("arcade"), TestContext.Current.CancellationToken);
        Assert.Equal("lanterna.zip", set.FileName);
        Assert.Equal(new FileInfo(zip).Length, set.Size);
        Assert.NotEqual("CBF43926", set.Crc32);
        Assert.NotNull(set.Sha1);

        var chd = RomHasher.Identify(WriteRom("lanterna.chd", "não importa"), ScreenScraperSystems.ForKey("ps1"), TestContext.Current.CancellationToken);
        Assert.Equal(new RomIdentity("lanterna.chd", 12), chd); // contêiner compactado: só nome e tamanho
    }

    [Fact]
    public async Task Hash_cache_survives_restarts_and_notices_changed_files()
    {
        string rom = WriteRom("lanterna.sfc", "123456789");
        string cacheFile = Path.Combine(_root, "cache", "rom-hashes.json");
        var snes = ScreenScraperSystems.ForKey("snes");

        var first = await new RomHashCache(cacheFile).GetAsync(rom, snes, TestContext.Current.CancellationToken);
        Assert.Equal("CBF43926", first.Crc32);
        Assert.True(File.Exists(cacheFile));

        // Outra abertura do Tako: vem do arquivo, mesmo se o conteúdo em disco mudasse sem mudar tamanho e data.
        string key = RomHashCache.KeyFor(new FileInfo(rom), snes);
        Assert.Contains(JsonEncodedText.Encode(key).ToString(), File.ReadAllText(cacheFile), StringComparison.Ordinal);
        Assert.Equal(first, await new RomHashCache(cacheFile).GetAsync(rom, snes, TestContext.Current.CancellationToken));

        File.WriteAllText(rom, "1234567890");
        File.SetLastWriteTimeUtc(rom, DateTime.UtcNow.AddMinutes(1));
        var changed = await new RomHashCache(cacheFile).GetAsync(rom, snes, TestContext.Current.CancellationToken);
        Assert.Equal(10, changed.Size);
        Assert.NotEqual(first.Crc32, changed.Crc32);
    }

    // ---------------------------------------------------------------- Respostas

    [Fact]
    public void Game_info_prefers_brazil_portuguese_and_regional_media()
    {
        var game = ScreenScraperParser.GameInfo(Response("jeuInfos-lanterna.json"), Now)!;

        Assert.Equal(MetadataPriority.ScreenScraper, game.Source);
        Assert.Equal("900077", game.IdIn(MetadataPriority.ScreenScraper));
        Assert.Equal("Lanterna Distante", game.Title); // região br antes de wor
        Assert.StartsWith("Um faroleiro", game.Description, StringComparison.Ordinal);
        Assert.Equal("pt-BR", game.DescriptionLanguage);
        Assert.Equal(["Plataforma", "Quebra-cabeça"], game.Genres); // pt quando há; inglês traduzido
        Assert.Equal(new DateOnly(1994, 8, 1), game.ReleaseDate); // us antes de jp; "1994-08"
        Assert.Equal("Maré Baixa Games", game.Developer);
        Assert.Equal("Estúdio Farol", game.Publisher);
        Assert.Equal(80, game.CriticScore); // 16/20
        Assert.Equal("1-2", game.Players);

        var cover = new Uri("ssmedia://neoclone.screenscraper.fr/api2/mediaJeu.php?systemeid=4&jeuid=900077&media=box-2D(us)");
        Assert.Equal(cover, game.CoverUrl); // us antes de jp; o "br" de outro host é ignorado
        Assert.Equal(new Uri("ssmedia://neoclone.screenscraper.fr/api2/mediaJeu.php?systemeid=4&jeuid=900077&media=fanart"), game.BackgroundUrl);
        Assert.EndsWith("media=box-3D(us)", game.Media[GameMedia.Box3D].ToString(), StringComparison.Ordinal);
        Assert.EndsWith("media=wheel-hd(wor)", game.Media[GameMedia.Logo].ToString(), StringComparison.Ordinal);
        Assert.EndsWith("media=screenmarquee(wor)", game.Media[GameMedia.Marquee].ToString(), StringComparison.Ordinal);
        Assert.EndsWith("media=ss(wor)", game.Media[GameMedia.Screenshot].ToString(), StringComparison.Ordinal);
        Assert.EndsWith("media=video-normalized", game.Media[GameMedia.Video].ToString(), StringComparison.Ordinal);
        Assert.False(game.Media.ContainsKey(GameMedia.Box2D));

        // Nenhuma credencial da resposta vai para os metadados (que ficam no cache em disco).
        string json = JsonSerializer.Serialize(game);
        Assert.DoesNotContain("SEGREDOFALSO", json, StringComparison.Ordinal);
        Assert.DoesNotContain("senhafalsa", json, StringComparison.Ordinal);
        Assert.DoesNotContain("DEVFALSO", json, StringComparison.Ordinal);
    }

    [Fact]
    public void Search_results_show_title_year_and_system()
    {
        var results = ScreenScraperParser.Search(Response("jeuRecherche-lanterna.json"));

        Assert.Equal(2, results.Count);
        Assert.Equal(new MetadataMatch(MetadataPriority.ScreenScraper, "900077", "Lanterna Distante") { Year = 1994, Platforms = "SNES" }, results[0]);
        Assert.Equal("Sistema Desconhecido", results[1].Platforms);
        Assert.Equal(1996, results[1].Year);
        Assert.Equal("Lanterna Distante (1994) · SNES · ScreenScraper", Presentation.GameText.MatchLabel(results[0]));
        Assert.Empty(ScreenScraperParser.Search(Response("jeuRecherche-vazia.json"))); // [{}] quando não acha
    }

    [Fact]
    public void Account_and_envelope_are_read_defensively()
    {
        var account = ScreenScraperParser.Account(Response("ssuserInfos.json"))!;
        Assert.Equal(new ScreenScraperAccount("jogador", 1, 1, 20, 20000) { KoToday = 0, MaxKoPerDay = 2000 }, account);
        Assert.Equal(19980, account.RemainingToday);
        Assert.False(account.IsExhausted);
        Assert.True((account with { RequestsToday = 20000 }).IsExhausted);
        Assert.True((account with { KoToday = 2000 }).IsExhausted);

        Assert.Null(ScreenScraperParser.Envelope("Erreur : Rom/Iso/Dossier non trouvée !"));
        Assert.Null(ScreenScraperParser.Envelope("""{"header":{"success":"false","error":"x"},"response":{}}"""));
        Assert.Null(ScreenScraperParser.Account(ScreenScraperParser.Envelope("""{"response":{}}""")!.Value));
        Assert.Null(ScreenScraperParser.GameInfo(ScreenScraperParser.Envelope("""{"response":{"jeu":{"id":"1"}}}""")!.Value, Now)); // sem nome
        Assert.Equal(new DateOnly(1996, 1, 1), ScreenScraperParser.ParseDate("1996"));
        Assert.Null(ScreenScraperParser.ParseDate("cedo"));
    }

    [Theory]
    [InlineData(200, "{\"header\":{}}", null, false)]
    [InlineData(404, "Erreur : Rom/Iso/Dossier non trouvée !", null, true)]
    [InlineData(200, "Erreur : Rom/Iso/Dossier non trouvée !", null, true)]
    [InlineData(430, "Votre quota de scrape est dépassé pour aujourd'hui !", ScreenScraperFailure.QuotaExceeded, false)]
    [InlineData(431, "Trop de roms non trouvées", ScreenScraperFailure.QuotaExceeded, false)]
    [InlineData(429, "Le nombre de threads autorisés est atteint", ScreenScraperFailure.ThreadLimit, false)]
    [InlineData(401, "API fermé pour les non membres", ScreenScraperFailure.Closed, false)]
    [InlineData(423, "API totalement fermé", ScreenScraperFailure.Closed, false)]
    [InlineData(426, "Le logiciel de scrape utilisé a été blacklisté", ScreenScraperFailure.Blacklisted, false)]
    [InlineData(403, "Erreur de login : Vérifier vos identifiants développeur !", ScreenScraperFailure.BadSoftware, false)]
    [InlineData(200, "Erreur de login : Vérifier vos identifiants utilisateur !", ScreenScraperFailure.BadUser, false)]
    [InlineData(500, "erro", ScreenScraperFailure.Unavailable, false)]
    public void Responses_are_classified_by_code_and_french_text(int status, string body, ScreenScraperFailure? expected, bool notFound)
    {
        Assert.Equal(expected, ScreenScraperClient.Classify((HttpStatusCode)status, body, out bool missing));
        Assert.Equal(notFound, missing);
    }

    [Fact]
    public void Error_messages_are_clear_and_distinct()
    {
        var messages = Enum.GetValues<ScreenScraperFailure>().Select(ScreenScraperErrors.Message).ToList();
        Assert.Equal(messages.Count, messages.Distinct().Count());
        Assert.Contains("ainda não tem acesso ao ScreenScraper", ScreenScraperErrors.Message(ScreenScraperFailure.NoSoftwareAccess), StringComparison.Ordinal);
    }

    [Fact]
    public void Media_urls_drop_credentials_and_only_go_back_to_screenscraper()
    {
        var stored = ScreenScraperMedia.FromResponse("https://neoclone.screenscraper.fr/api2/mediaJeu.php?devid=D&devpassword=P&softname=S&ssid=U&sspassword=X&systemeid=4&jeuid=1&media=box-2D(us)")!;
        Assert.Equal("ssmedia://neoclone.screenscraper.fr/api2/mediaJeu.php?systemeid=4&jeuid=1&media=box-2D(us)", stored.ToString());

        var download = ScreenScraperMedia.ToDownload(stored, Software, Account)!;
        Assert.Equal(Uri.UriSchemeHttps, download.Scheme);
        Assert.Contains("devid=dev&devpassword=devpass&softname=Tako&ssid=jogador&sspassword=senha%20secreta&systemeid=4", download.AbsoluteUri, StringComparison.Ordinal);

        Assert.Null(ScreenScraperMedia.FromResponse("https://exemplo.invalid/a.png"));
        Assert.Null(ScreenScraperMedia.FromResponse("https://screenscraper.fr.exemplo.invalid/a.png"));
        Assert.Null(ScreenScraperMedia.ToDownload(new Uri("ssmedia://exemplo.invalid/a.png"), Software, Account));
        Assert.Null(ScreenScraperMedia.ToDownload(new Uri("https://neoclone.screenscraper.fr/a.png"), Software, Account));
    }

    // ---------------------------------------------------------------- Credenciais

    [Fact]
    public void Software_credentials_come_only_from_the_release_build()
    {
        // Os testes não são build de release: o assembly não tem os atributos.
        Assert.False(ScreenScraperSoftware.FromAssembly(typeof(ScreenScraperSoftware).Assembly).IsAvailable);
        Assert.False(ScreenScraperSoftware.FromAssembly(Assembly.GetExecutingAssembly()).IsAvailable);
        Assert.Equal(ScreenScraperSoftware.Missing, ScreenScraperSoftware.FromAssembly(null));
        var release = ScreenScraperSoftware.FromValues(" id ", "senha", null);
        Assert.True(release.IsAvailable);
        Assert.Equal("id", release.DevId);
        Assert.Equal("Tako", release.SoftName);
        Assert.DoesNotContain("senha", release.ToString(), StringComparison.Ordinal);
        Assert.DoesNotContain("senha secreta", Account.ToString(), StringComparison.Ordinal);
    }

    [Fact]
    public void Account_store_encrypts_the_whole_file()
    {
        string file = Path.Combine(_root, "screenscraper.dat");
        var store = new ScreenScraperSettingsStore(file, new ReversingProtector());
        Assert.Equal(ScreenScraperSettings.Off, store.Current);

        store.Save(Account with { User = " jogador " });

        Assert.DoesNotContain("jogador", File.ReadAllText(file), StringComparison.Ordinal);
        var reopened = new ScreenScraperSettingsStore(file, new ReversingProtector()).Current;
        Assert.Equal(Account, reopened);
        Assert.True(reopened.IsActive);
        Assert.Null(reopened.Problem());
        Assert.Contains("usuário", (Account with { User = "" }).Problem(), StringComparison.Ordinal);
        Assert.Contains("senha", (Account with { Password = "" }).Problem(), StringComparison.Ordinal);

        File.WriteAllText(file, "lixo");
        Assert.Equal(ScreenScraperSettings.Off, new ScreenScraperSettingsStore(file, new ReversingProtector()).Current);
    }

    // ---------------------------------------------------------------- Cliente

    [Fact]
    public async Task Without_software_credentials_nothing_goes_to_the_network()
    {
        var http = new FakeHttp();
        using var client = new ScreenScraperClient(http.Client, ScreenScraperSoftware.Missing, () => Account, TimeProvider.System);

        Assert.False(client.HasSoftwareAccess);
        Assert.False(client.IsConfigured);
        var error = await Assert.ThrowsAsync<ScreenScraperException>(() => client.GetAsync("jeuInfos.php", [], TestContext.Current.CancellationToken));
        Assert.Equal(ScreenScraperFailure.NoSoftwareAccess, error.Failure);
        var test = await Assert.ThrowsAsync<ScreenScraperException>(() => ScreenScraperClient.TestAsync(http.Client, ScreenScraperSoftware.Missing, Account, TimeProvider.System, TestContext.Current.CancellationToken));
        Assert.Equal(ScreenScraperFailure.NoSoftwareAccess, test.Failure);
        Assert.Empty(http.Requests);

        var provider = new ScreenScraperProvider(client, new RomHashCache(), new FixedTimeProvider(Now));
        Assert.False(provider.IsEnabled);
    }

    [Fact]
    public async Task Test_reports_level_and_daily_quota()
    {
        var http = new FakeHttp();
        http.Responses.Enqueue((HttpStatusCode.OK, Fixture("ssuserInfos.json")));

        var account = await ScreenScraperClient.TestAsync(http.Client, Software, Account with { Enabled = false }, TimeProvider.System, TestContext.Current.CancellationToken);

        Assert.Equal(1, account.Level);
        Assert.Equal(19980, account.RemainingToday);
        string sent = Assert.Single(http.Requests);
        Assert.StartsWith("https://api.screenscraper.fr/api2/ssuserInfos.php?devid=dev&devpassword=devpass&softname=Tako&output=json&ssid=jogador&sspassword=senha%20secreta", sent, StringComparison.Ordinal);

        http.Responses.Enqueue((HttpStatusCode.OK, "Erreur de login : Vérifier vos identifiants utilisateur !"));
        var bad = await Assert.ThrowsAsync<ScreenScraperException>(() => ScreenScraperClient.TestAsync(http.Client, Software, Account, TimeProvider.System, TestContext.Current.CancellationToken));
        Assert.Equal(ScreenScraperFailure.BadUser, bad.Failure);
    }

    [Fact]
    public async Task Client_retries_thread_limit_caches_without_credentials_and_learns_maxthreads()
    {
        var http = new FakeHttp();
        http.Responses.Enqueue(((HttpStatusCode)429, "Le nombre de threads autorisés est atteint"));
        http.Responses.Enqueue((HttpStatusCode.OK, Fixture("jeuInfos-lanterna.json")));
        var cache = new MemoryCache();
        var delays = new List<TimeSpan>();
        using var client = Client(http, cache, delays, () => Account, TimeProvider.System);

        var first = await client.GetAsync("jeuInfos.php", [new("gameid", "900077")], TestContext.Current.CancellationToken);
        var again = await client.GetAsync("jeuInfos.php", [new("gameid", "900077")], TestContext.Current.CancellationToken);

        Assert.NotNull(first);
        Assert.NotNull(again);
        Assert.Equal(2, client.NetworkRequests); // 429 + sucesso; a segunda veio do cache
        Assert.Contains(TimeSpan.FromSeconds(1), delays);
        Assert.Equal(2, client.Account!.MaxThreads);
        Assert.Equal(19880, client.Account.RemainingToday);
        string key = Assert.Single(cache.Keys);
        Assert.Equal("jeuInfos.php?gameid=900077", key);
        Assert.False(client.IsPaused);
    }

    [Fact]
    public async Task Daily_quota_pauses_until_the_reset_or_an_account_change()
    {
        var http = new FakeHttp();
        http.Responses.Enqueue(((HttpStatusCode)430, "Votre quota de scrape est dépassé pour aujourd'hui !"));
        var settings = Account;
        var time = new FixedTimeProvider(Now);
        using var client = Client(http, null, [], () => settings, time);

        var error = await Assert.ThrowsAsync<ScreenScraperException>(() => client.GetAsync("jeuInfos.php", [new("gameid", "1")], TestContext.Current.CancellationToken));
        Assert.Equal(ScreenScraperFailure.QuotaExceeded, error.Failure);
        Assert.True(client.IsPaused);
        Assert.Equal(ScreenScraperFailure.QuotaExceeded, client.PauseReason);

        // Pausado: nem vai à rede.
        await Assert.ThrowsAsync<ScreenScraperException>(() => client.GetAsync("jeuInfos.php", [new("gameid", "2")], TestContext.Current.CancellationToken));
        Assert.Single(http.Requests);

        // Outra conta: tenta de novo.
        settings = Account with { User = "outra" };
        Assert.False(client.IsPaused);
    }

    [Fact]
    public void Quota_resets_after_midnight_in_paris()
    {
        var reset = ScreenScraperClient.NextQuotaReset(Now);
        Assert.True(reset > Now);
        Assert.True(reset - Now <= TimeSpan.FromHours(25));
        Assert.Equal(5, reset.Minute);
    }

    // ---------------------------------------------------------------- Provedor e combinação

    [Fact]
    public async Task Provider_matches_the_rom_by_hash_and_reuses_the_answer()
    {
        var http = new FakeHttp();
        http.Responses.Enqueue((HttpStatusCode.OK, Fixture("jeuInfos-lanterna.json")));
        using var client = Client(http, null, [], () => Account, TimeProvider.System);
        var provider = new ScreenScraperProvider(client, new RomHashCache(), new FixedTimeProvider(Now));
        string rom = WriteRom("Lanterna Distante (USA).sfc", "123456789");
        var lookup = new MetadataLookup("Lanterna Distante", new Dictionary<string, string>(), "snes") { RomPath = rom };

        string? id = await provider.FindExactAsync(lookup, TestContext.Current.CancellationToken);
        var metadata = await provider.FetchAsync(id!, TestContext.Current.CancellationToken);

        Assert.Equal("900077", id);
        Assert.Equal("Lanterna Distante", metadata!.Title);
        string sent = Assert.Single(http.Requests);
        Assert.Contains("/api2/jeuInfos.php?", sent, StringComparison.Ordinal);
        Assert.Contains("&systemeid=4&romtype=rom&romnom=Lanterna%20Distante%20%28USA%29.sfc&romtaille=9&crc=CBF43926&md5=25F9E794323B453885F5181F1B624D0B&sha1=F7C3BC1D808E04732ADF679965CCC34CA7AE3441", sent, StringComparison.Ordinal);

        // A mesma ROM não é consultada de novo nesta execução.
        Assert.Equal("900077", await provider.FindExactAsync(lookup, TestContext.Current.CancellationToken));
        Assert.Single(http.Requests);

        // Jogo de PC ou sem arquivo: o ScreenScraper não é consultado.
        Assert.Null(await provider.FindExactAsync(lookup with { Platform = null }, TestContext.Current.CancellationToken));
        Assert.Null(await provider.FindExactAsync(lookup with { RomPath = Path.Combine(_root, "sumiu.sfc") }, TestContext.Current.CancellationToken));
        Assert.Empty(await provider.SearchAsync(lookup with { Platform = "pc" }, TestContext.Current.CancellationToken));
        Assert.Single(http.Requests);
    }

    [Fact]
    public void Rom_query_marks_disc_images_as_iso()
    {
        var query = ScreenScraperProvider.RomQuery(new RomIdentity("Lanterna.chd", 700), ScreenScraperSystems.ForKey("ps1")!);
        Assert.Equal(
            ["systemeid=57", "romtype=iso", "romnom=Lanterna.chd", "romtaille=700"],
            query.Select(kv => kv.Key + "=" + kv.Value));
    }

    [Fact]
    public async Task Search_lists_the_hash_match_first_then_name_results()
    {
        var http = new FakeHttp();
        http.Responses.Enqueue((HttpStatusCode.NotFound, "Erreur : Rom/Iso/Dossier non trouvée !"));
        http.Responses.Enqueue((HttpStatusCode.OK, Fixture("jeuRecherche-lanterna.json")));
        using var client = Client(http, null, [], () => Account, TimeProvider.System);
        var provider = new ScreenScraperProvider(client, new RomHashCache(), new FixedTimeProvider(Now));
        var lookup = new MetadataLookup("Lanterna", new Dictionary<string, string>(), "snes") { RomPath = WriteRom("lanterna.sfc", "x") };

        var results = await provider.SearchAsync(lookup, TestContext.Current.CancellationToken);

        Assert.Equal(["900077", "900078"], results.Select(r => r.Id));
        Assert.All(results, r => Assert.False(r.IsExact));
        Assert.Contains("jeuRecherche.php?", http.Requests[1], StringComparison.Ordinal);
        Assert.Contains("&systemeid=4&recherche=Lanterna", http.Requests[1], StringComparison.Ordinal);
        Assert.False(provider.IsPaused); // "não encontrada" não pausa
    }

    [Fact]
    public async Task Roms_use_screenscraper_first_and_igdb_fills_the_gaps()
    {
        var http = new FakeHttp();
        http.Responses.Enqueue((HttpStatusCode.OK, Fixture("jeuInfos-lanterna.json")));
        using var client = Client(http, null, [], () => Account, TimeProvider.System);
        var screenScraper = new ScreenScraperProvider(client, new RomHashCache(), new FixedTimeProvider(Now));
        var igdb = new StubProvider(MetadataPriority.Igdb, new GameMetadata
        {
            Source = MetadataPriority.Igdb,
            SourceIds = new Dictionary<string, string> { [MetadataPriority.Igdb] = "42" },
            Title = "Distant Lantern",
            Description = "English summary",
            DescriptionLanguage = "en",
            Themes = ["Fantasia"],
            Franchise = "Lanterna",
            CoverUrl = new Uri("https://images.igdb.com/capa.jpg"),
        });
        var agent = new MetadataAgent([igdb, screenScraper], new FixedTimeProvider(Now));
        var game = ManualGameRecord.CreateRom(WriteRom("Lanterna Distante (USA).sfc", "123456789"), "snes", Now).ToGame();

        var update = await agent.UpdateAsync(game, null, force: false, TestContext.Current.CancellationToken);
        var merged = update.Metadata!;
        merged.ApplyTo(game);

        Assert.Equal("ScreenScraper + IGDB", merged.Source);
        Assert.Equal(["Lanterna Distante (USA).sfc"], [Path.GetFileName(igdb.LastLookup!.RomPath!)]);
        Assert.Equal("snes", igdb.LastLookup.Platform); // a IGDB filtra pela plataforma
        Assert.StartsWith("Um faroleiro", game.Description, StringComparison.Ordinal);
        Assert.Equal(["Fantasia"], game.Themes); // só a IGDB tem
        Assert.Equal("Lanterna", game.Franchise);
        Assert.Equal(ScreenScraperMedia.Scheme, game.CoverUri!.Scheme); // caixa do ScreenScraper, não a capa da IGDB
        Assert.Equal("1-2", game.Players);
        Assert.True(game.Media.ContainsKey(GameMedia.Logo));
        Assert.False(agent.NeedsUpdate(game, merged));
    }

    [Fact]
    public async Task Pc_games_never_spend_the_screenscraper_quota()
    {
        var http = new FakeHttp();
        using var client = Client(http, null, [], () => Account, TimeProvider.System);
        var screenScraper = new ScreenScraperProvider(client, new RomHashCache(), new FixedTimeProvider(Now));
        var steam = new StubProvider(SteamStoreProvider.SourceName, null);
        var agent = new MetadataAgent([steam, screenScraper], new FixedTimeProvider(Now));
        var game = new Game { Id = "epic:1", Title = "Lanterna", Store = GameStore.Epic, Accent = "#000000" };

        var update = await agent.UpdateAsync(game, null, force: false, TestContext.Current.CancellationToken);

        Assert.Equal(["Steam"], update.Metadata!.Consulted);
        Assert.False(agent.NeedsUpdate(game, update.Metadata));
        Assert.Empty(await agent.SearchAsync(game, TestContext.Current.CancellationToken));
        Assert.Empty(http.Requests);
    }

    [Fact]
    public async Task Exhausted_quota_keeps_roms_pending_and_their_old_data()
    {
        var http = new FakeHttp();
        http.Responses.Enqueue(((HttpStatusCode)430, "Votre quota de scrape est dépassé pour aujourd'hui !"));
        using var client = Client(http, null, [], () => Account, new FixedTimeProvider(Now));
        var screenScraper = new ScreenScraperProvider(client, new RomHashCache(), new FixedTimeProvider(Now));
        var igdb = new StubProvider(MetadataPriority.Igdb, null);
        var agent = new MetadataAgent([igdb, screenScraper], new FixedTimeProvider(Now));
        var game = ManualGameRecord.CreateRom(WriteRom("lanterna.sfc", "123456789"), "snes", Now).ToGame();

        var update = await agent.UpdateAsync(game, null, force: false, TestContext.Current.CancellationToken);

        Assert.True(screenScraper.IsPaused);
        Assert.Equal([MetadataPriority.Igdb], update.Metadata!.Consulted); // não marcou "sem correspondência" no ScreenScraper
        Assert.False(agent.NeedsUpdate(game, update.Metadata)); // pausado: espera, sem girar em falso

        // Dados antigos do ScreenScraper continuam na combinação enquanto ele está pausado.
        var old = new GameMetadata { Source = MetadataPriority.ScreenScraper, SourceIds = new Dictionary<string, string> { [MetadataPriority.ScreenScraper] = "900077" }, Title = "Lanterna Distante", Players = "1-2" };
        var cached = MetadataMerge.Combine([old], MetadataKind.Emulated, Now, [MetadataPriority.ScreenScraper]);
        var later = await agent.UpdateAsync(game, cached, force: false, TestContext.Current.CancellationToken);
        Assert.Contains(later.Metadata!.Parts, p => p.Source == MetadataPriority.ScreenScraper);
        Assert.Equal("1-2", later.Metadata.Players);
        Assert.Single(http.Requests);
    }

    // ---------------------------------------------------------------- Apoio

    private string WriteRom(string name, string content)
    {
        string path = Path.Combine(_root, name);
        File.WriteAllText(path, content);
        return path;
    }

    private static ScreenScraperClient Client(FakeHttp http, IIgdbResponseCache? cache, List<TimeSpan> delays, Func<ScreenScraperSettings> settings, TimeProvider time) =>
        new(http.Client, Software, settings, time, cache, (wait, _) =>
        {
            delays.Add(wait);
            return Task.CompletedTask;
        });

    private sealed class StubProvider(string name, GameMetadata? result) : IMetadataProvider
    {
        public string Name => name;

        public bool IsEnabled => true;

        public MetadataLookup? LastLookup { get; private set; }

        public Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken)
        {
            LastLookup = lookup;
            return Task.FromResult<IReadOnlyList<MetadataMatch>>(result is null ? [] : [new MetadataMatch(name, result.IdIn(name)!, result.Title!) { IsExact = true }]);
        }

        public Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken)
        {
            LastLookup = lookup;
            return Task.FromResult<string?>(null);
        }

        public Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken) => Task.FromResult(result);
    }

    private sealed class MemoryCache : IIgdbResponseCache
    {
        private readonly Dictionary<string, string> _items = [];

        public IEnumerable<string> Keys => _items.Keys;

        public string? TryRead(string key) => _items.GetValueOrDefault(key);

        public void Write(string key, string json) => _items[key] = json;
    }

    private sealed class ReversingProtector : ISecretProtector
    {
        public string Protect(string secret) => "rev:" + new string([.. secret.Reverse()]);

        public bool TryUnprotect(string stored, out string secret)
        {
            secret = stored.StartsWith("rev:", StringComparison.Ordinal) ? new string([.. stored[4..].Reverse()]) : string.Empty;
            return secret.Length > 0;
        }
    }

    /// <summary>ScreenScraper de mentira: respostas em fila; guarda os endereços pedidos.</summary>
    private sealed class FakeHttp : HttpMessageHandler
    {
        public FakeHttp() => Client = new HttpClient(this);

        public HttpClient Client { get; }

        public Queue<(HttpStatusCode Status, string Body)> Responses { get; } = new();

        public List<string> Requests { get; } = [];

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            Requests.Add(request.RequestUri!.AbsoluteUri);
            var (status, body) = Responses.Count > 0 ? Responses.Dequeue() : (HttpStatusCode.NotFound, "Erreur : Rom/Iso/Dossier non trouvée !");
            return Task.FromResult(new HttpResponseMessage(status) { Content = new StringContent(body, Encoding.UTF8, "application/json") });
        }
    }
}
