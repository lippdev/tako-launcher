using System.Collections.Concurrent;
using System.Globalization;
using Launcher.Core.Library.Metadata.Art;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;

namespace Launcher.Core.Library.Metadata.SteamGridDb;

/// <summary>
/// SteamGridDB (ADR-0023): capas, fundos, logos e ícones feitos pela comunidade para qualquer jogo. Casa pelo appid da
/// Steam (<c>games/steam/{appid}</c>) quando há; senão, pelo nome (<c>search/autocomplete</c>). Como fonte automática só
/// completa a capa e o fundo de jogos que as outras fontes deixaram sem (<see cref="IArtFallbackProvider"/>), e só com
/// "Usar o SteamGridDB quando faltar capa" ligado. No seletor, lista as imagens com o filtro escolhido.
/// </summary>
public sealed partial class SteamGridDbProvider(SteamGridDbClient client, TimeProvider time, ILogger<SteamGridDbProvider>? logger = null) : IArtFallbackProvider
{
    private readonly ILogger _logger = logger ?? NullLogger<SteamGridDbProvider>.Instance;
    private readonly ConcurrentDictionary<int, string?> _bySteam = new();

    public string Name => MetadataPriority.SteamGridDb;

    public bool IsEnabled => client.IsConfigured;

    public bool AutoFill => client.Settings.AutoFill;

    public async Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (!IsEnabled || string.IsNullOrWhiteSpace(lookup.Title))
        {
            return [];
        }

        var results = new List<MetadataMatch>();
        if (await FindExactAsync(lookup, cancellationToken).ConfigureAwait(false) is { } exact
            && await QueryAsync(SteamGridDbQueries.ById(exact), cancellationToken).ConfigureAwait(false) is { } game)
        {
            results.AddRange(SteamGridDbParser.Games(game).Select(m => m with { IsExact = true }));
        }

        if (await QueryAsync(SteamGridDbQueries.Autocomplete(lookup.Title), cancellationToken).ConfigureAwait(false) is { } found)
        {
            results.AddRange(SteamGridDbParser.Games(found).Where(m => !results.Exists(r => r.Id == m.Id)));
        }

        return results;
    }

    /// <summary>Pelo appid da Steam do jogo (da loja ou aprendido de outra fonte), sem ambiguidade.</summary>
    public async Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (!IsEnabled || !lookup.ExternalIds.TryGetValue(ExternalGameIds.Steam, out string? steam)
            || !int.TryParse(steam, NumberStyles.None, CultureInfo.InvariantCulture, out int appId))
        {
            return null;
        }

        if (_bySteam.TryGetValue(appId, out string? known))
        {
            return known;
        }

        var data = await QueryAsync(SteamGridDbQueries.BySteamAppId(appId), cancellationToken).ConfigureAwait(false);
        string? id = data is { } game && SteamGridDbParser.Games(game) is { Count: > 0 } games ? games[0].Id : null;
        _bySteam[appId] = id;
        return id;
    }

    /// <summary>Nome do jogo e a melhor capa vertical e o melhor fundo estáticos (sem NSFW/humor).</summary>
    public async Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken)
    {
        if (!IsEnabled || string.IsNullOrWhiteSpace(id) || await QueryAsync(SteamGridDbQueries.ById(id), cancellationToken).ConfigureAwait(false) is not { } game
            || SteamGridDbParser.Games(game) is not { Count: > 0 } games)
        {
            return null;
        }

        var match = games[0];

        var cover = SteamGridDbQueries.BestAutomatic(await ImagesAsync(id, new SteamGridDbFilter(ArtKind.Cover), cancellationToken).ConfigureAwait(false), ArtKind.Cover);
        var hero = SteamGridDbQueries.BestAutomatic(await ImagesAsync(id, new SteamGridDbFilter(ArtKind.Hero), cancellationToken).ConfigureAwait(false), ArtKind.Hero);
        return new GameMetadata
        {
            Source = Name,
            FetchedAt = time.GetUtcNow(),
            SourceIds = new Dictionary<string, string>(StringComparer.Ordinal) { [Name] = id },
            Title = match.Title,
            ReleaseDate = match.Year is int year ? new DateOnly(year, 1, 1) : null,
            CoverUrl = cover?.Url,
            BackgroundUrl = hero?.Url,
        };
    }

    /// <summary>Imagens do jogo com o filtro do seletor (lista vazia se o SteamGridDB não tiver ou falhar).</summary>
    public async Task<IReadOnlyList<ArtCandidate>> ImagesAsync(string gameId, SteamGridDbFilter filter, CancellationToken cancellationToken, int page = 0)
    {
        ArgumentNullException.ThrowIfNull(filter);
        var data = await QueryAsync(SteamGridDbQueries.Images(gameId, filter, page), cancellationToken).ConfigureAwait(false);
        return data is { } list ? SteamGridDbQueries.Apply(SteamGridDbParser.Images(list, filter.Kind), filter) : [];
    }

    /// <summary>Busca livre por nome (campo de busca do seletor): outro jogo ou outro termo.</summary>
    public async Task<IReadOnlyList<MetadataMatch>> SearchTermAsync(string term, CancellationToken cancellationToken) =>
        IsEnabled && !string.IsNullOrWhiteSpace(term) && await QueryAsync(SteamGridDbQueries.Autocomplete(term), cancellationToken).ConfigureAwait(false) is { } found
            ? SteamGridDbParser.Games(found)
            : [];

    /// <summary>Falha em segundo plano não interrompe nada: fica registrada e volta nula.</summary>
    private async Task<System.Text.Json.JsonElement?> QueryAsync(string path, CancellationToken cancellationToken)
    {
        try
        {
            return await client.GetAsync(path, cancellationToken).ConfigureAwait(false);
        }
        catch (SteamGridDbException ex)
        {
            LogQueryFailed(_logger, path, ex.Failure, ex.Message);
            return null;
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "SteamGridDB: consulta a {Path} falhou ({Failure}): {Reason}")]
    private static partial void LogQueryFailed(ILogger logger, string path, SteamGridDbFailure failure, string reason);
}
