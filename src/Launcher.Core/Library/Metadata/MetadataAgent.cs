namespace Launcher.Core.Library.Metadata;

/// <summary>Resultado de uma atualização: os metadados novos (nulo = nada a gravar) e se algum provedor foi consultado.</summary>
public sealed record MetadataUpdate(GameMetadata? Metadata, bool Queried);

/// <summary>
/// O "agente" (como no Plex), para N provedores: para cada jogo, consulta as fontes ligadas na ordem do tipo de jogo
/// (<see cref="MetadataPriority"/>), casa primeiro por ids (appid da Steam, ids nas lojas, ids que uma fonte informa
/// sobre a outra) e só então pelo título, e combina campo a campo (<see cref="MetadataMerge"/>). Não guarda nada: quem
/// chama lê e grava o cache.
/// </summary>
public sealed class MetadataAgent(IEnumerable<IMetadataProvider> providers, TimeProvider time)
{
    public static readonly TimeSpan MaxAge = TimeSpan.FromDays(30);
    public static readonly TimeSpan NoMatchRetry = TimeSpan.FromDays(7);

    private readonly IReadOnlyList<IMetadataProvider> _providers = [.. providers];

    /// <summary>Provedores ligados agora (a IGDB some quando não há chave).</summary>
    public IReadOnlyList<IMetadataProvider> Enabled => [.. _providers.Where(p => p.IsEnabled)];

    /// <summary>"Steam" ou "Steam e IGDB", para os textos da interface.</summary>
    public string EnabledNames => Enabled.Count switch
    {
        0 => "nenhuma fonte",
        1 => Enabled[0].Name,
        _ => string.Join(", ", Enabled.Take(Enabled.Count - 1).Select(p => p.Name)) + " e " + Enabled[^1].Name,
    };

    public bool IsEnabled(string providerName) => _providers.Any(p => p.Name == providerName && p.IsEnabled);

    /// <summary>Cache gravado antes da combinação (só Steam) vira uma combinação de uma parte só.</summary>
    public static GameMetadata? Normalize(GameMetadata? cached) => cached switch
    {
        null => null,
        { Parts.Count: 0, Consulted.Count: 0 } legacy when legacy.IsEmpty => legacy with { Consulted = [legacy.Source] },
        { Parts.Count: 0, Consulted.Count: 0 } legacy => legacy with { Parts = [legacy], Consulted = [legacy.Source] },
        _ => cached,
    };

    /// <summary>Precisa consultar: sem cache, cache velho ou uma fonte ligada que ainda não foi consultada.</summary>
    public bool NeedsUpdate(Game game, GameMetadata? cached, bool force = false)
    {
        ArgumentNullException.ThrowIfNull(game);
        cached = Normalize(cached);
        return force || cached is null || IsStale(cached) || For(MetadataPriority.KindOf(game)).Any(p => !IsPaused(p) && !cached.Consulted.Contains(p.Name));
    }

    /// <summary>Lote antes da rodada: provedores que sabem resolver muitos jogos de uma vez (IGDB).</summary>
    public async Task PrefetchAsync(IReadOnlyCollection<Game> games, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(games);
        var lookups = games.Select(MetadataLookup.For).Where(l => l.ExternalIds.Count > 0).ToList();
        if (lookups.Count == 0)
        {
            return;
        }

        foreach (var batch in Enabled.OfType<IBatchMetadataProvider>())
        {
            await batch.PrefetchAsync(lookups, cancellationToken).ConfigureAwait(false);
        }
    }

    public async Task<MetadataUpdate> UpdateAsync(Game game, GameMetadata? cached, bool force, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(game);
        cached = Normalize(cached);
        if (!NeedsUpdate(game, cached, force))
        {
            return new MetadataUpdate(null, false);
        }

        bool pinned = cached?.IsPinned == true;
        bool refreshAll = force || cached is null || IsStale(cached);
        var kind = MetadataPriority.KindOf(game);
        var enabled = For(kind);
        var keep = refreshAll ? [] : cached!.Parts.Where(p => enabled.Any(e => e.Name == p.Source)).ToList();
        var consulted = refreshAll ? [] : cached!.Consulted.Where(name => enabled.Any(e => e.Name == name)).ToList();
        var ids = new Dictionary<string, string>(ExternalGameIds.For(game), StringComparer.Ordinal);
        if (pinned)
        {
            foreach (var (source, id) in cached!.SourceIds)
            {
                ids[source] = id; // escolha do usuário vale mais que o appid da loja
            }
        }

        foreach (var part in keep)
        {
            Learn(ids, part);
        }

        var parts = new List<GameMetadata>(keep);
        int before = consulted.Count;
        bool queried = false;
        foreach (var provider in enabled.Where(p => !consulted.Contains(p.Name) && !IsPaused(p)))
        {
            queried = true;
            var outcome = await ConsultAsync(provider, game, ids, titleSearch: !pinned, cancellationToken).ConfigureAwait(false);
            if (outcome.Transient || IsPaused(provider))
            {
                // Limite da API, fora do ar ou provedor pausado durante a consulta (cota do dia do ScreenScraper): fica
                // para a próxima rodada, sem marcar como "sem correspondência".
                continue;
            }

            consulted.Add(provider.Name);
            if (outcome.Metadata is { } metadata)
            {
                parts.Add(metadata);
                Learn(ids, metadata);
            }
        }

        if (consulted.Count == before)
        {
            return new MetadataUpdate(null, queried);
        }

        // Só parte das fontes consultada agora: o prazo de validade continua o da rodada completa anterior.
        var fetchedAt = refreshAll ? time.GetUtcNow() : cached!.FetchedAt;
        var combined = MetadataMerge.Combine(parts, kind, fetchedAt, consulted) with { IsPinned = pinned };
        return new MetadataUpdate(combined, queried);
    }

