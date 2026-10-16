using System.Globalization;

namespace Launcher.Core.Library.Metadata.Art;

/// <summary>Tipo de imagem do jogo que o usuário pode escolher (ADR-0023): uma aba por tipo no seletor.</summary>
public enum ArtKind
{
    /// <summary>Capa vertical (2:3) da grade.</summary>
    Cover,

    /// <summary>Arte de fundo (paisagem) dos detalhes e do destaque.</summary>
    Hero,

    /// <summary>Logo sem fundo.</summary>
    Logo,

    /// <summary>Ícone quadrado.</summary>
    Icon,
}

/// <summary>Nomes e chaves dos tipos de imagem.</summary>
public static class ArtKinds
{
    public static IReadOnlyList<ArtKind> All { get; } = [ArtKind.Cover, ArtKind.Hero, ArtKind.Logo, ArtKind.Icon];

    /// <summary>"Capa", "Fundo", "Logo", "Ícone".</summary>
    public static string Label(ArtKind kind) => kind switch
    {
        ArtKind.Cover => "Capa",
        ArtKind.Hero => "Fundo",
        ArtKind.Logo => "Logo",
        _ => "Ícone",
    };

    /// <summary>Chave estável no banco (<c>game_art.kind</c>).</summary>
    public static string Key(ArtKind kind) => kind switch
    {
        ArtKind.Cover => "cover",
        ArtKind.Hero => "hero",
        ArtKind.Logo => "logo",
        _ => "icon",
    };

    public static ArtKind? FromKey(string? key) => key switch
    {
        "cover" => ArtKind.Cover,
        "hero" => ArtKind.Hero,
        "logo" => ArtKind.Logo,
        "icon" => ArtKind.Icon,
        _ => null,
    };
}

/// <summary>De onde vêm as imagens do seletor (ADR-0023). Os nomes são os mesmos dos provedores de metadados.</summary>
public static class ArtSources
{
    public const string Steam = SteamStoreProvider.SourceName;
    public const string Igdb = MetadataPriority.Igdb;
    public const string ScreenScraper = MetadataPriority.ScreenScraper;
    public const string SteamGridDb = MetadataPriority.SteamGridDb;

    /// <summary>Imagens do próprio PC (Imagens, Downloads, Área de Trabalho ou qualquer arquivo).</summary>
    public const string File = "Arquivo do PC";

    /// <summary>Ordem das abas de fonte no seletor.</summary>
    public static IReadOnlyList<string> Order { get; } = [Steam, Igdb, ScreenScraper, SteamGridDb, File];

    /// <summary>Tipos que cada fonte costuma ter (a aba some quando a fonte não tem o tipo).</summary>
    public static bool Offers(string source, ArtKind kind) => source switch
    {
        Igdb => kind is ArtKind.Cover or ArtKind.Hero,
        ScreenScraper => kind is ArtKind.Cover or ArtKind.Hero or ArtKind.Logo,
        _ => true,
    };
}

/// <summary>
/// Uma imagem que o usuário pode escolher para o jogo: endereço completo (aplicado e baixado só ao escolher ou na
/// prévia), miniatura para a grade, tamanho, rótulo e o autor (SteamGridDB pede o crédito).
/// </summary>
public sealed record ArtCandidate(string Source, ArtKind Kind, Uri Url)
{
    /// <summary>Imagem menor para a grade; nula = a própria <see cref="Url"/>.</summary>
    public Uri? Thumbnail { get; init; }

    public int? Width { get; init; }

    public int? Height { get; init; }

    /// <summary>O que é a imagem ("Caixa 2D · EUA", "Arte oficial", "Capa da biblioteca (2x)").</summary>
    public string? Label { get; init; }

    /// <summary>Quem fez a imagem (SteamGridDB).</summary>
    public string? Author { get; init; }

    /// <summary>Estilo no SteamGridDB (alternate, blurred, material, no_logo, white_logo, official...).</summary>
    public string? Style { get; init; }

    public bool IsAnimated { get; init; }

    public bool IsNsfw { get; init; }

    public bool IsHumor { get; init; }

    /// <summary>Pontuação da comunidade (SteamGridDB), para ordenar.</summary>
    public int Score { get; init; }

    public Uri Preview => Thumbnail ?? Url;

    /// <summary>"600 × 900", quando a fonte informa.</summary>
    public string? SizeText => Width is int w && Height is int h ? string.Create(CultureInfo.InvariantCulture, $"{w} × {h}") : null;

    /// <summary>Linha da prévia: "Caixa 2D · EUA · 600 × 900 · por Fulano".</summary>
    public string Caption => string.Join(" · ", new[] { Label, SizeText, Author is null ? null : $"por {Author}" }.Where(p => !string.IsNullOrWhiteSpace(p)));
}
