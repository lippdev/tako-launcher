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

    public Uri? CoverUri { get; init; }

    public Uri? HeroUri { get; init; }

    /// <summary>Ponto de enquadramento da arte de fundo, como "50% 40%" (x y).</summary>
    public string? HeroAlignment { get; init; }

    public IReadOnlyList<string> Genres { get; init; } = [];

    public string Description { get; init; } = string.Empty;

    /// <summary>Aviso exibido nos detalhes (ex.: port não oficial).</summary>
    public string? Note { get; init; }

    public DateOnly? ReleaseDate { get; init; }

    public DateOnly? UpcomingRelease { get; init; }

    public bool IsInstalled { get; init; }

    public DateTimeOffset AddedAt { get; init; }

    public bool IsFavorite { get; set; }

    public DateTimeOffset? LastPlayedAt { get; set; }

    public TimeSpan PlayTime { get; set; }

    public bool IsUpcoming => UpcomingRelease is not null;

    public bool CanPlay => IsInstalled && !IsUpcoming;
}
