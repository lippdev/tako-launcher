namespace Launcher.Core.Library;

public enum GameStore
{
    Steam,
    Epic,
    Gog,
    Xbox,
    Manual,
}

/// <summary>Jogo da biblioteca. Dados de catálogo são imutáveis; estado do usuário (favorito, tempo jogado) muda.</summary>
public sealed class Game
{
    public required string Id { get; init; }

    public required string Title { get; init; }

    public required GameStore Store { get; init; }

    public string? SizeLabel { get; init; }

    /// <summary>Cor de destaque extraída da capa (#RRGGBB).</summary>
    public required string Accent { get; init; }

    /// <summary>Capa (2:3). Muda quando os metadados encontram a arte de um jogo adicionado manualmente.</summary>
    public Uri? CoverUri { get; set; }

    public Uri? HeroUri { get; set; }

    /// <summary>A capa é só um plano B da própria loja (ex.: logo do pacote Xbox): os metadados podem trocá-la.</summary>
    public bool HasFallbackArt { get; init; }

    /// <summary>Ponto de enquadramento da arte de fundo, como "50% 40%" (x y).</summary>
    public string? HeroAlignment { get; init; }

    // Metadados: vêm da loja ou de um provedor (Library.Metadata) e podem chegar depois de o jogo aparecer.

    public IReadOnlyList<string> Genres { get; set; } = [];

    public IReadOnlyList<string> Themes { get; set; } = [];

    /// <summary>Modos de jogo (um jogador, cooperativo...).</summary>
    public IReadOnlyList<string> GameModes { get; set; } = [];

    /// <summary>Franquia ou série.</summary>
    public string? Franchise { get; set; }

    public string Description { get; set; } = string.Empty;

    public string? Developer { get; set; }

    public string? Publisher { get; set; }

    /// <summary>Nota do Metacritic (0–100), quando a fonte informa.</summary>
    public int? MetacriticScore { get; set; }

    /// <summary>Nota da crítica agregada (0–100) de outra fonte (IGDB), mostrada quando não há a do Metacritic.</summary>
    public int? CriticScore { get; set; }

    /// <summary>Aviso exibido nos detalhes (ex.: port não oficial).</summary>
    public string? Note { get; init; }

    public DateOnly? ReleaseDate { get; set; }

    public DateOnly? UpcomingRelease { get; init; }

    public bool IsInstalled { get; init; }

    /// <summary>
    /// Como abrir: URI (<c>steam://rungameid/570</c>) ou caminho do executável/atalho. Nulo = jogo de demonstração
    /// (sessão simulada, usada nos testes).
    /// </summary>
    public string? LaunchTarget { get; init; }

    public string? LaunchArguments { get; init; }

    public string? WorkingDirectory { get; init; }

    /// <summary>Pasta do jogo no disco ("Abrir pasta").</summary>
    public string? InstallPath { get; init; }

    /// <summary>Como reconhecer o fim do jogo (MVP-026); nulo = automático (<see cref="Session.GameTracking.Plan"/>).</summary>
    public Session.GameTrackingStrategy? Tracking { get; init; }

    /// <summary>
    /// Id do produto na própria loja, quando ele difere do <see cref="Id"/> (Epic: CatalogItemId; Xbox: StoreId da
    /// Microsoft Store): serve para casar os metadados sem depender do nome.
    /// </summary>
    public string? StoreProductId { get; init; }

    /// <summary>
    /// Plataforma de origem para jogos de emulador ("snes", "ps2"...; ver <see cref="Metadata.ScreenScraper.ScreenScraperSystems"/>
    /// e <see cref="Metadata.Igdb.IgdbPlatforms"/>); nula = PC.
    /// </summary>
    public string? Platform { get; init; }

    /// <summary>Arquivo da ROM (jogo de emulador), usado para casar os metadados pelo hash (ADR-0022).</summary>
    public string? RomPath { get; init; }

    /// <summary>Jogo de emulador: ainda não abre pelo Tako (o épico EMU traz os emuladores); só tem os dados.</summary>
    public bool IsRom => RomPath is not null;

    /// <summary>Número de jogadores ("1-2"), quando a fonte informa.</summary>
    public string? Players { get; set; }

    /// <summary>Outras mídias por tipo (<see cref="Metadata.GameMedia"/>): caixa 3D, logo, marquee, captura, fanart, vídeo.</summary>
    public IReadOnlyDictionary<string, Uri> Media { get; set; } = new Dictionary<string, Uri>();

    /// <summary>
    /// Artes escolhidas pelo usuário, por tipo (ADR-0023): valem mais que as automáticas (<see cref="CoverUri"/>,
    /// <see cref="HeroUri"/>, <see cref="Media"/>) e o agente de metadados nunca as troca. Vazio = tudo automático.
    /// </summary>
    public IReadOnlyDictionary<Metadata.Art.ArtKind, Uri> PinnedArt { get; set; } = new Dictionary<Metadata.Art.ArtKind, Uri>();

    /// <summary>Capa exibida: a escolhida pelo usuário ou a automática.</summary>
    public Uri? ShownCover => PinnedArt.GetValueOrDefault(Metadata.Art.ArtKind.Cover) ?? CoverUri;

    /// <summary>Arte de fundo exibida: a escolhida pelo usuário ou a automática.</summary>
    public Uri? ShownHero => PinnedArt.GetValueOrDefault(Metadata.Art.ArtKind.Hero) ?? HeroUri;

    /// <summary>Logo exibido (sem fundo), quando há.</summary>
    public Uri? ShownLogo => PinnedArt.GetValueOrDefault(Metadata.Art.ArtKind.Logo) ?? Media.GetValueOrDefault(Metadata.GameMedia.Logo);

    /// <summary>Ícone exibido, quando há.</summary>
    public Uri? ShownIcon => PinnedArt.GetValueOrDefault(Metadata.Art.ArtKind.Icon) ?? Media.GetValueOrDefault(Metadata.GameMedia.Icon);

    /// <summary>Jogo correspondente na Steam (da própria loja ou encontrado pelos metadados).</summary>
    public int? SteamAppId { get; set; }

    /// <summary>Outras lojas em que o mesmo jogo está instalado (STORE-010); cada cópia segue um item próprio.</summary>
    public IReadOnlyList<GameStore> AlsoInstalledIn { get; set; } = [];

    public DateTimeOffset AddedAt { get; init; }

    public bool IsFavorite { get; set; }

    public DateTimeOffset? LastPlayedAt { get; set; }

    public TimeSpan PlayTime { get; set; }

    /// <summary>Ids das coleções do usuário que têm este jogo (UX-003; ver <see cref="CollectionLibrary.ApplyTo"/>).</summary>
    public IReadOnlySet<string> Collections { get; set; } = new HashSet<string>();

    public bool IsUpcoming => UpcomingRelease is not null;

    public bool CanPlay => IsInstalled && !IsUpcoming && !IsRom;
}
