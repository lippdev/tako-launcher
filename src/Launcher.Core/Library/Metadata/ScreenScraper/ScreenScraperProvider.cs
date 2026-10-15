using System.Collections.Concurrent;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;

namespace Launcher.Core.Library.Metadata.ScreenScraper;

/// <summary>
/// Provedor ScreenScraper (ADR-0022): jogos de emulador (ROMs). Casa primeiro pelo hash da ROM (<c>jeuInfos.php</c> com
/// CRC/MD5/SHA1, tamanho, nome e sistema), sem ambiguidade; senão, pelo nome no sistema (<c>jeuRecherche.php</c>). Só
/// atende jogos com <see cref="Game.Platform"/> de console: jogos de PC ficam com a Steam e a IGDB, sem gastar a cota.
/// Desligado sem as credenciais do software, sem a conta do usuário ou enquanto pausado (cota do dia, servidor fechado).
/// </summary>
public sealed partial class ScreenScraperProvider(ScreenScraperClient client, RomHashCache hashes, TimeProvider time, ILogger<ScreenScraperProvider>? logger = null) : IPausableMetadataProvider
{
    /// <summary>Resultados da busca por nome mostrados no "Corrigir correspondência".</summary>
    public const int MaxSearchResults = 8;

    private static readonly string[] DiscExtensions = [".iso", ".cue", ".bin", ".chd", ".gdi", ".cdi", ".cso", ".rvz", ".wia", ".gcz", ".gcm", ".wbfs", ".pbp", ".mdf", ".ecm", ".m3u", ".xiso"];

    private readonly ILogger _logger = logger ?? NullLogger<ScreenScraperProvider>.Instance;
    private readonly ConcurrentDictionary<string, GameMetadata> _games = new(StringComparer.Ordinal);

    // ROM já consultada nesta execução (caminho + tamanho + data) → id do jogo, ou nulo quando o ScreenScraper não a
    // conhece: a busca pelo nome logo depois não repete a consulta pelo hash.
    private readonly ConcurrentDictionary<string, string?> _roms = new(StringComparer.Ordinal);

    public string Name => MetadataPriority.ScreenScraper;

    public bool IsEnabled => client.IsConfigured;

    /// <summary>Cota do dia esgotada, servidor fechado ou sem rede: volta sozinho (<see cref="ScreenScraperClient.IsPaused"/>).</summary>
    public bool IsPaused => client.IsPaused;

    public async Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (!IsEnabled || IsPaused || ScreenScraperSystems.ForKey(lookup.Platform) is not { } system)
        {
            return [];
        }

        var results = new List<MetadataMatch>();
        if (await FindExactAsync(lookup, cancellationToken).ConfigureAwait(false) is { } exactId && _games.TryGetValue(exactId, out var exact))
        {
            results.Add(new MetadataMatch(Name, exactId, exact.Title ?? lookup.Title) { IsExact = true, Year = exact.ReleaseDate?.Year, Platforms = system.Name });
        }

        if (!string.IsNullOrWhiteSpace(lookup.Title)
            && await QueryAsync("jeuRecherche.php", [new("systemeid", ScreenScraperClient.Number(system.ScreenScraperId)), new("recherche", lookup.Title.Trim())], cancellationToken).ConfigureAwait(false) is { } response)
        {
            foreach (var match in ScreenScraperParser.Search(response).Take(MaxSearchResults))
            {
                if (!results.Exists(r => r.Id == match.Id))
                {
                    results.Add(match);
                }
            }
        }

