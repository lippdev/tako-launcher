using System.Globalization;
using System.Text.Json;
using Launcher.Core.Library.Metadata.Igdb;

namespace Launcher.Core.Library.Metadata.ScreenScraper;

/// <summary>Respostas JSON da API v2 do ScreenScraper → metadados do Tako (puro, testado com respostas escritas à mão).</summary>
public static class ScreenScraperParser
{
    /// <summary>Regiões do nome, na ordem de preferência (ADR-0022): Brasil, Portugal, EUA, mundo, Europa, ScreenScraper, Japão.</summary>
    public static readonly IReadOnlyList<string> NameRegions = ["br", "pt", "us", "wor", "eu", "ss", "jp"];

    /// <summary>Regiões das mídias e da data de lançamento: Brasil, EUA, mundo, Europa, ScreenScraper, Japão.</summary>
    public static readonly IReadOnlyList<string> MediaRegions = ["br", "us", "wor", "eu", "ss", "jp"];

    /// <summary>Idiomas da sinopse e dos gêneros: português, depois inglês.</summary>
    public static readonly IReadOnlyList<string> Languages = ["pt", "en"];

    // Tipo de mídia do ScreenScraper → chave em GameMetadata (os primeiros têm preferência).
    private static readonly (string Kind, string[] Types)[] MediaTypes =
    [
        (GameMedia.Box2D, ["box-2D"]),
        (GameMedia.Box3D, ["box-3D"]),
        (GameMedia.Logo, ["wheel-hd", "wheel"]),
        (GameMedia.Marquee, ["screenmarquee", "marquee", "screenmarqueesmall"]),
        (GameMedia.Screenshot, ["ss", "sstitle"]),
        (GameMedia.Fanart, ["fanart"]),
        (GameMedia.Video, ["video-normalized", "video"]),
    ];

