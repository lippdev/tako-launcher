using System.Globalization;

namespace Launcher.Core.Library.Metadata;

/// <summary>Ids do jogo em lojas, usados pelos provedores para casar sem depender do nome.</summary>
public static class ExternalGameIds
{
    public const string Steam = "Steam";
    public const string Gog = "GOG";
    public const string Epic = "Epic";
    public const string Microsoft = "Microsoft";

    public static IReadOnlyDictionary<string, string> For(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        var ids = new Dictionary<string, string>(StringComparer.Ordinal);
        // Só o appid da própria Steam: o de outras lojas pode ter vindo de uma correspondência pelo nome, que a
        // atualização precisa poder refazer.
        if (game.Store == GameStore.Steam && game.SteamAppId is int appId)
        {
            ids[Steam] = appId.ToString(CultureInfo.InvariantCulture);
        }

        switch (game.Store)
        {
            case GameStore.Gog when game.Id.StartsWith("gog:", StringComparison.Ordinal):
                ids[Gog] = game.Id["gog:".Length..];
                break;
            case GameStore.Epic when !string.IsNullOrWhiteSpace(game.StoreProductId):
                ids[Epic] = game.StoreProductId;
                break;
            case GameStore.Xbox when !string.IsNullOrWhiteSpace(game.StoreProductId):
                ids[Microsoft] = game.StoreProductId;
                break;
        }

        return ids;
    }
}

/// <summary>Tipo de jogo para decidir a ordem das fontes.</summary>
public enum MetadataKind
{
    /// <summary>Jogo da loja Steam (tem appid próprio).</summary>
    Steam,

    /// <summary>Jogo de PC de outra loja (Epic, GOG, Xbox) ou adicionado pelo usuário.</summary>
    Pc,

    /// <summary>Jogo de console rodando em emulador (ROM).</summary>
    Emulated,
}

/// <summary>Campos que a combinação decide um a um.</summary>
public enum MetadataField
{
    Description,
    Genres,
    Themes,
    GameModes,
    Franchise,
    ReleaseDate,
    Developer,
    Publisher,
    Score,
    Art,
}

/// <summary>
/// Ordem das fontes por tipo de jogo (ADR-0021), genérica para N provedores: a Steam manda nos jogos da Steam, a IGDB
/// nos de outras lojas de PC e, quando existir, o ScreenScraper nas ROMs. Provedores fora da lista entram no fim, na
/// ordem em que foram registrados.
/// </summary>
public static class MetadataPriority
{
    public const string Igdb = "IGDB";

    /// <summary>Provedor de ROMs previsto (mídia de emulação por hash); ainda não implementado.</summary>
    public const string ScreenScraper = "ScreenScraper";

    public static MetadataKind KindOf(Game game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return !string.IsNullOrEmpty(game.Platform) && !string.Equals(game.Platform, "pc", StringComparison.OrdinalIgnoreCase) ? MetadataKind.Emulated
            : game.Store == GameStore.Steam ? MetadataKind.Steam
            : MetadataKind.Pc;
    }

    /// <summary>Ordem geral das fontes para o tipo de jogo.</summary>
    public static IReadOnlyList<string> Order(MetadataKind kind) => kind switch
    {
        MetadataKind.Steam => [SteamStoreProvider.SourceName, Igdb, ScreenScraper],
        MetadataKind.Emulated => [ScreenScraper, Igdb, SteamStoreProvider.SourceName],
        _ => [Igdb, SteamStoreProvider.SourceName, ScreenScraper],
    };

    /// <summary>
    /// Ordem para um campo. Exceção: em jogos de PC, a arte da Steam vem primeiro quando existe (capa 600×900 e fundo
    /// 3840×1240, maiores que os da IGDB). A descrição segue a ordem geral, mas a combinação prefere português.
    /// </summary>
    public static IReadOnlyList<string> Order(MetadataKind kind, MetadataField field) => (kind, field) switch
    {
        (MetadataKind.Pc, MetadataField.Art) => [SteamStoreProvider.SourceName, Igdb, ScreenScraper],
        _ => Order(kind),
    };

    /// <summary>Ordena provedores pela prioridade do tipo; desconhecidos vão para o fim, sem perder a ordem original.</summary>
    public static IReadOnlyList<T> Sort<T>(IEnumerable<T> items, Func<T, string> name, IReadOnlyList<string> order)
    {
        ArgumentNullException.ThrowIfNull(items);
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(order);
        return [.. items.Select((item, index) => (item, index))
            .OrderBy(x => order.IndexOf(name(x.item)) is int i and >= 0 ? i : order.Count)
            .ThenBy(x => x.index)
            .Select(x => x.item)];
    }

