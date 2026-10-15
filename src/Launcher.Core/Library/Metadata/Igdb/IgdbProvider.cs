using System.Collections.Concurrent;
using System.Globalization;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;

namespace Launcher.Core.Library.Metadata.Igdb;

/// <summary>
/// Provedor IGDB (ADR-0021): jogos de qualquer loja e de emuladores. Casa primeiro pelos ids nas lojas
/// (<c>external_games</c>: Steam, GOG, Epic, Microsoft Store), sem ambiguidade; senão, pelo nome, filtrando pela
/// plataforma. Os jogos encontrados ficam na memória: buscar e depois detalhar não repete a consulta, e o
/// <see cref="PrefetchAsync"/> resolve a biblioteca inteira em poucas consultas em lote. Sem chave configurada, fica
/// desligado e não consulta nada.
/// </summary>
public sealed partial class IgdbProvider(IgdbClient client, TimeProvider time, ILogger<IgdbProvider>? logger = null) : IBatchMetadataProvider
{
    private static readonly string[] ExactOrder = [ExternalGameIds.Steam, ExternalGameIds.Gog, ExternalGameIds.Epic, ExternalGameIds.Microsoft];

    private readonly ILogger _logger = logger ?? NullLogger<IgdbProvider>.Instance;
    private readonly ConcurrentDictionary<(string Source, string Uid), long> _external = new();
    private readonly ConcurrentDictionary<(string Source, string Uid), bool> _externalMisses = new();
    private readonly ConcurrentDictionary<long, GameMetadata> _games = new();

    public string Name => MetadataPriority.Igdb;

    public bool IsEnabled => client.IsConfigured;

    public async Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (!IsEnabled)
        {
            return [];
        }

        var results = new List<MetadataMatch>();
        if (await FindExactAsync(lookup, cancellationToken).ConfigureAwait(false) is { } exactId
            && await FetchAsync(exactId, cancellationToken).ConfigureAwait(false) is { Title: { } exactTitle })
        {
            results.Add(new MetadataMatch(Name, exactId, exactTitle) { IsExact = true });
        }

        var array = await QueryAsync("games", IgdbQueries.Search(lookup.Title, IgdbPlatforms.IdsFor(lookup.Platform)), cancellationToken).ConfigureAwait(false);
        if (array is { } found)
        {
            foreach (var item in found.EnumerateArray())
            {
                Remember(item);
                if (IgdbParser.Match(item) is { } match && !results.Exists(r => r.Id == match.Id))
                {
                    results.Add(match);
                }
            }
        }

        return results;
    }

    public async Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (!IsEnabled)
        {
            return null;
        }

        foreach (string source in ExactOrder)
        {
            if (!lookup.ExternalIds.TryGetValue(source, out string? uid) || string.IsNullOrWhiteSpace(uid))
            {
                continue;
            }

            if (!_external.ContainsKey((source, uid)) && !_externalMisses.ContainsKey((source, uid)))
            {
                await ResolveExternalAsync(source, [uid], cancellationToken).ConfigureAwait(false);
            }

            if (_external.TryGetValue((source, uid), out long id))
            {
                return IgdbClient.Number(id);
            }
        }

        return null;
    }

    public async Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken)
    {
        if (!IsEnabled || !long.TryParse(id, NumberStyles.None, CultureInfo.InvariantCulture, out long gameId))
        {
            return null;
        }

        if (!_games.ContainsKey(gameId))
        {
            await FetchGamesAsync([gameId], cancellationToken).ConfigureAwait(false);
        }

        return _games.TryGetValue(gameId, out var metadata) ? metadata with { FetchedAt = time.GetUtcNow() } : null;
    }

    /// <summary>Lote: uma consulta de <c>external_games</c> por loja e uma de <c>games</c> a cada 500 jogos.</summary>
    public async Task PrefetchAsync(IReadOnlyCollection<MetadataLookup> lookups, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookups);
        if (!IsEnabled)
        {
            return;
        }

        foreach (string source in ExactOrder)
        {
            var uids = lookups
                .Select(l => l.ExternalIds.GetValueOrDefault(source))
                .Where(uid => !string.IsNullOrWhiteSpace(uid) && !_external.ContainsKey((source, uid)) && !_externalMisses.ContainsKey((source, uid)))
                .OfType<string>()
                .Distinct(StringComparer.Ordinal)
                .ToList();
            foreach (var chunk in uids.Chunk(IgdbQueries.MaxLimit / 2))
            {
                await ResolveExternalAsync(source, chunk, cancellationToken).ConfigureAwait(false);
            }
        }

        var missing = _external.Values.Where(id => !_games.ContainsKey(id)).Distinct().ToList();
        foreach (var chunk in missing.Chunk(IgdbQueries.MaxLimit))
        {
            await FetchGamesAsync(chunk, cancellationToken).ConfigureAwait(false);
        }
    }

    private async Task ResolveExternalAsync(string source, string[] uids, CancellationToken cancellationToken)
    {
        if (!IgdbQueries.ExternalSources.TryGetValue(source, out int sourceId) || uids.Length == 0)
        {
            return;
        }

        if (await QueryAsync("external_games", IgdbQueries.ExternalGames(sourceId, uids), cancellationToken).ConfigureAwait(false) is not { } array)
        {
            return; // falha: tenta de novo na próxima atualização
        }

        var found = IgdbParser.ExternalGames(array);
        foreach (string uid in uids)
        {
            if (found.TryGetValue(uid, out long id))
            {
                _external[(source, uid)] = id;
            }
            else
            {
                _externalMisses[(source, uid)] = true;
            }
        }
    }

    private async Task FetchGamesAsync(long[] ids, CancellationToken cancellationToken)
    {
        if (await QueryAsync("games", IgdbQueries.GamesById(ids), cancellationToken).ConfigureAwait(false) is { } array)
        {
            foreach (var item in array.EnumerateArray())
            {
                Remember(item);
            }
        }
    }

    private void Remember(JsonElement item)
    {
        if (IgdbParser.Game(item, time.GetUtcNow()) is { } metadata && long.TryParse(metadata.SourceIds[Name], NumberStyles.None, CultureInfo.InvariantCulture, out long id))
        {
            _games[id] = metadata;
            foreach (var (source, uid) in metadata.SourceIds.Where(kv => kv.Key != Name))
            {
                _external.TryAdd((source, uid), id);
            }
        }
    }

    /// <summary>Falha da IGDB na atualização em segundo plano não interrompe nada: fica registrada e volta nula.</summary>
    private async Task<JsonElement?> QueryAsync(string endpoint, string body, CancellationToken cancellationToken)
    {
        try
        {
            return await client.QueryAsync(endpoint, body, cancellationToken).ConfigureAwait(false);
        }
        catch (IgdbException ex) when (ex.Failure != IgdbFailure.NotConfigured)
        {
            LogQueryFailed(_logger, endpoint, ex.Failure, ex.Message);
            return null;
        }
        catch (IgdbException)
        {
            return null;
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "IGDB: consulta a {Endpoint} falhou ({Failure}): {Reason}")]
    private static partial void LogQueryFailed(ILogger logger, string endpoint, IgdbFailure failure, string reason);
}
