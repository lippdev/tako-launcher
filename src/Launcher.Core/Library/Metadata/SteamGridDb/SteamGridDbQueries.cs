using System.Globalization;
using System.Text.Json;
using Launcher.Core.Library.Metadata.Art;

namespace Launcher.Core.Library.Metadata.SteamGridDb;

/// <summary>
/// Filtro do seletor para o SteamGridDB (ADR-0023): tipo de imagem, capa vertical ou horizontal, estilo e se mostra as
/// animadas. NSFW e humor ficam sempre de fora.
/// </summary>
public sealed record SteamGridDbFilter(ArtKind Kind)
{
    /// <summary>Capa horizontal (920×430, 460×215) em vez da vertical (600×900, 342×482, 660×930).</summary>
    public bool Horizontal { get; init; }

    /// <summary>Estilo (<see cref="SteamGridDbQueries.Styles"/>); nulo = todos.</summary>
    public string? Style { get; init; }

    /// <summary>Inclui imagens animadas (webp/apng); desligado = só estáticas.</summary>
    public bool Animated { get; init; }
}

/// <summary>Caminhos da API, filtros e escolha automática (puro, testado com respostas escritas à mão).</summary>
public static class SteamGridDbQueries
{
    public static readonly IReadOnlyList<string> VerticalCover = ["600x900", "342x482", "660x930"];
    public static readonly IReadOnlyList<string> HorizontalCover = ["920x430", "460x215"];

    /// <summary>Estilos que a API aceita por tipo de imagem.</summary>
    public static IReadOnlyList<string> Styles(ArtKind kind) => kind switch
    {
        ArtKind.Cover => ["alternate", "blurred", "white_logo", "material", "no_logo"],
        ArtKind.Hero => ["alternate", "blurred", "material"],
        ArtKind.Logo => ["official", "white", "black", "custom"],
        _ => ["official", "custom"],
    };

    /// <summary>Nome do estilo para a tela.</summary>
    public static string StyleLabel(string? style) => style switch
    {
        null => "Todos",
        "alternate" => "Alternativo",
        "blurred" => "Desfocado",
        "white_logo" => "Logo branco",
        "material" => "Material",
        "no_logo" => "Sem logo",
        "official" => "Oficial",
        "white" => "Branco",
        "black" => "Preto",
        "custom" => "Da comunidade",
        _ => style,
    };

    public static string Autocomplete(string term) => "search/autocomplete/" + Uri.EscapeDataString((term ?? string.Empty).Trim());

    public static string BySteamAppId(int appId) => "games/steam/" + appId.ToString(CultureInfo.InvariantCulture);

    public static string ById(string gameId) => "games/id/" + Uri.EscapeDataString(gameId);

    /// <summary>
    /// Imagens do jogo com o filtro: <c>grids</c> (capa), <c>heroes</c> (fundo), <c>logos</c> e <c>icons</c>; dimensões só
    /// para a capa; <c>types=static</c> sem as animadas; NSFW e humor desligados.
    /// </summary>
    public static string Images(string gameId, SteamGridDbFilter filter, int page = 0)
    {
        ArgumentNullException.ThrowIfNull(filter);
        string endpoint = filter.Kind switch
        {
            ArtKind.Cover => "grids",
            ArtKind.Hero => "heroes",
            ArtKind.Logo => "logos",
            _ => "icons",
        };
        var query = new List<string>();
        if (filter.Kind == ArtKind.Cover)
        {
            query.Add("dimensions=" + string.Join(",", filter.Horizontal ? HorizontalCover : VerticalCover));
        }

        if (filter.Style is { } style && Styles(filter.Kind).Contains(style))
        {
            query.Add("styles=" + style);
        }

        query.Add("types=" + (filter.Animated ? "static,animated" : "static"));
        query.Add("nsfw=false");
        query.Add("humor=false");
        if (page > 0)
        {
            query.Add("page=" + page.ToString(CultureInfo.InvariantCulture)); // rolagem infinita do seletor
        }

        return $"{endpoint}/game/{Uri.EscapeDataString(gameId)}?{string.Join("&", query)}";
    }

    /// <summary>
    /// Aplica o filtro na lista (a API pode ignorar algum parâmetro): dimensão da capa, estilo, animadas, NSFW e humor; a
    /// mais votada primeiro.
    /// </summary>
    public static IReadOnlyList<ArtCandidate> Apply(IEnumerable<ArtCandidate> candidates, SteamGridDbFilter filter)
    {
        ArgumentNullException.ThrowIfNull(candidates);
        ArgumentNullException.ThrowIfNull(filter);
        var sizes = filter.Horizontal ? HorizontalCover : VerticalCover;
        return [.. candidates
            .Where(c => c.Kind == filter.Kind && !c.IsNsfw && !c.IsHumor)
            .Where(c => filter.Animated || !c.IsAnimated)
            .Where(c => filter.Style is null || string.Equals(c.Style, filter.Style, StringComparison.Ordinal))
            .Where(c => filter.Kind != ArtKind.Cover || c.Width is null || sizes.Contains(Size(c)))
            .OrderByDescending(c => c.Score)];
    }

