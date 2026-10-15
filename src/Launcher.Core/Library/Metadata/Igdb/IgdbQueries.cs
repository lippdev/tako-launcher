using System.Globalization;
using System.Text;

namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>Consultas Apicalypse da IGDB v4 (texto puro, testado).</summary>
public static class IgdbQueries
{
    /// <summary>Até 500 itens por consulta (limite da IGDB).</summary>
    public const int MaxLimit = 500;

    /// <summary>
    /// Campos de um jogo, já expandidos (nome das empresas, gêneros, imagens), para buscar tudo numa consulta.
    /// <c>external_game_source</c> substitui o antigo <c>category</c> (descontinuado), com os mesmos números.
    /// </summary>
    public const string GameFields =
        "name, summary, first_release_date, aggregated_rating, total_rating, platforms, " +
        "genres.name, themes.name, game_modes.name, " +
        "involved_companies.company.name, involved_companies.developer, involved_companies.publisher, " +
        "cover.image_id, artworks.image_id, screenshots.image_id, " +
        "franchise.name, franchises.name, collections.name, " +
        "external_games.uid, external_games.external_game_source, " +
        "game_localizations.name, game_localizations.region.identifier, game_localizations.region.name, game_localizations.cover.image_id";

    /// <summary>Lojas → id da fonte em <c>external_game_sources</c> (os mesmos números do antigo <c>category</c>).</summary>
    public static readonly IReadOnlyDictionary<string, int> ExternalSources = new Dictionary<string, int>(StringComparer.Ordinal)
    {
        [ExternalGameIds.Steam] = 1,
        [ExternalGameIds.Gog] = 5,
        [ExternalGameIds.Microsoft] = 11,
        [ExternalGameIds.Epic] = 26,
    };

    /// <summary>Busca por nome, só jogos principais (sem versões/edições), filtrada pela plataforma quando conhecida.</summary>
    public static string Search(string title, IReadOnlyList<int> platforms, int limit = 10)
    {
        ArgumentNullException.ThrowIfNull(title);
        ArgumentNullException.ThrowIfNull(platforms);
        var where = new List<string> { "version_parent = null" };
        if (platforms.Count > 0)
        {
            where.Add($"platforms = ({List(platforms)})");
        }

        return $"search \"{Escape(title)}\"; fields {GameFields}; where {string.Join(" & ", where)}; limit {limit.ToString(CultureInfo.InvariantCulture)};";
    }

    /// <summary>Vários jogos pelo id (lote).</summary>
    public static string GamesById(IEnumerable<long> ids)
    {
        var list = ids.Distinct().Order().ToList(); // ordem fixa: a mesma consulta acerta o cache
        return $"fields {GameFields}; where id = ({List(list)}); limit {Math.Clamp(list.Count, 1, MaxLimit).ToString(CultureInfo.InvariantCulture)};";
    }

    /// <summary>Ids de jogos da IGDB a partir dos ids numa loja (lote, uma fonte por consulta).</summary>
    public static string ExternalGames(int source, IEnumerable<string> uids)
    {
        var list = uids.Distinct(StringComparer.Ordinal).ToList();
        string quoted = string.Join(",", list.Select(u => $"\"{Escape(u)}\""));
        return $"fields game, uid, external_game_source; where external_game_source = {source.ToString(CultureInfo.InvariantCulture)} & uid = ({quoted}); limit {Math.Clamp(list.Count * 2, 1, MaxLimit).ToString(CultureInfo.InvariantCulture)};";
    }

    /// <summary>Consulta mínima do botão "Testar".</summary>
    public const string Ping = "fields name; limit 1;";

    /// <summary>Endereço de uma imagem da IGDB no tamanho pedido (<c>t_cover_big</c>, <c>t_1080p</c>, <c>t_screenshot_huge</c>).</summary>
    public static Uri Image(string imageId, string size) => new($"https://images.igdb.com/igdb/image/upload/{size}/{Uri.EscapeDataString(imageId)}.jpg");

    /// <summary>Texto entre aspas no Apicalypse: escapa barra invertida e aspas, tira quebras de linha.</summary>
    public static string Escape(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        var builder = new StringBuilder(text.Length);
        foreach (char c in text)
        {
            switch (c)
            {
                case '\\' or '"':
                    builder.Append('\\').Append(c);
                    break;
                case '\r' or '\n' or '\t':
                    builder.Append(' ');
                    break;
                default:
                    builder.Append(c);
                    break;
            }
        }

        return builder.ToString().Trim();
    }

    private static string List<T>(IEnumerable<T> values) where T : IFormattable =>
        string.Join(",", values.Select(v => v.ToString(null, CultureInfo.InvariantCulture)));
}