    private static int IndexOf(this IReadOnlyList<string> list, string value)
    {
        for (int i = 0; i < list.Count; i++)
        {
            if (string.Equals(list[i], value, StringComparison.Ordinal))
            {
                return i;
            }
        }

        return -1;
    }
}

/// <summary>Combina o resultado de vários provedores, campo a campo, pela prioridade do tipo de jogo (regra pura, testada).</summary>
public static class MetadataMerge
{
    public static GameMetadata Combine(IReadOnlyList<GameMetadata> parts, MetadataKind kind, DateTimeOffset now, IReadOnlyList<string>? consulted = null)
    {
        ArgumentNullException.ThrowIfNull(parts);
        var found = parts.Where(p => !p.IsEmpty).ToList();
        IReadOnlyList<GameMetadata> By(MetadataField field) => MetadataPriority.Sort(found, p => p.Source, MetadataPriority.Order(kind, field));
        T? First<T>(MetadataField field, Func<GameMetadata, T?> pick) where T : class => By(field).Select(pick).FirstOrDefault(v => v is not null);
        T? FirstValue<T>(MetadataField field, Func<GameMetadata, T?> pick) where T : struct => By(field).Select(pick).FirstOrDefault(v => v is not null);
        IReadOnlyList<string> FirstList(MetadataField field, Func<GameMetadata, IReadOnlyList<string>> pick) => By(field).Select(pick).FirstOrDefault(l => l.Count > 0) ?? [];

        // Descrição: a primeira em português na ordem; senão, a primeira que houver.
        var withText = By(MetadataField.Description).Where(p => !string.IsNullOrWhiteSpace(p.Description)).ToList();
        var description = withText.FirstOrDefault(p => p.DescriptionLanguage?.StartsWith("pt", StringComparison.OrdinalIgnoreCase) == true) ?? withText.FirstOrDefault();

        // Arte: capa e fundo vêm da mesma fonte para combinar entre si.
        var art = By(MetadataField.Art).FirstOrDefault(p => p.CoverUrl is not null || p.SteamAppId is not null);

        var ids = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var part in MetadataPriority.Sort(found, p => p.Source, MetadataPriority.Order(kind)).Reverse())
        {
            foreach (var (source, id) in part.SourceIds)
            {
                ids[source] = id; // a fonte de maior prioridade escreve por último e vence
            }
        }

        int? steamAppId = found.Select(p => p.SteamAppId).FirstOrDefault(v => v is not null)
            ?? (ids.TryGetValue(SteamStoreProvider.SourceName, out string? steam) && int.TryParse(steam, NumberStyles.None, CultureInfo.InvariantCulture, out int parsed) ? parsed : null);
        var order = MetadataPriority.Sort(found, p => p.Source, MetadataPriority.Order(kind));
        return new GameMetadata
        {
            Source = found.Count == 0 ? string.Join(" + ", consulted ?? []) : string.Join(" + ", order.Select(p => p.Source)),
            FetchedAt = now,
            SteamAppId = steamAppId,
            SourceIds = ids,
            Consulted = consulted ?? [.. parts.Select(p => p.Source).Distinct()],
            Parts = [.. parts.Select(p => p with { Parts = [] })],
            Title = order.Select(p => p.Title).FirstOrDefault(t => t is not null),
            Description = description?.Description,
            DescriptionLanguage = description?.DescriptionLanguage,
            Genres = FirstList(MetadataField.Genres, p => p.Genres),
            Themes = FirstList(MetadataField.Themes, p => p.Themes),
            GameModes = FirstList(MetadataField.GameModes, p => p.GameModes),
            Franchise = First(MetadataField.Franchise, p => p.Franchise),
            ReleaseDate = FirstValue(MetadataField.ReleaseDate, p => p.ReleaseDate),
            Developer = First(MetadataField.Developer, p => p.Developer),
            Publisher = First(MetadataField.Publisher, p => p.Publisher),
            MetacriticScore = FirstValue(MetadataField.Score, p => p.MetacriticScore),
            CriticScore = FirstValue(MetadataField.Score, p => p.CriticScore),
            CoverUrl = art?.CoverUrl ?? (art?.SteamAppId is int coverId ? GameMetadata.SteamArt(coverId, "cover") : null),
            BackgroundUrl = art?.BackgroundUrl ?? (art?.SteamAppId is int heroId ? GameMetadata.SteamArt(heroId, "hero") : null),
        };
    }
}
