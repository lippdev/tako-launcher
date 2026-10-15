using System.Windows.Media;

namespace Launcher.PlayniteCompat.P10.Data;

#pragma warning disable CA1822 // Bindings do WPF só leem propriedades de instância; valores fixos ficam como propriedades de instância.

/// <summary>Item com nome (plataforma, gênero, desenvolvedora...), como os do banco do Playnite.</summary>
public sealed record P10NamedItem(string Name)
{
    /// <summary>Ícone da plataforma no Playnite; o Tako não tem (nulo).</summary>
    public ImageSource? Icon => null;

    public override string ToString() => Name;
}

/// <summary>Plugin de biblioteca no Playnite (<c>LibraryPlugin.Name</c>, <c>LibraryPlugin.LibraryIcon</c>).</summary>
public sealed record P10LibraryPlugin(string Name)
{
    public ImageSource? LibraryIcon => null;

    public override string ToString() => Name;
}

/// <summary>
/// Jogo como os temas Playnite o enxergam: o subconjunto de <c>GamesCollectionViewEntry</c> usado pelos temas
/// homologados (MVP-018), com os mesmos nomes de propriedades. Só leitura para o tema; quem preenche é o launcher.
/// <c>Game</c> aponta para o próprio item, porque os temas também leem <c>Game.Platforms</c>, <c>Game.IsInstalled</c>...
/// </summary>
public sealed class P10GameEntry(string id, string name) : P10Observable
{
    private ImageSource? _coverImage;
    private ImageSource? _backgroundImage;
    private bool _isRunning;

    public string Id { get; } = id;

    public string Name { get; } = name;

    public string DisplayName => Name;

    public P10GameEntry Game => this;

    public string Description { get; init; } = string.Empty;

    public bool IsInstalled { get; init; }

    public bool IsRunning
    {
        get => _isRunning;
        set => SetField(ref _isRunning, value);
    }

    public bool IsLaunching => false;

    public bool IsInstalling => false;

    public bool IsUninstalling => false;

    public bool Favorite { get; init; }

    public bool Hidden => false;

    public bool IsHidden => false;

    /// <summary>Tempo jogado em segundos (como no Playnite).</summary>
    public ulong Playtime { get; init; }

    public DateTime? LastActivity { get; init; }

    public DateTime? Added { get; init; }

    /// <summary>Data de lançamento já formatada (o Playnite expõe uma estrutura que os temas mostram como texto).</summary>
    public string? ReleaseDate { get; init; }

    public int? ReleaseYear { get; init; }

    public IReadOnlyList<P10NamedItem> Platforms { get; init; } = [];

    public IReadOnlyList<P10NamedItem> Genres { get; init; } = [];

    public IReadOnlyList<P10NamedItem> Developers { get; init; } = [];

    public IReadOnlyList<P10NamedItem> Publishers { get; init; } = [];

    public IReadOnlyList<P10NamedItem> Tags { get; init; } = [];

    public IReadOnlyList<P10NamedItem> Categories { get; init; } = [];

    public IReadOnlyList<P10NamedItem> Features { get; init; } = [];

    public P10NamedItem? CompletionStatus { get; init; }

    public P10NamedItem? Source { get; init; }

    /// <summary>Nome da biblioteca como no Playnite ("Steam", "Epic", "GOG", "Xbox", "Playnite").</summary>
    public string? Library => LibraryPlugin?.Name;

    public P10LibraryPlugin? LibraryPlugin { get; init; }

    public int? CriticScore { get; init; }

    public int? CommunityScore { get; init; }

    public int? UserScore { get; init; }

    /// <summary>
    /// Capa sob demanda: o launcher fornece a capa por <see cref="CoverSource"/> e a lê só quando o tema mostra o item
    /// (grade virtualizada); <see cref="NotifyCoverChanged"/> avisa quando ela fica pronta. Um valor atribuído vence.
    /// </summary>
    public Func<ImageSource?>? CoverSource { get; init; }

    public ImageSource? CoverImage
    {
        get => _coverImage ?? CoverSource?.Invoke();
        set
        {
            if (SetField(ref _coverImage, value))
            {
                OnPropertyChanged(nameof(CoverImageObject));
                OnPropertyChanged(nameof(CoverImageObjectCached));
            }
        }
    }

    /// <summary>A capa de <see cref="CoverSource"/> ficou pronta ou mudou.</summary>
    public void NotifyCoverChanged()
    {
        OnPropertyChanged(nameof(CoverImage));
        OnPropertyChanged(nameof(CoverImageObject));
        OnPropertyChanged(nameof(CoverImageObjectCached));
    }

    public ImageSource? CoverImageObject => CoverImage;

    public ImageSource? CoverImageObjectCached => CoverImage;

    /// <summary>Arte de fundo; o launcher carrega sob demanda (jogo selecionado).</summary>
    public ImageSource? BackgroundImage
    {
        get => _backgroundImage;
        set
        {
            if (SetField(ref _backgroundImage, value))
            {
                OnPropertyChanged(nameof(DisplayBackgroundImage));
                OnPropertyChanged(nameof(DisplayBackgroundImageObject));
            }
        }
    }

    public ImageSource? DisplayBackgroundImage => BackgroundImage;

    public ImageSource? DisplayBackgroundImageObject => BackgroundImage;

    public ImageSource? Icon => null;

    public ImageSource? IconObject => null;

    public ImageSource? IconObjectCached => null;

    public override string ToString() => Name;
}
