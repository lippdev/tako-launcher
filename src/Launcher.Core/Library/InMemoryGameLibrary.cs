namespace Launcher.Core.Library;

/// <summary>Biblioteca só em memória (testes e demonstração).</summary>
public sealed class InMemoryGameLibrary(IEnumerable<Game> games, TimeProvider time) : IGameLibrary
{
    private readonly List<Game> _games = games.ToList();
    private readonly List<ManualGameRecord> _manual = [];

    public event EventHandler? Changed;

    public IReadOnlyList<Game> Games => _games;

    public Game? Find(string id) => _games.Find(g => g.Id == id);

    public ManualGameRecord? FindManual(string id) => _manual.Find(m => m.Id == id);

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

    public void RecordLaunch(string id) => RecordSession(id, TimeSpan.Zero);

    public Game AddManual(ManualGameRecord record)
    {
        ArgumentNullException.ThrowIfNull(record);
        _manual.Add(record);
        var game = record.ToGame();
        _games.Add(game);
        Changed?.Invoke(this, EventArgs.Empty);
        return game;
    }

    public void UpdateManual(ManualGameRecord record)
    {
        ArgumentNullException.ThrowIfNull(record);
        RemoveManual(record.Id);
        AddManual(record);
    }

    public void RemoveManual(string id)
    {
        _manual.RemoveAll(m => m.Id == id);
        _games.RemoveAll(g => g.Id == id);
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public void Refresh() => Changed?.Invoke(this, EventArgs.Empty);
}
