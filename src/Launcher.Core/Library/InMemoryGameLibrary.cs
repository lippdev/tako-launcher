namespace Launcher.Core.Library;

public sealed class InMemoryGameLibrary(IEnumerable<Game> games, TimeProvider time) : IGameLibrary
{
    private readonly List<Game> _games = games.ToList();

    public event EventHandler? Changed;

    public IReadOnlyList<Game> Games => _games;

    public Game? Find(string id) => _games.Find(g => g.Id == id);

    public void ToggleFavorite(string id)
    {
        var game = Find(id) ?? throw new KeyNotFoundException($"Jogo '{id}' não está na biblioteca.");
        game.IsFavorite = !game.IsFavorite;
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public void RecordSession(string id, TimeSpan duration)
    {
        var game = Find(id) ?? throw new KeyNotFoundException($"Jogo '{id}' não está na biblioteca.");
        game.LastPlayedAt = time.GetUtcNow();
        game.PlayTime += duration;
        Changed?.Invoke(this, EventArgs.Empty);
    }
}