    /// <summary>"Corrigir correspondência": resultados de todas as fontes ligadas, na ordem do tipo de jogo.</summary>
    public async Task<IReadOnlyList<MetadataMatch>> SearchAsync(Game game, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(game);
        var lookup = MetadataLookup.For(game);
        var results = new List<MetadataMatch>();
        foreach (var provider in For(MetadataPriority.KindOf(game)).Where(p => !IsPaused(p)))
        {
            results.AddRange(await SearchProviderAsync(provider, lookup, cancellationToken).ConfigureAwait(false));
        }

        return results;
    }

    /// <summary>Correspondência escolhida pelo usuário: os dados dela e, pelos ids que ela traz, os das outras fontes.</summary>
    public async Task<GameMetadata?> PinAsync(Game game, MetadataMatch match, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(game);
        ArgumentNullException.ThrowIfNull(match);
        if (_providers.FirstOrDefault(p => p.Name == match.Source) is not { } chosen
            || await chosen.FetchAsync(match.Id, cancellationToken).ConfigureAwait(false) is not { } metadata)
        {
            return null;
        }

        var kind = MetadataPriority.KindOf(game);
        var ids = new Dictionary<string, string>(StringComparer.Ordinal) { [chosen.Name] = match.Id };
        Learn(ids, metadata);
        var parts = new List<GameMetadata> { metadata };
        var consulted = new List<string> { chosen.Name };
        foreach (var provider in For(kind).Where(p => p != chosen && !IsPaused(p)))
        {
            var outcome = await ConsultAsync(provider, game, ids, titleSearch: false, cancellationToken).ConfigureAwait(false);
            if (!outcome.Transient && !IsPaused(provider))
            {
                consulted.Add(provider.Name);
            }

            if (outcome.Metadata is { } other)
            {
                parts.Add(other);
                Learn(ids, other);
            }
        }

        var combined = MetadataMerge.Combine(parts, kind, time.GetUtcNow(), consulted);
        var pinnedIds = new Dictionary<string, string>(combined.SourceIds, StringComparer.Ordinal) { [chosen.Name] = match.Id };
        return combined with { IsPinned = true, SourceIds = pinnedIds };
    }

    /// <summary>
    /// Fontes ligadas que atendem o tipo de jogo, na ordem dele. O ScreenScraper só atende ROMs (ADR-0022): consultá-lo
    /// para jogos de PC gastaria a cota diária do usuário sem ganho.
    /// </summary>
    private IReadOnlyList<IMetadataProvider> For(MetadataKind kind) =>
        MetadataPriority.Sort(Enabled.Where(p => MetadataPriority.Applies(p.Name, kind)), p => p.Name, MetadataPriority.Order(kind));

    /// <summary>Pausado (ex.: cota do dia): não é consultado agora, mas os dados que já trouxe continuam na combinação.</summary>
    private static bool IsPaused(IMetadataProvider provider) => provider is IPausableMetadataProvider { IsPaused: true };

    private bool IsStale(GameMetadata cached) => time.GetUtcNow() - cached.FetchedAt >= (cached.IsEmpty ? NoMatchRetry : MaxAge);

    private static void Learn(Dictionary<string, string> ids, GameMetadata metadata)
    {
        foreach (var (source, id) in metadata.SourceIds)
        {
            ids.TryAdd(source, id);
        }
    }

    private static async Task<(GameMetadata? Metadata, bool Transient)> ConsultAsync(IMetadataProvider provider, Game game, Dictionary<string, string> ids, bool titleSearch, CancellationToken cancellationToken)
    {
        var lookup = MetadataLookup.For(game) with { ExternalIds = new Dictionary<string, string>(ids, StringComparer.Ordinal) };
        string? id = ids.GetValueOrDefault(provider.Name)
            ?? await provider.FindExactAsync(lookup, cancellationToken).ConfigureAwait(false);
        if (id is null && titleSearch)
        {
            var results = await SearchProviderAsync(provider, lookup, cancellationToken).ConfigureAwait(false);
            id = (results.FirstOrDefault(r => r.IsExact) ?? TitleMatcher.Best(game.Title, results))?.Id;
        }

        if (id is null)
        {
            return (null, false); // sem correspondência nesta fonte
        }

        var metadata = await provider.FetchAsync(id, cancellationToken).ConfigureAwait(false);
        return (metadata, metadata is null);
    }

    /// <summary>Busca pelo título; sem resultados, tenta o título limpo (a loja não acha "… GOTY", por exemplo).</summary>
    private static async Task<IReadOnlyList<MetadataMatch>> SearchProviderAsync(IMetadataProvider provider, MetadataLookup lookup, CancellationToken cancellationToken)
    {
        foreach (string term in TitleMatcher.SearchTerms(lookup.Title))
        {
            var results = await provider.SearchAsync(lookup with { Title = term }, cancellationToken).ConfigureAwait(false);
            if (results.Count > 0)
            {
                return results;
            }
        }

        return [];
    }
}
