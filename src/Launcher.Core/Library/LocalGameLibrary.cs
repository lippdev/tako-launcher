namespace Launcher.Core.Library;

/// <summary>
/// Biblioteca real: jogos encontrados nas lojas instaladas (<paramref name="discover"/>, hoje a Steam) mais os adicionados
/// pelo usuário, com favoritos e "jogado por último" guardados em <see cref="LibraryStore"/>.
/// </summary>
public sealed class LocalGameLibrary(Func<IReadOnlyList<Game>> discover, LibraryStore store, TimeProvider time, Action<Game>? enrich = null) : IGameLibrary
{
    private List<Game> _games = [];

    public event EventHandler? Changed;

    public IReadOnlyList<Game> Games => _games;

    public Game? Find(string id) => _games.Find(g => g.Id == id);

    public ManualGameRecord? FindManual(string id) => store.Manual.Find(m => m.Id == id);

    /// <summary>Lê o arquivo do usuário e procura os jogos instalados.</summary>
    public void Load()
    {
        store.Load();
        Rebuild();
    }

    public void Refresh()
    {
        Rebuild();
        Changed?.Invoke(this, EventArgs.Empty);
    }

    public void ToggleFavorite(string id)
    {
        var game = Find(id) ?? throw new KeyNotFoundException($"Jogo '{id}' não está na biblioteca.");
        game.IsFavorite = !game.IsFavorite;
        Update(id, s => s with { IsFavorite = game.IsFavorite });
    }

    public void RecordSession(string id, TimeSpan duration)
    {
        var game = Find(id) ?? throw new KeyNotFoundException($"Jogo '{id}' não está na biblioteca.");
        var now = time.GetUtcNow();
        game.LastPlayedAt = now;
        game.PlayTime += duration;
        Update(id, s => s with { LastPlayedAt = now, PlayMinutes = s.PlayMinutes + duration.TotalMinutes });
    }

    public void RecordLaunch(string id) => RecordSession(id, TimeSpan.Zero);

    public Game AddManual(ManualGameRecord record)
    {
        ArgumentNullException.ThrowIfNull(record);
        store.Manual.Add(record);
        store.Save();
        Refresh();
        return Find(record.Id)!;
    }

    public void UpdateManual(ManualGameRecord record)
    {
        ArgumentNullException.ThrowIfNull(record);
        int index = store.Manual.FindIndex(m => m.Id == record.Id);
        if (index < 0)
        {
            throw new KeyNotFoundException($"Jogo '{record.Id}' não foi adicionado manualmente.");
        }

        store.Manual[index] = record;
        store.Save();
        Refresh();
    }

    public void RemoveManual(string id)
    {
        store.Manual.RemoveAll(m => m.Id == id);
        store.State.Remove(id);
        store.Save();
        Refresh();
    }

    private void Update(string id, Func<GameStateRecord, GameStateRecord> change)
    {
        store.State[id] = change(store.State.GetValueOrDefault(id) ?? new GameStateRecord());
        store.Save();
        Changed?.Invoke(this, EventArgs.Empty);
    }

    private void Rebuild()
    {
        var games = new List<Game>();
        foreach (var game in discover().Concat(store.Manual.Select(m => m.ToGame())))
        {
            if (games.Exists(g => g.Id == game.Id))
            {
                continue;
            }

            if (store.State.TryGetValue(game.Id, out var state))
            {
                game.IsFavorite = state.IsFavorite;
                if (state.LastPlayedAt > game.LastPlayedAt || game.LastPlayedAt is null)
                {
                    game.LastPlayedAt = state.LastPlayedAt;
                }

                // A Steam já soma o próprio tempo; o registrado pelo launcher vale para os demais.
                if (game.Store != GameStore.Steam)
                {
                    game.PlayTime += TimeSpan.FromMinutes(state.PlayMinutes);
                }
            }

            enrich?.Invoke(game); // metadados em cache (descrição, gêneros, arte de jogos manuais)
            games.Add(game);
        }

        _games = games;
    }
}
