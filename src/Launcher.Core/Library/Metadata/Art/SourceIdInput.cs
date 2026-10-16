using System.Text.RegularExpressions;

namespace Launcher.Core.Library.Metadata.Art;

/// <summary>
/// O que o usuário digitou ou colou para fixar o jogo numa fonte ("Fontes e IDs", ADR-0023): o número do jogo ou o
/// endereço da página dele. A IGDB usa o nome na URL (<c>igdb.com/games/hades</c>), que vira o número por uma consulta.
/// </summary>
public sealed record SourceIdParse(string? Id, string? Slug, string? Problem)
{
    public bool IsValid => Problem is null;
}

/// <summary>Lê ids e endereços de página de cada fonte (puro, testado).</summary>
public static partial class SourceIdInput
{
    /// <summary>Exemplo mostrado no teclado virtual de cada fonte.</summary>
    public static string Example(string source) => source switch
    {
        ArtSources.Steam => "Ex.: 1145360 ou store.steampowered.com/app/1145360",
        ArtSources.Igdb => "Ex.: 113112 ou igdb.com/games/hades",
        ArtSources.ScreenScraper => "Ex.: 3 ou screenscraper.fr/gameinfos.php?gameid=3",
        ArtSources.SteamGridDb => "Ex.: 5249 ou steamgriddb.com/game/5249",
        _ => "Número do jogo nesta fonte",
    };

    public static SourceIdParse Parse(string source, string? text)
    {
        string value = (text ?? string.Empty).Trim();
        if (value.Length == 0)
        {
            return new SourceIdParse(null, null, "Digite ou cole o número do jogo ou o endereço da página dele.");
        }

        if (DigitsOnly().IsMatch(value))
        {
            return value.TrimStart('0').Length == 0 ? Bad(source) : new SourceIdParse(value.TrimStart('0'), null, null);
        }

        var match = source switch
        {
            ArtSources.Steam => SteamUrl().Match(value),
            ArtSources.Igdb => IgdbUrl().Match(value),
            ArtSources.ScreenScraper => ScreenScraperUrl().Match(value),
            ArtSources.SteamGridDb => SteamGridDbUrl().Match(value),
            _ => Match.Empty,
        };
        if (!match.Success)
        {
            return Bad(source);
        }

        if (match.Groups["id"].Success)
        {
            return new SourceIdParse(match.Groups["id"].Value, null, null);
        }

        return new SourceIdParse(null, match.Groups["slug"].Value.ToLowerInvariant(), null);
    }

    private static SourceIdParse Bad(string source) =>
        new(null, null, $"Não reconheci o jogo nesse texto. {Example(source)}.");

    [GeneratedRegex(@"^\d{1,12}$")]
    private static partial Regex DigitsOnly();

    [GeneratedRegex(@"(?:store\.steampowered\.com|steamcommunity\.com|steamdb\.info|s\.team)/app/(?<id>\d{1,12})", RegexOptions.IgnoreCase)]
    private static partial Regex SteamUrl();

    [GeneratedRegex(@"igdb\.com/games/(?<slug>[a-z0-9][a-z0-9\-\.]*)", RegexOptions.IgnoreCase)]
    private static partial Regex IgdbUrl();

    [GeneratedRegex(@"screenscraper\.fr/.*[?&](?:gameid|jeuid)=(?<id>\d{1,12})", RegexOptions.IgnoreCase)]
    private static partial Regex ScreenScraperUrl();

    [GeneratedRegex(@"steamgriddb\.com/(?:game|grid|hero|logo|icon)s?/(?<id>\d{1,12})", RegexOptions.IgnoreCase)]
    private static partial Regex SteamGridDbUrl();
}
