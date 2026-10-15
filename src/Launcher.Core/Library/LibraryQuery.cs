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
}

public static class LibraryQuery
{
    public static IReadOnlyList<Game> Apply(IEnumerable<Game> games, LibraryFilter filter, LibrarySort sort)
    {
        var matching = games.Where(filter.Matches);
        IOrderedEnumerable<Game> ordered = sort switch
        {
            LibrarySort.Name => matching.OrderBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase),
            LibrarySort.PlayTime => matching.OrderByDescending(g => g.PlayTime),
            LibrarySort.RecentlyAdded => matching.OrderByDescending(g => g.AddedAt),
            // Jogados primeiro (mais recente no topo); nunca jogados depois, por nome.
            _ => matching.OrderBy(g => g.LastPlayedAt is null).ThenByDescending(g => g.LastPlayedAt),
        };
        return ordered.ThenBy(g => g.Title, StringComparer.CurrentCultureIgnoreCase).ToList();
    }

    /// <summary>Fileira "Jogados recentemente" da Início: só jogos já jogados, do mais recente ao mais antigo.</summary>
    public static IReadOnlyList<Game> RecentlyPlayed(IEnumerable<Game> games) =>
        games.Where(g => g.LastPlayedAt is not null).OrderByDescending(g => g.LastPlayedAt).ToList();

    public static IReadOnlyList<Game> RecentlyAdded(IEnumerable<Game> games, int count) =>
        games.OrderByDescending(g => g.AddedAt).Take(count).ToList();
}
