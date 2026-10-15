namespace Launcher.Core.Library.Metadata;

/// <summary>Dados de um jogo vindos de um provedor (como os "agentes" do Plex para filmes).</summary>
public sealed record GameMetadata
{
    public required string Source { get; init; }

    public DateTimeOffset FetchedAt { get; init; }

    /// <summary>Correspondência escolhida pelo usuário ("Corrigir correspondência"); a atualização automática não a troca.</summary>
    public bool IsPinned { get; init; }

    public int? SteamAppId { get; init; }

    public string? Title { get; init; }

    public string? Description { get; init; }

    public IReadOnlyList<string> Genres { get; init; } = [];

    public DateOnly? ReleaseDate { get; init; }

    public string? Developer { get; init; }

    public string? Publisher { get; init; }

    public int? MetacriticScore { get; init; }

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

        game.ReleaseDate = ReleaseDate ?? game.ReleaseDate;
        game.Developer = Developer ?? game.Developer;
        game.Publisher = Publisher ?? game.Publisher;
        game.MetacriticScore = MetacriticScore ?? game.MetacriticScore;
        if (SteamAppId is int appId)
        {
            game.SteamAppId = appId;
            string id = appId.ToString(System.Globalization.CultureInfo.InvariantCulture);
            if (game.CoverUri is null || game.HasFallbackArt || game.CoverUri.Scheme == Steam.SteamGames.ArtScheme)
            {
                game.CoverUri = new Uri($"{Steam.SteamGames.ArtScheme}://{id}/cover");
                game.HeroUri = new Uri($"{Steam.SteamGames.ArtScheme}://{id}/hero");
            }
        }
    }
}

/// <summary>Resultado de busca por título.</summary>
public sealed record MetadataMatch(string Title, int SteamAppId);

/// <summary>Fonte de dados de jogos. Hoje: Steam Store (sem chave). IGDB e SteamGridDB entram como provedores com chave.</summary>
public interface IMetadataProvider
{
    string Name { get; }

    Task<IReadOnlyList<MetadataMatch>> SearchAsync(string title, CancellationToken cancellationToken);

    Task<GameMetadata?> FetchAsync(int steamAppId, CancellationToken cancellationToken);
}
