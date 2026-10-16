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

    /// <summary>
    /// Precisa consultar: sem cache, cache velho, uma fonte ligada que ainda não foi consultada ou um id fixado pelo
    /// usuário ("Fontes e IDs", ADR-0023) diferente do que está no cache.
    /// </summary>
    public bool NeedsUpdate(Game game, GameMetadata? cached, bool force = false, IReadOnlyDictionary<string, string>? pinned = null)
    {
        ArgumentNullException.ThrowIfNull(game);
        cached = Normalize(cached);
        var sources = For(MetadataPriority.KindOf(game));
        return force || cached is null || IsStale(cached)
            || sources.Any(p => !IsPaused(p) && !cached.Consulted.Contains(p.Name))
            || PinChanged(cached, pinned, sources);
    }

    /// <summary>Fontes ligadas que atendem o jogo (as de "Fontes e IDs"), na ordem do tipo; inclui as que só completam artes.</summary>
    public IReadOnlyList<IMetadataProvider> Sources(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        var kind = MetadataPriority.KindOf(game);
        return MetadataPriority.Sort(Enabled.Where(p => MetadataPriority.Applies(p.Name, kind)), p => p.Name, MetadataPriority.Order(kind));
    }

    /// <summary>Busca o termo numa fonte só ("Fontes e IDs"); sem resultados, tenta o termo limpo.</summary>
    public async Task<IReadOnlyList<MetadataMatch>> SearchSourceAsync(Game game, string source, string term, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(game);
        if (Sources(game).FirstOrDefault(p => p.Name == source) is not { } provider || IsPaused(provider) || string.IsNullOrWhiteSpace(term))
        {
            return [];
        }

        return await SearchProviderAsync(provider, MetadataLookup.For(game) with { Title = term.Trim() }, cancellationToken).ConfigureAwait(false);
    }

    /// <summary>
    /// Id digitado ou colado → id na fonte (a IGDB resolve o nome da URL) e os dados que ele representa, para mostrar o
    /// nome antes de aplicar. Nulo quando a fonte não conhece o id.
    /// </summary>
    public async Task<(string Id, GameMetadata Metadata)?> ValidateAsync(string source, Art.SourceIdParse input, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(input);
        if (!input.IsValid || _providers.FirstOrDefault(p => p.Name == source && p.IsEnabled) is not { } provider)
        {
            return null;
        }

        string? id = input.Id;
        if (id is null && input.Slug is { } slug && provider is ISlugResolver resolver)
        {
            id = await resolver.IdFromSlugAsync(slug, cancellationToken).ConfigureAwait(false);
        }

        return id is not null && await provider.FetchAsync(id, cancellationToken).ConfigureAwait(false) is { } metadata ? (id, metadata) : null;
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

    /// <summary>
    /// Consulta as fontes que faltam. <paramref name="pinned"/> são os ids que o usuário fixou por fonte ("Fontes e IDs"):
    /// valem mais que os da loja e os aprendidos, a fonte não é buscada pelo nome e id vazio quer dizer "nenhum" (a
    /// fonte fica de fora deste jogo).
    /// </summary>
    public async Task<MetadataUpdate> UpdateAsync(Game game, GameMetadata? cached, bool force, CancellationToken cancellationToken, IReadOnlyDictionary<string, string>? pinned = null)
    {
        ArgumentNullException.ThrowIfNull(game);
        cached = Normalize(cached);
        pinned ??= new Dictionary<string, string>();
        if (!NeedsUpdate(game, cached, force, pinned))
        {
            return new MetadataUpdate(null, false);
        }

        bool legacyPinned = cached?.IsPinned == true;
        bool refreshAll = force || cached is null || IsStale(cached) || PinChanged(cached, pinned, For(MetadataPriority.KindOf(game)));
        var kind = MetadataPriority.KindOf(game);
        var enabled = For(kind);
        var keep = refreshAll ? [] : cached!.Parts.Where(p => enabled.Any(e => e.Name == p.Source)).ToList();
        var consulted = refreshAll ? [] : cached!.Consulted.Where(name => enabled.Any(e => e.Name == name)).ToList();
        var ids = new Dictionary<string, string>(ExternalGameIds.For(game), StringComparer.Ordinal);
        if (legacyPinned)
        {
            foreach (var (source, id) in cached!.SourceIds)
            {
                ids[source] = id; // escolha do usuário vale mais que o appid da loja
            }
        }

        foreach (var (source, id) in pinned.Where(kv => kv.Value.Length > 0))
        {
            ids[source] = id; // id fixado em "Fontes e IDs" vale mais que todos
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
            bool none = pinned.TryGetValue(provider.Name, out string? chosen) && chosen.Length == 0;
            bool hasArt = parts.Exists(p => p.CoverUrl is not null || p.SteamAppId is not null);
            if (none || (provider is IArtFallbackProvider && hasArt && !pinned.ContainsKey(provider.Name)))
            {
                // "Nenhum" escolhido pelo usuário, ou fonte de artes para um jogo que já tem capa: nada a consultar.
                consulted.Add(provider.Name);
                continue;
            }

            queried = true;
            var outcome = await ConsultAsync(provider, game, ids, titleSearch: !legacyPinned && !pinned.ContainsKey(provider.Name), cancellationToken).ConfigureAwait(false);
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
        var combined = MetadataMerge.Combine(parts, kind, fetchedAt, consulted) with { IsPinned = legacyPinned };
        if (pinned.Count > 0)
        {
            var sourceIds = new Dictionary<string, string>(combined.SourceIds, StringComparer.Ordinal);
            foreach (var (source, id) in pinned)
            {
                if (id.Length > 0)
                {
                    sourceIds[source] = id;
                }
                else
                {
                    sourceIds.Remove(source);
                }
            }

            combined = combined with { SourceIds = sourceIds };
        }

        return new MetadataUpdate(combined, queried);
    }

    /// <summary>"Corrigir correspondência": resultados de todas as fontes ligadas, na ordem do tipo de jogo.</summary>
    public async Task<IReadOnlyList<MetadataMatch>> SearchAsync(Game game, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(game);
        var lookup = MetadataLookup.For(game);
        var results = new List<MetadataMatch>();
        foreach (var provider in For(MetadataPriority.KindOf(game)).Where(p => !IsPaused(p) && p is not IArtFallbackProvider))
        {
            results.AddRange(await SearchProviderAsync(provider, lookup, cancellationToken).ConfigureAwait(false));
        }

        return results;
    }

    /// <summary>
    /// Fontes ligadas que atendem o tipo de jogo, na ordem dele. O ScreenScraper só atende ROMs (ADR-0022): consultá-lo
    /// para jogos de PC gastaria a cota diária do usuário sem ganho.
    /// </summary>
    private IReadOnlyList<IMetadataProvider> For(MetadataKind kind) =>
        MetadataPriority.Sort(Enabled.Where(p => MetadataPriority.Applies(p.Name, kind) && p is not IArtFallbackProvider { AutoFill: false }), p => p.Name, MetadataPriority.Order(kind));

    /// <summary>Algum id fixado pelo usuário (de uma fonte consultada) difere do cache, ou "nenhum" com dados no cache.</summary>
    private static bool PinChanged(GameMetadata cached, IReadOnlyDictionary<string, string>? pinned, IReadOnlyList<IMetadataProvider> sources) =>
        pinned is not null && pinned.Any(kv => sources.Any(p => p.Name == kv.Key && p is not IArtFallbackProvider) && (kv.Value.Length > 0
            ? cached.Parts.FirstOrDefault(p => p.Source == kv.Key)?.IdIn(kv.Key) != kv.Value
            : cached.Parts.Any(p => p.Source == kv.Key)));

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
