using Launcher.Core.Persistence;

namespace Launcher.Core.Library;

/// <summary>Coleção criada pelo usuário (UX-003): um nome e os jogos dela (por id).</summary>
public sealed record CollectionInfo(string Id, string Name, DateTimeOffset CreatedAt, IReadOnlySet<string> GameIds)
{
    public bool Contains(string gameId) => GameIds.Contains(gameId);
}

/// <summary>Regras do nome de uma coleção: sem espaços sobrando, não vazio, curto e sem repetir outro (sem acento/caixa).</summary>
public static class CollectionNames
{
    public const int MaxLength = 40;

    /// <summary>Nome limpo (espaços nas pontas e repetidos saem).</summary>
    public static string Clean(string? name) =>
        string.Join(' ', (name ?? string.Empty).Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries));

    /// <summary>Mensagem para o usuário quando o nome não serve; nulo quando serve.</summary>
    /// <param name="exceptId">Coleção sendo renomeada (pode manter o próprio nome).</param>
    public static string? Problem(string? name, IEnumerable<CollectionInfo> existing, string? exceptId = null)
    {
        ArgumentNullException.ThrowIfNull(existing);
        string clean = Clean(name);
        if (clean.Length == 0)
        {
            return "Dê um nome à coleção.";
        }

        if (clean.Length > MaxLength)
        {
            return $"Use até {MaxLength} letras.";
        }

        return existing.Any(c => c.Id != exceptId && LibraryQuery.TitleComparer.Equals(c.Name, clean))
            ? "Já existe uma coleção com esse nome."
            : null;
    }
}

/// <summary>
/// Coleções do usuário em memória, gravadas no <see cref="CollectionStore"/> a cada mudança. Os jogos guardam em
/// <see cref="Game.Collections"/> de quais coleções fazem parte (para os filtros), atualizado por <see cref="ApplyTo"/>.
/// </summary>
public sealed class CollectionLibrary(CollectionStore store, TimeProvider time)
{
    private static readonly IReadOnlySet<string> NoCollections = new HashSet<string>(StringComparer.Ordinal);
    private List<CollectionInfo> _collections = [];

    public event EventHandler? Changed;

    /// <summary>Em ordem de nome.</summary>
    public IReadOnlyList<CollectionInfo> Collections => _collections;

    public void Load()
    {
        _collections = Sorted(store.LoadAll());
    }

    public CollectionInfo? Find(string id) => _collections.Find(c => c.Id == id);

    /// <summary>Coleções que têm o jogo.</summary>
    public IReadOnlyList<CollectionInfo> Of(string gameId) => _collections.Where(c => c.Contains(gameId)).ToList();

    /// <summary>Ids das coleções de cada jogo, para os filtros da biblioteca.</summary>
    public void ApplyTo(IEnumerable<Game> games)
    {
        ArgumentNullException.ThrowIfNull(games);
        foreach (var game in games)
        {
            var ids = _collections.Where(c => c.Contains(game.Id)).Select(c => c.Id).ToHashSet(StringComparer.Ordinal);
            game.Collections = ids.Count == 0 ? NoCollections : ids;
        }
    }

    /// <summary>Cria a coleção (opcionalmente já com um jogo). Lança <see cref="ArgumentException"/> se o nome não serve.</summary>
    public CollectionInfo Create(string name, string? firstGameId = null)
    {
        string clean = Valid(name, null);
        var now = time.GetUtcNow();
        var collection = new CollectionInfo(Guid.NewGuid().ToString("N"), clean, now, firstGameId is null ? NoCollections : new HashSet<string>(StringComparer.Ordinal) { firstGameId });
        store.Insert(collection);
        if (firstGameId is not null)
        {
            store.AddGame(collection.Id, firstGameId, now);
        }

        Replace(collection.Id, collection);
        return collection;
    }

    public void Rename(string id, string name)
    {
        var collection = Find(id) ?? throw new KeyNotFoundException($"Coleção '{id}' não existe.");
        string clean = Valid(name, id);
        store.Rename(id, clean);
        Replace(id, collection with { Name = clean });
    }

    public void Delete(string id)
    {
        store.Delete(id);
        Replace(id, null);
    }

    /// <summary>Põe ou tira o jogo da coleção. Devolve verdadeiro quando o jogo entrou.</summary>
    public bool Toggle(string collectionId, string gameId)
    {
        var collection = Find(collectionId) ?? throw new KeyNotFoundException($"Coleção '{collectionId}' não existe.");
        var games = new HashSet<string>(collection.GameIds, StringComparer.Ordinal);
        bool added = games.Add(gameId);
        if (added)
        {
            store.AddGame(collectionId, gameId, time.GetUtcNow());
        }
        else
        {
            games.Remove(gameId);
            store.RemoveGame(collectionId, gameId);
        }

        Replace(collectionId, collection with { GameIds = games });
        return added;
    }

    private static List<CollectionInfo> Sorted(IEnumerable<CollectionInfo> collections) =>
        collections.OrderBy(c => c.Name, LibraryQuery.TitleComparer).ThenBy(c => c.CreatedAt).ToList();

    private string Valid(string name, string? exceptId)
    {
        if (CollectionNames.Problem(name, _collections, exceptId) is { } problem)
        {
            throw new ArgumentException(problem, nameof(name));
        }

        return CollectionNames.Clean(name);
    }

    private void Replace(string id, CollectionInfo? collection)
    {
        var list = _collections.Where(c => c.Id != id).ToList();
        if (collection is not null)
        {
            list.Add(collection);
        }

        _collections = Sorted(list);
        Changed?.Invoke(this, EventArgs.Empty);
    }
}
