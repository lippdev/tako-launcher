using Launcher.Core.Presentation;

namespace Launcher.Core.Library;

/// <summary>Filtro de instalação do painel de filtros.</summary>
public enum InstallFilter
{
    Any,
    Installed,
    NotInstalled,
}

/// <summary>
/// O que os filtros e a ordenação precisam saber de um jogo. Serve aos jogos do Tako (<see cref="Of"/>) e aos jogos
/// que o adaptador de temas Playnite mostra, para os dois usarem a mesma regra.
/// </summary>
/// <param name="Source">Fonte como aparece para o usuário ("Steam", "Adicionado").</param>
/// <param name="Score">Nota de 0 a 100 (Metacritic), quando existe.</param>
public readonly record struct LibraryFacts(
    string Title,
    bool IsInstalled,
    bool IsFavorite,
    string Source,
    IReadOnlyList<string> Genres,
    DateTimeOffset? LastPlayedAt,
    TimeSpan PlayTime,
    int? Score,
    DateTimeOffset AddedAt)
{
    public static LibraryFacts Of(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return new LibraryFacts(
            game.Title,
            game.IsInstalled,
            game.IsFavorite,
            GameText.StoreName(game.Store),
            game.Genres,
            game.LastPlayedAt,
            game.PlayTime,
            game.MetacriticScore,
            game.AddedAt);
    }
}

/// <summary>
/// Busca, filtros e ordenação da biblioteca (UX-001/002/004). Imutável: cada mudança no painel gera um novo critério.
/// Dentro de fontes ou gêneros vale qualquer um dos marcados; entre filtros diferentes, todos precisam valer.
/// </summary>
public sealed record LibraryCriteria
{
    private static readonly IReadOnlySet<string> NoValues = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

    public static LibraryCriteria None { get; } = new();

    /// <summary>Texto da busca (sem acento e sem diferenciar maiúsculas; ver <see cref="SearchText"/>).</summary>
    public string Search { get; init; } = string.Empty;

    public InstallFilter Install { get; init; }

    public bool FavoritesOnly { get; init; }

    /// <summary>Só jogos já jogados (a mesma regra da fileira "Jogados recentemente" da Início).</summary>
    public bool PlayedOnly { get; init; }

    public IReadOnlySet<string> Sources { get; init; } = NoValues;

    public IReadOnlySet<string> Genres { get; init; } = NoValues;

    public LibrarySort Sort { get; init; } = LibrarySort.RecentlyPlayed;

    public bool HasSearch => !string.IsNullOrWhiteSpace(Search);

    /// <summary>Quantos filtros do painel estão ligados (a busca e a ordenação não contam).</summary>
    public int ActiveFilterCount =>
        (Install == InstallFilter.Any ? 0 : 1) + (FavoritesOnly ? 1 : 0) + (PlayedOnly ? 1 : 0) + Sources.Count + Genres.Count;

    public bool HasFilters => ActiveFilterCount > 0;

    public bool Matches(LibraryFacts game) => Matches(game, SearchText.Query.Parse(Search));

    /// <summary>Versão para listas: a busca já normalizada uma vez.</summary>
    public bool Matches(LibraryFacts game, SearchText.Query query)
    {
        ArgumentNullException.ThrowIfNull(query);
        return Install switch
        {
            InstallFilter.Installed when !game.IsInstalled => false,
            InstallFilter.NotInstalled when game.IsInstalled => false,
            _ => (!FavoritesOnly || game.IsFavorite)
                && (!PlayedOnly || game.LastPlayedAt is not null)
                && (Sources.Count == 0 || Sources.Contains(game.Source))
                && (Genres.Count == 0 || (game.Genres ?? []).Any(Genres.Contains))
                && (query.IsEmpty || SearchText.Matches(SearchText.Normalize(game.Title), query)),
        };
    }

    /// <summary>Todos → Instalados → Não instalados → Todos.</summary>
    public LibraryCriteria NextInstall() => this with { Install = (InstallFilter)(((int)Install + 1) % 3) };

    public LibraryCriteria ToggleSource(string source) => this with { Sources = Toggle(Sources, source) };

    public LibraryCriteria ToggleGenre(string genre) => this with { Genres = Toggle(Genres, genre) };

    /// <summary>Desliga os filtros do painel; a busca e a ordenação continuam.</summary>
    public LibraryCriteria ClearFilters() => this with
    {
        Install = InstallFilter.Any,
        FavoritesOnly = false,
        PlayedOnly = false,
        Sources = NoValues,
        Genres = NoValues,
    };

    private static HashSet<string> Toggle(IReadOnlySet<string> values, string value)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(value);
        var copy = new HashSet<string>(values, StringComparer.OrdinalIgnoreCase);
        if (!copy.Remove(value))
        {
            copy.Add(value);
        }

        return copy;
    }
}
