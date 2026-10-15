using System.IO;
using System.Text.Json;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Agente de metadados (como no Plex): para cada jogo, encontra a correspondência no provedor (pelo appid da Steam ou
/// pelo título), busca descrição, gêneros, lançamento, desenvolvedora e nota do Metacritic, e guarda em
/// <c>%LOCALAPPDATA%\Tako\cache\metadata</c>. O cache é aplicado na hora; a atualização roda em segundo plano, um jogo
/// por vez, respeitando o limite da API. "Corrigir correspondência" fixa a escolha do usuário.
/// </summary>
public sealed partial class MetadataService(IMetadataProvider provider, TimeProvider time, ILogger<MetadataService> logger)
{
    private static readonly TimeSpan MaxAge = TimeSpan.FromDays(30);
    private static readonly TimeSpan NoMatchRetry = TimeSpan.FromDays(7);
    private static readonly TimeSpan Spacing = TimeSpan.FromSeconds(1.5);
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    private readonly string _directory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Tako", "cache", "metadata");
    private bool _running;
    private bool _pending;

    /// <summary>Metadados novos para o jogo (id); disparado na thread de UI.</summary>
    public event EventHandler<string>? GameUpdated;

    public string ProviderName => provider.Name;

    /// <summary>Aplica o que já está em cache (chamado pela biblioteca ao montar cada jogo).</summary>
    public void ApplyCached(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        Read(game.Id)?.ApplyTo(game);
    }

    /// <summary>Atualiza em segundo plano os jogos sem dados ou com dados antigos; <paramref name="force"/> busca todos.</summary>
    public async Task RefreshAsync(Func<IReadOnlyList<Game>> games, bool force = false)
    {
        if (_running)
        {
            _pending = true; // a rodada atual termina e roda de novo
            return;
        }

        _running = true;
        try
        {
            do
            {
                _pending = false;
                foreach (var game in games().ToList())
                {
                    if (await UpdateAsync(game, force).ConfigureAwait(true))
                    {
                        await Task.Delay(Spacing).ConfigureAwait(true);
                    }
                }

                force = false;
            }
            while (_pending);
        }
        finally
        {
            _running = false;
        }
    }

    public Task<IReadOnlyList<MetadataMatch>> SearchAsync(string title) => provider.SearchAsync(title, CancellationToken.None);

    /// <summary>Correspondência escolhida pelo usuário: busca os dados desse jogo e fixa a escolha.</summary>
    public async Task<bool> PinAsync(Game game, int steamAppId)
    {
        ArgumentNullException.ThrowIfNull(game);
        var metadata = await provider.FetchAsync(steamAppId, CancellationToken.None).ConfigureAwait(true);
        if (metadata is null)
        {
            return false;
        }

        Write(game.Id, metadata with { IsPinned = true });
        GameUpdated?.Invoke(this, game.Id);
        return true;
    }

    /// <summary>Consulta o provedor se precisar; devolve verdadeiro quando consultou (para espaçar a próxima).</summary>
    private async Task<bool> UpdateAsync(Game game, bool force)
    {
        var cached = Read(game.Id);
        var now = time.GetUtcNow();
        if (!force && cached is not null && now - cached.FetchedAt < (cached.SteamAppId is null ? NoMatchRetry : MaxAge))
        {
            return false;
        }

        int? appId = game.Store == GameStore.Steam ? game.SteamAppId : null;
        appId ??= cached is { IsPinned: true } ? cached.SteamAppId : null;
        if (appId is null)
        {
            var match = TitleMatcher.Best(game.Title, await provider.SearchAsync(game.Title, CancellationToken.None).ConfigureAwait(true));
            if (match is null)
            {
                Write(game.Id, new GameMetadata { Source = provider.Name, FetchedAt = now }); // sem correspondência: tenta de novo em 7 dias
                LogNoMatch(logger, game.Title);
                return true;
            }

            appId = match.SteamAppId;
        }

        var metadata = await provider.FetchAsync(appId.Value, CancellationToken.None).ConfigureAwait(true);
        if (metadata is null)
        {
            return true; // limite da API ou fora do ar: fica para a próxima rodada
        }

        Write(game.Id, metadata with { IsPinned = cached?.IsPinned ?? false });
        GameUpdated?.Invoke(this, game.Id);
        return true;
    }

    private string PathFor(string gameId) => Path.Combine(_directory, string.Concat(gameId.Select(c => Path.GetInvalidFileNameChars().Contains(c) ? '_' : c)) + ".json");

    private GameMetadata? Read(string gameId)
    {
        string path = PathFor(gameId);
        try
        {
            return File.Exists(path) ? JsonSerializer.Deserialize<GameMetadata>(File.ReadAllText(path), Options) : null;
        }
        catch (Exception ex) when (ex is JsonException or IOException)
        {
            return null;
        }
    }

    private void Write(string gameId, GameMetadata metadata)
    {
        Directory.CreateDirectory(_directory);
        File.WriteAllText(PathFor(gameId), JsonSerializer.Serialize(metadata, Options));
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Metadados: nenhuma correspondência para '{Title}'")]
    private static partial void LogNoMatch(ILogger logger, string title);
}
