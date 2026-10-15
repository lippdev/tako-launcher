namespace Launcher.Core.Library;

/// <summary>Jogo adicionado pelo usuário (executável, atalho ou URI). Persistido em <see cref="LibraryStore"/>.</summary>
public sealed record ManualGameRecord
{
    public required string Id { get; init; }

    public required string Title { get; init; }

    /// <summary>Executável, atalho (.lnk/.url) ou URI.</summary>
    public required string Path { get; init; }

    public string? Arguments { get; init; }

    public string? WorkingDirectory { get; init; }

    /// <summary>Imagem local escolhida como capa (2:3).</summary>
    public string? CoverPath { get; init; }

    /// <summary>Imagem local de fundo; sem ela, usa a capa.</summary>
    public string? HeroPath { get; init; }

    public DateTimeOffset AddedAt { get; init; }

    public static ManualGameRecord Create(string title, string path, DateTimeOffset now) => new()
    {
        Id = $"manual:{Guid.NewGuid():N}",
        Title = title,
        Path = path,
        WorkingDirectory = System.IO.Path.GetDirectoryName(path),
        AddedAt = now,
    };

    public Game ToGame()
    {
        bool isUri = Uri.TryCreate(Path, UriKind.Absolute, out var uri) && !uri.IsFile;
        string? cover = CoverPath is not null && File.Exists(CoverPath) ? CoverPath : null;
        string? hero = HeroPath is not null && File.Exists(HeroPath) ? HeroPath : cover;
        return new Game
        {
            Id = Id,
            Title = Title,
            Store = GameStore.Manual,
            Accent = "#2B3A55",
            CoverUri = cover is null ? null : new Uri(cover),
            HeroUri = hero is null ? null : new Uri(hero),
            IsInstalled = isUri || File.Exists(Path),
            AddedAt = AddedAt,
            LaunchTarget = Path,
            LaunchArguments = Arguments,
            WorkingDirectory = WorkingDirectory,
            InstallPath = isUri ? null : System.IO.Path.GetDirectoryName(Path),
        };
    }
}

/// <summary>Estado do usuário por jogo (qualquer loja): favorito, última vez aberto pelo launcher e tempo registrado.</summary>
public sealed record GameStateRecord
{
    public bool IsFavorite { get; init; }

    public DateTimeOffset? LastPlayedAt { get; init; }

    public double PlayMinutes { get; init; }
}
