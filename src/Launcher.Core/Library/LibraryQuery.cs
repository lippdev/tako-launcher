namespace Launcher.Core.Library;

public enum LibraryFilterKind
{
    All,
    Installed,
    Favorites,
    Store,
}

/// <summary>Filtro da biblioteca (barra lateral).</summary>
public readonly record struct LibraryFilter(LibraryFilterKind Kind, GameStore? Store = null)
{
    public static LibraryFilter All { get; } = new(LibraryFilterKind.All);

    public static LibraryFilter Installed { get; } = new(LibraryFilterKind.Installed);

    public static LibraryFilter Favorites { get; } = new(LibraryFilterKind.Favorites);

    public static LibraryFilter ForStore(GameStore store) => new(LibraryFilterKind.Store, store);

    public bool Matches(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return Kind switch
        {
            LibraryFilterKind.Installed => game.IsInstalled,
            LibraryFilterKind.Favorites => game.IsFavorite,
            LibraryFilterKind.Store => game.Store == Store,
            _ => true,
        };
    }
}

public enum LibrarySort
{
    RecentlyPlayed,
    Name,
    PlayTime,
    RecentlyAdded,

    /// <summary>Nota (Metacritic), maior primeiro; jogos sem nota no fim.</summary>
    Rating,
}

public static class LibraryQuery
{
    /// <summary>Ordem de nomes da biblioteca: sem diferenciar maiúsculas nem acentos ("Él" junto de "El").</summary>
    public static StringComparer TitleComparer { get; } =
        StringComparer.Create(System.Globalization.CultureInfo.InvariantCulture, System.Globalization.CompareOptions.IgnoreCase | System.Globalization.CompareOptions.IgnoreNonSpace);

    public static IReadOnlyList<Game> Apply(IEnumerable<Game> games, LibraryFilter filter, LibrarySort sort) =>
        Apply(games, filter, LibraryCriteria.None with { Sort = sort });

    /// <summary>Biblioteca do Tako: rota da barra lateral + busca, filtros do painel e ordenação.</summary>
    public static IReadOnlyList<Game> Apply(IEnumerable<Game> games, LibraryFilter filter, LibraryCriteria criteria) =>
        Apply(games.Where(filter.Matches), LibraryFacts.Of, criteria);

    /// <summary>
    /// Filtra e ordena qualquer lista de jogos (os do Tako ou os de um tema Playnite) pelos fatos de cada um. Empates
    /// saem em ordem de nome.
    /// </summary>
    public static IReadOnlyList<T> Apply<T>(IEnumerable<T> items, Func<T, LibraryFacts> facts, LibraryCriteria criteria)
    {
        ArgumentNullException.ThrowIfNull(items);
        ArgumentNullException.ThrowIfNull(facts);
        ArgumentNullException.ThrowIfNull(criteria);
        var query = SearchText.Query.Parse(criteria.Search);
        var matching = items.Select(item => (Item: item, Facts: facts(item))).Where(row => criteria.Matches(row.Facts, query));
        var ordered = criteria.Sort switch
        {
            LibrarySort.Name => matching.OrderBy(r => r.Facts.Title, TitleComparer),
            LibrarySort.PlayTime => matching.OrderByDescending(r => r.Facts.PlayTime),
            LibrarySort.RecentlyAdded => matching.OrderByDescending(r => r.Facts.AddedAt),
            LibrarySort.Rating => matching.OrderBy(r => r.Facts.Score is null).ThenByDescending(r => r.Facts.Score),
            // Jogados primeiro (mais recente no topo); nunca jogados depois, por nome.
            _ => matching.OrderBy(r => r.Facts.LastPlayedAt is null).ThenByDescending(r => r.Facts.LastPlayedAt),
        };
        return ordered.ThenBy(r => r.Facts.Title, TitleComparer).Select(r => r.Item).ToList();
    }

    /// <summary>Fontes presentes na lista (Steam, Adicionado...), com quantos jogos cada uma tem, em ordem de nome.</summary>
    public static IReadOnlyList<FacetValue> Sources<T>(IEnumerable<T> items, Func<T, LibraryFacts> facts) =>
        Facet(items.Select(i => (IEnumerable<string>)[facts(i).Source]));

    /// <summary>Gêneros presentes na lista (vêm dos metadados), com quantos jogos cada um tem, em ordem de nome.</summary>
    public static IReadOnlyList<FacetValue> Genres<T>(IEnumerable<T> items, Func<T, LibraryFacts> facts) =>
        Facet(items.Select(i => facts(i).Genres));

    /// <summary>Sistemas de emulador presentes (chaves "snes", "ps1"; EMU-007), na ordem da tabela de sistemas, com quantos jogos cada um tem.</summary>
    public static IReadOnlyList<FacetValue> Systems<T>(IEnumerable<T> items, Func<T, LibraryFacts> facts)
    {
        var order = Metadata.ScreenScraper.ScreenScraperSystems.All.Select(s => s.Key).ToList();
        return [.. items.Select(i => facts(i).System).OfType<string>()
            .GroupBy(s => s, StringComparer.OrdinalIgnoreCase)
            .Select(g => new FacetValue(g.Key, g.Count()))
            .OrderBy(f => order.IndexOf(f.Name) is int i and >= 0 ? i : int.MaxValue)
            .ThenBy(f => f.Name, StringComparer.OrdinalIgnoreCase)];
    }

    /// <summary>Fileira "Jogados recentemente" da Início: só jogos já jogados, do mais recente ao mais antigo.</summary>
    public static IReadOnlyList<Game> RecentlyPlayed(IEnumerable<Game> games) =>
        games.Where(g => g.LastPlayedAt is not null).OrderByDescending(g => g.LastPlayedAt).ToList();

    public static IReadOnlyList<Game> RecentlyAdded(IEnumerable<Game> games, int count) =>
        games.OrderByDescending(g => g.AddedAt).Take(count).ToList();

    private static List<FacetValue> Facet(IEnumerable<IEnumerable<string>> values) =>
        values.SelectMany(v => v.Where(x => !string.IsNullOrWhiteSpace(x)).Select(x => x.Trim()).Distinct(StringComparer.OrdinalIgnoreCase))
            .GroupBy(v => v, StringComparer.OrdinalIgnoreCase)
            .Select(g => new FacetValue(g.First(), g.Count()))
            .OrderBy(f => f.Name, TitleComparer)
            .ToList();
}

/// <summary>Um valor de filtro (fonte ou gênero) e quantos jogos têm esse valor.</summary>
public sealed record FacetValue(string Name, int Count);
