using System.IO;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Art;
using Launcher.Core.Library.Metadata.Igdb;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.Core.Library.Metadata.SteamGridDb;

namespace Launcher.UI.Wpf.Services;

/// <summary>O que o seletor pede: as imagens de uma fonte, de um tipo, para o jogo (ADR-0023).</summary>
public sealed record ArtQuery(Game Game, string Source, ArtKind Kind)
{
    /// <summary>Filtro do SteamGridDB (estilo, formato, animadas).</summary>
    public SteamGridDbFilter? Filter { get; init; }

    /// <summary>Outro jogo do SteamGridDB (busca livre do seletor); nulo = o casado com este jogo.</summary>
    public string? SteamGridDbGameId { get; init; }

    /// <summary>Página do SteamGridDB (rolagem infinita).</summary>
    public int Page { get; init; }
}

/// <summary>Imagens de cada fonte para o seletor. O teste de fumaça troca por um simulado: o CI nunca vai à rede.</summary>
public interface IArtCatalog
{
    /// <summary>Fontes ligadas para o jogo, na ordem das abas, com "Arquivo do PC" no fim.</summary>
    IReadOnlyList<string> Sources(Game game);

    Task<IReadOnlyList<ArtCandidate>> ListAsync(ArtQuery query, CancellationToken cancellationToken);

    /// <summary>Busca livre no SteamGridDB (outro jogo ou outro termo).</summary>
    Task<IReadOnlyList<MetadataMatch>> SearchSteamGridDbAsync(string term, CancellationToken cancellationToken);

    /// <summary>Nome do jogo do SteamGridDB em uso para este jogo (casado ou fixado), para o seletor mostrar.</summary>
    Task<MetadataMatch?> SteamGridDbGameAsync(Game game, CancellationToken cancellationToken);
}

/// <summary>
/// Imagens por fonte, usando os ids casados ou fixados em "Fontes e IDs": Steam pelo appid (capa 2x, fundo, logo,
/// ícone), IGDB (capa, artes, capturas), ScreenScraper (caixas, fanart, capturas e logos por região), SteamGridDB (com o
/// filtro) e imagens recentes do PC. Downloads só quando a miniatura aparece (cache do <see cref="ArtService"/>).
/// </summary>
public sealed class ArtCatalog(
    MetadataService metadata,
    IgdbProvider igdb,
    ScreenScraperProvider screenScraper,
    SteamGridDbProvider steamGridDb) : IArtCatalog
{
    public IReadOnlyList<string> Sources(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        var ids = metadata.KnownIds(game);
        var list = new List<string>();
        if (ids.ContainsKey(ArtSources.Steam) || game.SteamAppId is not null)
        {
            list.Add(ArtSources.Steam);
        }

        if (igdb.IsEnabled)
        {
            list.Add(ArtSources.Igdb);
        }

        if (screenScraper.IsEnabled && game.IsRom)
        {
            list.Add(ArtSources.ScreenScraper);
        }

        if (steamGridDb.IsEnabled)
        {
            list.Add(ArtSources.SteamGridDb);
        }

        list.Add(ArtSources.File);
        return list;
    }

    public async Task<IReadOnlyList<ArtCandidate>> ListAsync(ArtQuery query, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(query);
        var ids = metadata.KnownIds(query.Game);
        switch (query.Source)
        {
            case ArtSources.Steam:
                int? appId = ids.TryGetValue(ArtSources.Steam, out string? steam) && int.TryParse(steam, out int parsed) ? parsed : query.Game.SteamAppId;
                return appId is int id ? [.. SteamArtCandidates.For(id).Where(c => c.Kind == query.Kind)] : [];
            case ArtSources.Igdb when ids.TryGetValue(ArtSources.Igdb, out string? igdbId):
                return [.. (await igdb.ImagesAsync(igdbId, cancellationToken).ConfigureAwait(true)).Where(c => c.Kind == query.Kind)];
            case ArtSources.ScreenScraper when ids.TryGetValue(ArtSources.ScreenScraper, out string? ssId):
                return [.. (await screenScraper.MediaAsync(ssId, cancellationToken).ConfigureAwait(true)).Where(c => c.Kind == query.Kind)];
            case ArtSources.SteamGridDb:
                string? gameId = query.SteamGridDbGameId ?? (await SteamGridDbGameAsync(query.Game, cancellationToken).ConfigureAwait(true))?.Id;
                return gameId is null ? [] : await steamGridDb.ImagesAsync(gameId, query.Filter ?? new SteamGridDbFilter(query.Kind), cancellationToken, query.Page).ConfigureAwait(true);
            case ArtSources.File:
                var files = await Task.Run(() => LocalImages.Find(PictureFolders()), cancellationToken).ConfigureAwait(true);
                return [.. files.Select(f => new ArtCandidate(ArtSources.File, query.Kind, new Uri(f)) { Label = Path.GetFileName(f) })];
            default:
                return [];
        }
    }

    public Task<IReadOnlyList<MetadataMatch>> SearchSteamGridDbAsync(string term, CancellationToken cancellationToken) =>
        steamGridDb.SearchTermAsync(term, cancellationToken);

    public async Task<MetadataMatch?> SteamGridDbGameAsync(Game game, CancellationToken cancellationToken)
    {
        ArgumentNullException.ThrowIfNull(game);
        var ids = metadata.KnownIds(game);
        if (ids.TryGetValue(ArtSources.SteamGridDb, out string? pinned))
        {
            return pinned.Length == 0 ? null : (await steamGridDb.SearchAsync(MetadataLookup.For(game), cancellationToken).ConfigureAwait(true)).FirstOrDefault(m => m.Id == pinned)
                ?? new MetadataMatch(ArtSources.SteamGridDb, pinned, game.Title);
        }

        var lookup = MetadataLookup.For(game) with { ExternalIds = ids };
        var results = await steamGridDb.SearchAsync(lookup, cancellationToken).ConfigureAwait(true);
        return results.FirstOrDefault(r => r.IsExact) ?? TitleMatcher.Best(game.Title, results);
    }

    /// <summary>Imagens, Downloads e Área de Trabalho do usuário.</summary>
    public static IReadOnlyList<string> PictureFolders() =>
    [
        Environment.GetFolderPath(Environment.SpecialFolder.MyPictures),
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Downloads"),
        Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory),
    ];
}
