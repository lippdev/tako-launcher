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

    public string Description { get; set; } = string.Empty;

    public string? Developer { get; set; }

    public string? Publisher { get; set; }

    /// <summary>Nota do Metacritic (0–100), quando a fonte informa.</summary>
    public int? MetacriticScore { get; set; }

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

    public bool CanPlay => IsInstalled && !IsUpcoming;
}
