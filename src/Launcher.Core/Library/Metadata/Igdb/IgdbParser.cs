using System.Globalization;
using System.Text.Json;

namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>Respostas da IGDB → metadados do Tako (puro, testado com respostas de exemplo).</summary>
public static class IgdbParser
{
    public const string CoverSize = "t_cover_big";
    public const string ArtworkSize = "t_1080p";
    public const string ScreenshotSize = "t_screenshot_huge";

    /// <summary>Um jogo do endpoint <c>games</c> (com os campos de <see cref="IgdbQueries.GameFields"/>).</summary>
    public static GameMetadata? Game(JsonElement game, DateTimeOffset now)
    {
        if (game.ValueKind != JsonValueKind.Object || !game.TryGetProperty("id", out var idValue) || !idValue.TryGetInt64(out long id))
        {
            return null;
        }

        var ids = new Dictionary<string, string>(StringComparer.Ordinal) { [MetadataPriority.Igdb] = IgdbClient.Number(id) };
        foreach (var external in Array(game, "external_games"))
        {
            if (Uid(external) is { } uid && SourceOf(external) is { } source && !ids.ContainsKey(source))
            {
                ids[source] = uid;
            }
        }

        var brazil = Array(game, "game_localizations").FirstOrDefault(IsBrazil);
        string? coverId = brazil.ValueKind == JsonValueKind.Object && brazil.TryGetProperty("cover", out var localCover) ? Text(localCover, "image_id") : null;
        coverId ??= game.TryGetProperty("cover", out var cover) ? Text(cover, "image_id") : null;
        // Fundo: arte oficial (artworks) em 1080p; sem ela, a primeira captura de tela.
        Uri? backgroundUrl = Array(game, "artworks").Select(a => Text(a, "image_id")).FirstOrDefault(i => i is not null) is { } artwork ? IgdbQueries.Image(artwork, ArtworkSize)
            : Array(game, "screenshots").Select(a => Text(a, "image_id")).FirstOrDefault(i => i is not null) is { } shot ? IgdbQueries.Image(shot, ScreenshotSize)
            : null;

        int? steamAppId = ids.TryGetValue(ExternalGameIds.Steam, out string? steam) && int.TryParse(steam, NumberStyles.None, CultureInfo.InvariantCulture, out int appId) ? appId : null;
        string? summary = Text(game, "summary")?.Trim();
        return new GameMetadata
        {
            Source = MetadataPriority.Igdb,
            FetchedAt = now,
            SteamAppId = steamAppId,
            SourceIds = ids,
            Title = (brazil.ValueKind == JsonValueKind.Object ? Text(brazil, "name") : null) ?? Text(game, "name"),
            // A IGDB não traduz resumos (só nome e capa por região): o resumo é sempre em inglês.
            Description = string.IsNullOrEmpty(summary) ? null : summary,
            DescriptionLanguage = string.IsNullOrEmpty(summary) ? null : "en",
            Genres = Names(game, "genres").Take(3).ToList(),
            Themes = Names(game, "themes").Take(3).ToList(),
            GameModes = Names(game, "game_modes").ToList(),
            Franchise = (game.TryGetProperty("franchise", out var franchise) ? Text(franchise, "name") : null)
                ?? Names(game, "franchises").FirstOrDefault()
                ?? Names(game, "collections").FirstOrDefault(),
            ReleaseDate = game.TryGetProperty("first_release_date", out var date) && date.TryGetInt64(out long seconds)
                ? DateOnly.FromDateTime(DateTimeOffset.FromUnixTimeSeconds(seconds).UtcDateTime)
                : null,
            Developer = Company(game, "developer"),
            Publisher = Company(game, "publisher"),
            CriticScore = Score(game, "aggregated_rating") ?? Score(game, "total_rating"),
            CoverUrl = coverId is null ? null : IgdbQueries.Image(coverId, CoverSize),
            BackgroundUrl = backgroundUrl,
        };
    }

