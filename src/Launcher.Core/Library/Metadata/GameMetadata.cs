using System.Globalization;

namespace Launcher.Core.Library.Metadata;

/// <summary>
/// Dados de um jogo vindos de um provedor (como os "agentes" do Plex para filmes) ou da combinação de vários
/// (<see cref="MetadataMerge"/>). Também é o formato do cache em disco, por jogo.
/// </summary>
public sealed record GameMetadata
{
    /// <summary>Provedor ("Steam", "IGDB") ou, na combinação, os provedores que contribuíram ("Steam + IGDB").</summary>
    public required string Source { get; init; }

    public DateTimeOffset FetchedAt { get; init; }

    /// <summary>Correspondência escolhida pelo usuário ("Corrigir correspondência"); a atualização automática não a troca.</summary>
    public bool IsPinned { get; init; }

    public int? SteamAppId { get; init; }

    /// <summary>
    /// Id do jogo em cada fonte ("Steam" → "570", "IGDB" → "1942") e, quando o provedor informa, em outras lojas
    /// ("GOG", "Epic", "Microsoft"): é o que permite casar entre fontes sem depender do nome.
    /// </summary>
    public IReadOnlyDictionary<string, string> SourceIds { get; init; } = new Dictionary<string, string>();

    /// <summary>Provedores consultados para esta combinação (mesmo os que não acharam nada).</summary>
    public IReadOnlyList<string> Consulted { get; init; } = [];

    /// <summary>Resultado de cada provedor, guardado para recombinar sem consultar de novo (só na combinação).</summary>
    public IReadOnlyList<GameMetadata> Parts { get; init; } = [];

    public string? Title { get; init; }

    public string? Description { get; init; }

    /// <summary>Idioma da descrição ("pt-BR", "en"); a combinação prefere português.</summary>
    public string? DescriptionLanguage { get; init; }

    public IReadOnlyList<string> Genres { get; init; } = [];

    public IReadOnlyList<string> Themes { get; init; } = [];

    /// <summary>Modos de jogo (um jogador, cooperativo...).</summary>
    public IReadOnlyList<string> GameModes { get; init; } = [];

    /// <summary>Franquia ou série.</summary>
    public string? Franchise { get; init; }

    public DateOnly? ReleaseDate { get; init; }

    public string? Developer { get; init; }

    public string? Publisher { get; init; }

    public int? MetacriticScore { get; init; }

    /// <summary>Nota da crítica agregada (0–100) de fontes que não são o Metacritic (ex.: IGDB).</summary>
    public int? CriticScore { get; init; }

    /// <summary>Capa (2:3) por endereço; <c>steam-art://</c> ou https.</summary>
    public Uri? CoverUrl { get; init; }

    /// <summary>Arte de fundo (paisagem).</summary>
    public Uri? BackgroundUrl { get; init; }

    public bool IsEmpty => SourceIds.Count == 0 && SteamAppId is null;

    /// <summary>Id deste jogo na fonte indicada, se conhecido.</summary>
    public string? IdIn(string source) =>
        SourceIds.TryGetValue(source, out string? id) ? id
        : source == SteamStoreProvider.SourceName && SteamAppId is int appId ? appId.ToString(CultureInfo.InvariantCulture)
        : null;

    /// <summary>Preenche o jogo sem apagar o que a loja já trouxe; capa só para jogo sem capa própria.</summary>
    public void ApplyTo(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        if (!string.IsNullOrWhiteSpace(Description))
        {
            game.Description = Description;
        }

        if (Genres.Count > 0)
        {
            game.Genres = Genres;
        }

        if (Themes.Count > 0)
        {
            game.Themes = Themes;
        }

        if (GameModes.Count > 0)
        {
            game.GameModes = GameModes;
        }

        game.Franchise = Franchise ?? game.Franchise;
        game.ReleaseDate = ReleaseDate ?? game.ReleaseDate;
        game.Developer = Developer ?? game.Developer;
        game.Publisher = Publisher ?? game.Publisher;
        game.MetacriticScore = MetacriticScore ?? game.MetacriticScore;
        game.CriticScore = CriticScore ?? game.CriticScore;
        if (SteamAppId is int appId)
        {
            game.SteamAppId = appId;
        }

        // Cache antigo (só Steam) não tem endereço de arte: vale a arte da Steam pelo appid.
        var cover = CoverUrl ?? (SteamAppId is int id ? SteamArt(id, "cover") : null);
        var background = BackgroundUrl ?? (SteamAppId is int heroId ? SteamArt(heroId, "hero") : null);
        if (cover is not null && (game.CoverUri is null || game.HasFallbackArt || IsProviderArt(game.CoverUri)))
        {
            game.CoverUri = cover;
            game.HeroUri = background ?? cover;
        }
    }