    /// <summary>
    /// Escolha automática ("Usar quando faltar capa"): estática, sem NSFW/humor; capa vertical prefere 600×900, depois a
    /// mais votada; fundo, a mais votada. Nula se nada servir.
    /// </summary>
    public static ArtCandidate? BestAutomatic(IEnumerable<ArtCandidate> candidates, ArtKind kind)
    {
        var allowed = Apply(candidates, new SteamGridDbFilter(kind));
        return allowed.OrderByDescending(c => kind == ArtKind.Cover && Size(c) == "600x900").ThenByDescending(c => c.Score).FirstOrDefault();
    }

    private static string Size(ArtCandidate c) => string.Create(CultureInfo.InvariantCulture, $"{c.Width}x{c.Height}");
}

/// <summary>Respostas do SteamGridDB → jogos e imagens (puro).</summary>
public static class SteamGridDbParser
{
    /// <summary><c>search/autocomplete</c> (lista) ou <c>games/…</c> (um jogo) → resultados.</summary>
    public static IReadOnlyList<MetadataMatch> Games(JsonElement data)
    {
        var items = data.ValueKind switch
        {
            JsonValueKind.Array => data.EnumerateArray().ToList(),
            JsonValueKind.Object => [data],
            _ => [],
        };
        var result = new List<MetadataMatch>();
        foreach (var item in items)
        {
            if (Long(item, "id") is long id && Text(item, "name") is { } name)
            {
                result.Add(new MetadataMatch(MetadataPriority.SteamGridDb, id.ToString(CultureInfo.InvariantCulture), name)
                {
                    Year = Long(item, "release_date") is long seconds and > 0 ? DateTimeOffset.FromUnixTimeSeconds(seconds).UtcDateTime.Year : null,
                    Platforms = item.TryGetProperty("types", out var types) && types.ValueKind == JsonValueKind.Array
                        ? string.Join(", ", types.EnumerateArray().Select(t => t.GetString()).OfType<string>().Select(PlatformLabel).Distinct())
                        : null,
                });
            }
        }

        return result;
    }

    /// <summary>Lista de <c>grids</c>/<c>heroes</c>/<c>logos</c>/<c>icons</c> → candidatos com autor, estilo e tamanho.</summary>
    public static IReadOnlyList<ArtCandidate> Images(JsonElement data, ArtKind kind)
    {
        if (data.ValueKind != JsonValueKind.Array)
        {
            return [];
        }

        var result = new List<ArtCandidate>();
        foreach (var item in data.EnumerateArray())
        {
            if (Text(item, "url") is not { } url || !Uri.TryCreate(url, UriKind.Absolute, out var uri) || uri.Scheme != Uri.UriSchemeHttps)
            {
                continue;
            }

            string? mime = Text(item, "mime");
            Uri? thumb = Text(item, "thumb") is { } t && Uri.TryCreate(t, UriKind.Absolute, out var thumbUri) && thumbUri.Scheme == Uri.UriSchemeHttps ? thumbUri : null;
            result.Add(new ArtCandidate(MetadataPriority.SteamGridDb, kind, uri)
            {
                Thumbnail = thumb,
                Width = (int?)Long(item, "width"),
                Height = (int?)Long(item, "height"),
                Style = Text(item, "style"),
                Author = item.TryGetProperty("author", out var author) ? Text(author, "name") : null,
                Score = (int)(Long(item, "score") ?? 0),
                IsNsfw = Bool(item, "nsfw"),
                IsHumor = Bool(item, "humor"),
                IsAnimated = mime is "image/webp" or "image/apng" || url.EndsWith(".webp", StringComparison.OrdinalIgnoreCase),
                Label = SteamGridDbQueries.StyleLabel(Text(item, "style")),
            });
        }

        return result;
    }

    private static string PlatformLabel(string type) => type switch
    {
        "steam" => "Steam",
        "origin" => "EA",
        "egs" => "Epic",
        "bnet" => "Battle.net",
        "uplay" => "Ubisoft",
        "flashpoint" => "Flashpoint",
        "eshop" => "Nintendo",
        _ => type,
    };

    private static bool Bool(JsonElement element, string property) =>
        element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.True;

    private static long? Long(JsonElement element, string property) =>
        element.ValueKind == JsonValueKind.Object && element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.Number && value.TryGetInt64(out long number) ? number : null;

    private static string? Text(JsonElement element, string property) =>
        element.ValueKind == JsonValueKind.Object && element.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.String ? value.GetString() : null;
}