    /// <summary>
    /// <c>response</c> de uma resposta completa (<c>{"header":…,"response":…}</c>); nulo se não for JSON ou o cabeçalho
    /// disser que falhou.
    /// </summary>
    public static JsonElement? Envelope(string json)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty("response", out var response) || response.ValueKind != JsonValueKind.Object)
            {
                return null;
            }

            if (root.TryGetProperty("header", out var header) && Text(header, "success") is { } success && !string.Equals(success, "true", StringComparison.OrdinalIgnoreCase))
            {
                return null;
            }

            return response.Clone();
        }
        catch (JsonException)
        {
            return null;
        }
    }

    /// <summary>Conta do usuário (<c>ssuser</c>), presente em toda resposta autenticada.</summary>
    public static ScreenScraperAccount? Account(JsonElement response)
    {
        if (!response.TryGetProperty("ssuser", out var user) || user.ValueKind != JsonValueKind.Object || Text(user, "id") is not { Length: > 0 } id)
        {
            return null;
        }

        return new ScreenScraperAccount(id, Int(user, "niveau") ?? 0, Int(user, "maxthreads") ?? 1, Int(user, "requeststoday") ?? 0, Int(user, "maxrequestsperday") ?? 0)
        {
            KoToday = Int(user, "requestskotoday") ?? 0,
            MaxKoPerDay = Int(user, "maxrequestskoperday") ?? 0,
        };
    }

    /// <summary>O jogo de um <c>jeuInfos.php</c> (<c>response.jeu</c>).</summary>
    public static GameMetadata? GameInfo(JsonElement response, DateTimeOffset now) =>
        response.TryGetProperty("jeu", out var jeu) ? Game(jeu, now) : null;

    /// <summary>Resultados de um <c>jeuRecherche.php</c> (<c>response.jeux</c>). Sem resultados, a API devolve <c>[{}]</c>.</summary>
    public static IReadOnlyList<MetadataMatch> Search(JsonElement response) =>
        [.. Array(response, "jeux").Select(Match).OfType<MetadataMatch>()];

    /// <summary>Um jogo (<c>jeu</c>) → metadados do Tako.</summary>
    public static GameMetadata? Game(JsonElement jeu, DateTimeOffset now)
    {
        if (jeu.ValueKind != JsonValueKind.Object || Text(jeu, "id") is not { Length: > 0 } id || Title(jeu) is not { } title)
        {
            return null;
        }

        var synopsis = ByLanguage(Array(jeu, "synopsis"));
        var media = Media(jeu);
        var score = jeu.TryGetProperty("note", out var note) ? Int(note, "text") : null;
        return new GameMetadata
        {
            Source = MetadataPriority.ScreenScraper,
            FetchedAt = now,
            SourceIds = new Dictionary<string, string>(StringComparer.Ordinal) { [MetadataPriority.ScreenScraper] = id },
            Title = title,
            Description = synopsis?.Text,
            DescriptionLanguage = synopsis is null ? null : synopsis.Value.Language == "pt" ? "pt-BR" : synopsis.Value.Language,
            Genres = Genres(jeu).Take(3).ToList(),
            ReleaseDate = ReleaseDate(jeu),
            Developer = jeu.TryGetProperty("developpeur", out var dev) ? Text(dev, "text") : null,
            Publisher = jeu.TryGetProperty("editeur", out var pub) ? Text(pub, "text") : null,
            // Nota de 0 a 20 no ScreenScraper.
            CriticScore = score is int s and >= 0 and <= 20 ? s * 5 : null,
            Players = jeu.TryGetProperty("joueurs", out var players) ? Text(players, "text") : null,
            CoverUrl = media.GetValueOrDefault(GameMedia.Box2D),
            // Fundo: fanart; sem ela, a captura de tela.
            BackgroundUrl = media.GetValueOrDefault(GameMedia.Fanart) ?? media.GetValueOrDefault(GameMedia.Screenshot),
            Media = media.Where(kv => kv.Key != GameMedia.Box2D).ToDictionary(kv => kv.Key, kv => kv.Value, StringComparer.Ordinal),
        };
    }

    /// <summary>Resultado para "Corrigir correspondência": nome, ano e sistema.</summary>
    public static MetadataMatch? Match(JsonElement jeu)
    {
        if (jeu.ValueKind != JsonValueKind.Object || Text(jeu, "id") is not { Length: > 0 } id || Title(jeu) is not { } title)
        {
            return null;
        }

        string? platform = null;
        if (jeu.TryGetProperty("systeme", out var system))
        {
            platform = Int(system, "id") is int sid && ScreenScraperSystems.ForScreenScraperId(sid) is { } known ? known.Name : Text(system, "text");
        }

        return new MetadataMatch(MetadataPriority.ScreenScraper, id, title)
        {
            Year = ReleaseDate(jeu)?.Year ?? Year(jeu),
            Platforms = platform,
        };
    }

    /// <summary>Nome pela região preferida (<see cref="NameRegions"/>); senão, o primeiro.</summary>
    public static string? Title(JsonElement jeu)
    {
        var names = Array(jeu, "noms").Select(n => (Region: Text(n, "region"), Text: Text(n, "text"))).Where(n => n.Text is { Length: > 0 }).ToList();
        if (names.Count == 0)
        {
            return Text(jeu, "nom");
        }

        foreach (string region in NameRegions)
        {
            if (names.FirstOrDefault(n => string.Equals(n.Region, region, StringComparison.OrdinalIgnoreCase)) is { Text: { } text })
            {
                return text;
            }
        }

        return names[0].Text;
    }

    /// <summary>Mídias por tipo, cada uma na região preferida (<see cref="MediaRegions"/>), sem credenciais na URL.</summary>
    public static IReadOnlyDictionary<string, Uri> Media(JsonElement jeu)
    {
        var items = Array(jeu, "medias")
            .Select(m => (Type: Text(m, "type"), Region: Text(m, "region"), Url: ScreenScraperMedia.FromResponse(Text(m, "url"))))
            .Where(m => m.Type is not null && m.Url is not null)
            .ToList();
        var result = new Dictionary<string, Uri>(StringComparer.Ordinal);
        foreach (var (kind, types) in MediaTypes)
        {
            foreach (string type in types)
            {
                var ofType = items.Where(m => string.Equals(m.Type, type, StringComparison.OrdinalIgnoreCase)).ToList();
                if (ofType.Count == 0)
                {
                    continue;
                }

                var best = MediaRegions.Select(r => ofType.FirstOrDefault(m => string.Equals(m.Region, r, StringComparison.OrdinalIgnoreCase))).FirstOrDefault(m => m.Url is not null);
                result[kind] = (best.Url ?? ofType[0].Url)!;
                break;
            }
        }

        return result;
    }

    private static DateOnly? ReleaseDate(JsonElement jeu)
    {
        var dates = Array(jeu, "dates").Select(d => (Region: Text(d, "region"), Date: ParseDate(Text(d, "text")))).Where(d => d.Date is not null).ToList();
        foreach (string region in MediaRegions)
        {
            if (dates.FirstOrDefault(d => string.Equals(d.Region, region, StringComparison.OrdinalIgnoreCase)) is { Date: { } date })
            {
                return date;
            }
        }

        return dates.Count > 0 ? dates.Min(d => d.Date) : null;
    }

    private static int? Year(JsonElement jeu) =>
        Array(jeu, "dates").Select(d => Text(d, "text")).Where(t => t is { Length: >= 4 }).Select(t => int.TryParse(t![..4], NumberStyles.None, CultureInfo.InvariantCulture, out int y) ? y : (int?)null).FirstOrDefault(y => y is not null);

    /// <summary>"1991-11-21", "1991-11" ou "1991".</summary>
    public static DateOnly? ParseDate(string? text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return null;
        }

        string[] formats = ["yyyy-MM-dd", "yyyy-MM", "yyyy"];
        return DateOnly.TryParseExact(text.Trim(), formats, CultureInfo.InvariantCulture, DateTimeStyles.None, out var date) ? date : null;
    }

    private static IEnumerable<string> Genres(JsonElement jeu)
    {
        foreach (var genre in Array(jeu, "genres"))
        {
            if (ByLanguage(Array(genre, "noms")) is { } name)
            {
                yield return name.Language == "pt" ? name.Text : IgdbText.Translate(name.Text);
            }
        }
    }

    /// <summary>Texto em português; senão, em inglês (<see cref="Languages"/>).</summary>
    private static (string Language, string Text)? ByLanguage(IEnumerable<JsonElement> items)
    {
        var texts = items.Select(i => (Language: Text(i, "langue"), Text: Text(i, "text")?.Trim())).Where(i => i.Text is { Length: > 0 }).ToList();
        foreach (string language in Languages)
        {
            if (texts.FirstOrDefault(t => string.Equals(t.Language, language, StringComparison.OrdinalIgnoreCase)) is { Text: { } text })
            {
                return (language, text);
            }
        }

        return null;
    }

    private static JsonElement[] Array(JsonElement element, string property) =>
        element.ValueKind == JsonValueKind.Object && element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.Array ? [.. value.EnumerateArray()] : [];

    // O ScreenScraper manda números como texto ("id": "3"); aceita os dois.
    private static string? Text(JsonElement element, string property) =>
        element.ValueKind == JsonValueKind.Object && element.TryGetProperty(property, out var value)
            ? value.ValueKind switch
            {
                JsonValueKind.String => value.GetString(),
                JsonValueKind.Number => value.GetRawText(),
                _ => null,
            }
            : null;

    private static int? Int(JsonElement element, string property) =>
        Text(element, property) is { } text && int.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out int value) ? value : null;
}