        return results;
    }

    /// <summary>Pelo hash da ROM: lê o arquivo (fora da thread de UI, com cache) e consulta <c>jeuInfos.php</c>.</summary>
    public async Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(lookup);
        if (!IsEnabled || IsPaused || ScreenScraperSystems.ForKey(lookup.Platform) is not { } system || lookup.RomPath is not { Length: > 0 } romPath || !File.Exists(romPath))
        {
            return null;
        }

        string romKey = RomHashCache.KeyFor(new FileInfo(romPath), system);
        if (_roms.TryGetValue(romKey, out string? known))
        {
            return known;
        }

        RomIdentity identity;
        try
        {
            identity = await hashes.GetAsync(romPath, system, cancellationToken).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogRomUnreadable(_logger, Path.GetFileName(romPath), ex.Message);
            return null;
        }

        var response = await QueryAsync("jeuInfos.php", RomQuery(identity, system), cancellationToken).ConfigureAwait(false);
        if (IsPaused)
        {
            return null; // falhou (cota, rede): não conta como "não conhece"
        }

        var metadata = response is { } found ? ScreenScraperParser.GameInfo(found, time.GetUtcNow()) : null;
        string? id = metadata?.SourceIds[Name];
        if (metadata is not null && id is not null)
        {
            _games[id] = metadata;
        }

        _roms[romKey] = id;
        return id;
    }

    public async Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken)
    {
        if (!IsEnabled || string.IsNullOrWhiteSpace(id))
        {
            return null;
        }

        if (!_games.TryGetValue(id, out var metadata))
        {
            if (await QueryAsync("jeuInfos.php", [new("gameid", id)], cancellationToken).ConfigureAwait(false) is not { } response
                || ScreenScraperParser.GameInfo(response, time.GetUtcNow()) is not { } fetched)
            {
                return null;
            }

            metadata = _games[id] = fetched;
        }

        return metadata with { FetchedAt = time.GetUtcNow() };
    }

    /// <summary>
    /// Parâmetros do <c>jeuInfos.php</c> para a ROM: sistema, tipo (rom/iso), nome, tamanho e os hashes calculados
    /// (<see cref="RomHasher"/>). Puro, testado.
    /// </summary>
    public static IReadOnlyList<KeyValuePair<string, string>> RomQuery(RomIdentity identity, EmulatedSystem system)
    {
        ArgumentNullException.ThrowIfNull(identity);
        ArgumentNullException.ThrowIfNull(system);
        var query = new List<KeyValuePair<string, string>>
        {
            new("systemeid", ScreenScraperClient.Number(system.ScreenScraperId)),
            new("romtype", DiscExtensions.Contains(Path.GetExtension(identity.FileName).ToLowerInvariant()) ? "iso" : "rom"),
            new("romnom", identity.FileName),
            new("romtaille", ScreenScraperClient.Number(identity.Size)),
        };
        if (identity.Crc32 is { } crc)
        {
            query.Add(new("crc", crc));
        }

        if (identity.Md5 is { } md5)
        {
            query.Add(new("md5", md5));
        }

        if (identity.Sha1 is { } sha1)
        {
            query.Add(new("sha1", sha1));
        }

        return query;
    }

    /// <summary>Falha em segundo plano não interrompe nada: fica registrada, o cliente pausa e o resultado volta nulo.</summary>
    private async Task<JsonElement?> QueryAsync(string endpoint, IReadOnlyList<KeyValuePair<string, string>> query, CancellationToken cancellationToken)
    {
        try
        {
            return await client.GetAsync(endpoint, query, cancellationToken).ConfigureAwait(false);
        }
        catch (ScreenScraperException ex) when (ex.Failure is not ScreenScraperFailure.NoSoftwareAccess and not ScreenScraperFailure.NotConfigured)
        {
            LogQueryFailed(_logger, endpoint, ex.Failure, ex.Message);
            return null;
        }
        catch (ScreenScraperException)
        {
            return null;
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "ScreenScraper: consulta a {Endpoint} falhou ({Failure}): {Reason}")]
    private static partial void LogQueryFailed(ILogger logger, string endpoint, ScreenScraperFailure failure, string reason);

    [LoggerMessage(Level = LogLevel.Warning, Message = "ScreenScraper: não foi possível ler a ROM {File}: {Reason}")]
    private static partial void LogRomUnreadable(ILogger logger, string file, string reason);
}