    /// <summary>Resultado de busca para "Corrigir correspondência" (nome, ano e plataformas).</summary>
    public static MetadataMatch? Match(JsonElement game)
    {
        if (!game.TryGetProperty("id", out var idValue) || !idValue.TryGetInt64(out long id) || Text(game, "name") is not { } name)
        {
            return null;
        }

        var platforms = Array(game, "platforms")
            .Select(p => p.ValueKind == JsonValueKind.Number && p.TryGetInt32(out int pid) ? IgdbPlatforms.NameOf(pid) : null)
            .OfType<string>()
            .Distinct()
            .ToList();
        return new MetadataMatch(MetadataPriority.Igdb, IgdbClient.Number(id), name)
        {
            Year = game.TryGetProperty("first_release_date", out var date) && date.TryGetInt64(out long seconds) ? DateTimeOffset.FromUnixTimeSeconds(seconds).UtcDateTime.Year : null,
            Platforms = platforms.Count == 0 ? null : string.Join(", ", platforms),
        };
    }

    /// <summary>Resposta de <c>external_games</c> → (uid na loja → id do jogo na IGDB).</summary>
    public static IReadOnlyDictionary<string, long> ExternalGames(JsonElement array)
    {
        var map = new Dictionary<string, long>(StringComparer.Ordinal);
        if (array.ValueKind != JsonValueKind.Array)
        {
            return map;
        }

        foreach (var item in array.EnumerateArray())
        {
            if (Uid(item) is { } uid && item.TryGetProperty("game", out var game) && game.TryGetInt64(out long id))
            {
                map.TryAdd(uid, id);
            }
        }

        return map;
    }

    private static string? SourceOf(JsonElement external)
    {
        int? source = Int(external, "external_game_source") ?? Int(external, "category");
        return source is int s ? IgdbQueries.ExternalSources.FirstOrDefault(kv => kv.Value == s).Key : null;
    }

    private static string? Uid(JsonElement external) => external.ValueKind == JsonValueKind.Object ? Text(external, "uid") : null;

    private static bool IsBrazil(JsonElement localization)
    {
        if (localization.ValueKind != JsonValueKind.Object || !localization.TryGetProperty("region", out var region) || region.ValueKind != JsonValueKind.Object)
        {
            return false;
        }

        string? identifier = Text(region, "identifier");
        return string.Equals(Text(region, "name"), "Brazil", StringComparison.OrdinalIgnoreCase)
            || identifier is not null && (identifier.Equals("br", StringComparison.OrdinalIgnoreCase) || identifier.EndsWith("_br", StringComparison.OrdinalIgnoreCase) || identifier.EndsWith("-br", StringComparison.OrdinalIgnoreCase));
    }

    private static string? Company(JsonElement game, string role)
    {
        foreach (var involved in Array(game, "involved_companies"))
        {
            if (involved.TryGetProperty(role, out var flag) && flag.ValueKind == JsonValueKind.True
                && involved.TryGetProperty("company", out var company) && Text(company, "name") is { } name)
            {
                return name;
            }
        }

        return null;
    }

    private static int? Score(JsonElement game, string property) =>
        game.TryGetProperty(property, out var value) && value.TryGetDouble(out double score) && score > 0 ? (int)Math.Round(Math.Clamp(score, 0, 100)) : null;

    private static IEnumerable<string> Names(JsonElement game, string property) =>
        Array(game, property).Select(item => Text(item, "name")).OfType<string>().Select(IgdbText.Translate).Distinct(StringComparer.Ordinal);

    private static JsonElement[] Array(JsonElement element, string property) =>
        element.ValueKind == JsonValueKind.Object && element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.Array ? [.. value.EnumerateArray()] : [];

    private static int? Int(JsonElement element, string property) =>
        element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.Number && value.TryGetInt32(out int number) ? number : null;

    private static string? Text(JsonElement element, string property) =>
        element.ValueKind == JsonValueKind.Object && element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;
}