    public static Uri SteamArt(int appId, string kind) =>
        new($"{Steam.SteamGames.ArtScheme}://{appId.ToString(CultureInfo.InvariantCulture)}/{kind}");

    // Arte que veio de um provedor (não do usuário nem do disco) pode ser trocada por outra de provedor.
    private static bool IsProviderArt(Uri uri) => uri.Scheme == Steam.SteamGames.ArtScheme || uri.Scheme == Uri.UriSchemeHttps;
}

/// <summary>Resultado de busca de um provedor; <see cref="Source"/> diz de onde veio ("Corrigir correspondência" mostra).</summary>
public sealed record MetadataMatch(string Source, string Id, string Title)
{
    /// <summary>Ano de lançamento, quando o provedor informa (ajuda a escolher entre homônimos).</summary>
    public int? Year { get; init; }

    /// <summary>Plataformas do resultado, quando o provedor informa ("PC", "SNES").</summary>
    public string? Platforms { get; init; }

    /// <summary>Casado por identificador (appid, id da GOG...), sem ambiguidade: dispensa a comparação de nomes.</summary>
    public bool IsExact { get; init; }

    public int? SteamAppId => Source == SteamStoreProvider.SourceName && int.TryParse(Id, NumberStyles.None, CultureInfo.InvariantCulture, out int id) ? id : null;

    public static MetadataMatch Steam(string title, int appId) =>
        new(SteamStoreProvider.SourceName, appId.ToString(CultureInfo.InvariantCulture), title);
}

/// <summary>
/// O que se sabe do jogo para procurá-lo: título, plataforma (nula = PC) e ids em lojas ("Steam" → "570",
/// "GOG" → "1207658924", "Epic", "Microsoft"), da própria loja ou aprendidos de outro provedor.
/// </summary>
public sealed record MetadataLookup(string Title, IReadOnlyDictionary<string, string> ExternalIds, string? Platform = null)
{
    public static MetadataLookup For(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return new MetadataLookup(game.Title, ExternalGameIds.For(game), game.Platform);
    }
}

/// <summary>Fonte de dados de jogos: Steam Store (sem chave), IGDB (chave da Twitch do usuário) e outras no futuro.</summary>
public interface IMetadataProvider
{
    /// <summary>Nome curto mostrado ao usuário e usado como chave nos ids e na prioridade (<see cref="MetadataPriority"/>).</summary>
    string Name { get; }

    /// <summary>Falso quando falta configuração (ex.: IGDB sem chave): o provedor é ignorado, sem erro.</summary>
    bool IsEnabled { get; }

    /// <summary>Busca pelo título (e, se o provedor souber usar, pela plataforma e pelos ids em lojas).</summary>
    Task<IReadOnlyList<MetadataMatch>> SearchAsync(MetadataLookup lookup, CancellationToken cancellationToken);

    /// <summary>Correspondência sem ambiguidade pelos ids em lojas; nulo se o provedor não sabe ou não achou.</summary>
    Task<string?> FindExactAsync(MetadataLookup lookup, CancellationToken cancellationToken);

    Task<GameMetadata?> FetchAsync(string id, CancellationToken cancellationToken);
}

/// <summary>Provedor que consegue adiantar muitos jogos de uma vez (consulta em lote) antes da atualização jogo a jogo.</summary>
public interface IBatchMetadataProvider : IMetadataProvider
{
    Task PrefetchAsync(IReadOnlyCollection<MetadataLookup> lookups, CancellationToken cancellationToken);
}
