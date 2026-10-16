using System.Globalization;
using System.Net;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Launcher.Core.Library.Metadata;

/// <summary>
/// Provedor de metadados pela API pública da loja Steam (sem chave): busca por título e detalhes em português (quando o jogo tem) —
/// descrição, gêneros, lançamento, desenvolvedora, publicadora e nota do Metacritic. A Steam limita ~200 consultas
/// a cada 5 minutos; quem chama espaça as consultas e guarda o resultado em cache.
/// </summary>
public sealed partial class SteamStoreProvider(HttpClient http) : IMetadataProvider
{
    public const string SourceName = "Steam";

    private static readonly string[] Months = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

    public string Name => SourceName;

    public bool IsEnabled => true;

    public async Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (lookup.Platform is { Length: > 0 } platform && !string.Equals(platform, "pc", StringComparison.OrdinalIgnoreCase))
        {
            return []; // a loja Steam só tem jogos de PC
        }

        var url = new Uri($"https://store.steampowered.com/api/storesearch/?term={Uri.EscapeDataString(lookup.Title)}&l=brazilian&cc=BR");
        using var document = await GetJsonAsync(url, cancellationToken).ConfigureAwait(false);
        if (document?.RootElement.TryGetProperty("items", out var items) != true)
        {
            return [];
        }

        return [.. items.EnumerateArray()
            .Where(i => i.TryGetProperty("type", out var type) && type.GetString() == "app")
            .Select(i => MetadataMatch.Steam(i.GetProperty("name").GetString() ?? string.Empty, i.GetProperty("id").GetInt32()) with
            {
                CoverUrl = i.TryGetProperty("tiny_image", out var tiny) && Uri.TryCreate(tiny.GetString(), UriKind.Absolute, out var image) ? image : null,
            })];
    }

    /// <summary>O appid já é o id da Steam: jogos da Steam (ou que outro provedor ligou a um appid) casam direto.</summary>
    public Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        return Task.FromResult(lookup.ExternalIds.GetValueOrDefault(ExternalGameIds.Steam));
    }

    public async Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken)
    {
        if (!int.TryParse(id, NumberStyles.None, CultureInfo.InvariantCulture, out int steamAppId))
        {
            return null;
        }

        var url = new Uri($"https://store.steampowered.com/api/appdetails?appids={steamAppId}&l=brazilian&cc=BR");
        using var document = await GetJsonAsync(url, cancellationToken).ConfigureAwait(false);
        return document is null ? null : Parse(document.RootElement, steamAppId, DateTimeOffset.UtcNow);
    }

    /// <summary>Resposta de <c>appdetails</c> → metadados (público para teste).</summary>
    public static GameMetadata? Parse(JsonElement root, int steamAppId, DateTimeOffset now)
    {
        if (!root.TryGetProperty(steamAppId.ToString(CultureInfo.InvariantCulture), out var entry)
            || !entry.TryGetProperty("success", out var success) || !success.GetBoolean()
            || !entry.TryGetProperty("data", out var data))
        {
            return null;
        }

        return new GameMetadata
        {
            Source = SourceName,
            FetchedAt = now,
            SteamAppId = steamAppId,
            SourceIds = new Dictionary<string, string> { [SourceName] = steamAppId.ToString(CultureInfo.InvariantCulture) },
            Title = Text(data, "name"),
            Description = Text(data, "short_description") is { } d ? WebUtility.HtmlDecode(d).Trim() : null,
            // Pedido em português; a loja só traduz a descrição de jogos com português entre os idiomas.
            DescriptionLanguage = Text(data, "supported_languages")?.Contains("Português", StringComparison.OrdinalIgnoreCase) == true ? "pt-BR" : "en",
            Genres = data.TryGetProperty("genres", out var genres) ? [.. genres.EnumerateArray().Select(g => Text(g, "description")).OfType<string>().Take(3)] : [],
            ReleaseDate = data.TryGetProperty("release_date", out var release) ? ParseDate(Text(release, "date")) : null,
            Developer = First(data, "developers"),
            Publisher = First(data, "publishers"),
            MetacriticScore = data.TryGetProperty("metacritic", out var meta) && meta.TryGetProperty("score", out var score) ? score.GetInt32() : null,
        };
    }

    /// <summary>Datas da loja: "4/set./2025" (português) ou "4 Sep, 2025" / "Sep 4, 2025" (inglês).</summary>
    public static DateOnly? ParseDate(string? text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return null;
        }

        var match = PortugueseDate().Match(text);
        if (match.Success)
        {
            int month = Array.IndexOf(Months, match.Groups["month"].Value.ToLowerInvariant()) + 1;
            if (month > 0)
            {
                return new DateOnly(int.Parse(match.Groups["year"].Value, CultureInfo.InvariantCulture), month, int.Parse(match.Groups["day"].Value, CultureInfo.InvariantCulture));
            }
        }

        string[] formats = ["d MMM, yyyy", "MMM d, yyyy", "d MMMM, yyyy", "MMMM d, yyyy"];
        return DateTime.TryParseExact(text.Trim(), formats, CultureInfo.InvariantCulture, DateTimeStyles.None, out var date) ? DateOnly.FromDateTime(date) : null;
    }

    private async Task<JsonDocument?> GetJsonAsync(Uri url, CancellationToken cancellationToken)
    {
        try
        {
            using var response = await http.GetAsync(url, cancellationToken).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode)
            {
                return null; // 429 (limite) ou indisponível: tenta de novo numa próxima atualização
            }

            await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken).ConfigureAwait(false);
            return await JsonDocument.ParseAsync(stream, cancellationToken: cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is HttpRequestException or JsonException or TaskCanceledException && !cancellationToken.IsCancellationRequested)
        {
            return null;
        }
    }

    private static string? Text(JsonElement element, string property) =>
        element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;

    private static string? First(JsonElement element, string property) =>
        element.TryGetProperty(property, out var list) && list.ValueKind == JsonValueKind.Array && list.GetArrayLength() > 0 ? list[0].GetString() : null;

    [GeneratedRegex(@"^\s*(?<day>\d{1,2})\s*(?:/|de)\s*(?<month>[A-Za-zç]{3})[a-zç]*\.?\s*(?:/|de)\s*(?<year>\d{4})", RegexOptions.IgnoreCase)]
    private static partial Regex PortugueseDate();
}
