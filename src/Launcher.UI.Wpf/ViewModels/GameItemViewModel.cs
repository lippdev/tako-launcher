using System.Globalization;
using System.Windows;
using System.Windows.Media;
using CommunityToolkit.Mvvm.ComponentModel;
using Launcher.Core.Library;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Jogo como aparece nas capas, no destaque e nos detalhes.</summary>
public sealed partial class GameItemViewModel : ObservableObject, INavKeyed
{
    private readonly ArtService _art;
    private readonly TimeProvider _time;

    // Capa sob demanda (PERF-004): referência fraca. Quem segura a imagem é a capa na tela e o cache LRU do ArtService;
    // fora da tela e fora do cache, o GC libera e a próxima leitura decodifica de novo.
    private WeakReference<ImageSource>? _cover;
    private bool _coverLoading;
    private bool _coverMissing;

    [ObservableProperty]
    private bool _isRunning;

    public GameItemViewModel(Game game, ArtService art, TimeProvider time)
    {
        Game = game;
        _art = art;
        _time = time;
        var palette = AccentPalette.For(game.Accent, dynamicColor: true);
        FallbackBrush = new LinearGradientBrush(ToColor(palette.Accent), ToColor(palette.Deep), 60);
        FallbackBrush.Freeze();
        HeroAlignment = ParseAlignment(game.HeroAlignment);
    }

    public Game Game { get; }

    public string Id => Game.Id;

    string INavKeyed.NavKey => Id;

    public string Title => Game.Title;

    public string StoreName => GameText.StoreName(Game.Store);

    public string StoreIconKey => $"Store.{Game.Store}";

    public Brush StoreBrush => StoreBrushes[Game.Store];

    public string SubLabel => Game.SizeLabel is null ? StoreName : $"{StoreName} · {Game.SizeLabel}";

    public string Genres => GameText.Genres(Game);

    public string Overline => Game.Genres.Count == 0 ? StoreName : $"{Genres} · {StoreName}";

    /// <summary>"Metacritic 90", quando a fonte de dados informa a nota.</summary>
    public string? MetacriticText => Game.MetacriticScore is int score ? score.ToString(CultureInfo.InvariantCulture) : null;

    public string? Developer => Game.Developer;

    /// <summary>Selo sobre a capa: "NÃO INSTALADO" ou "LANÇA EM 29/10".</summary>
    public string? Badge => Game.UpcomingRelease is DateOnly up ? $"LANÇA EM {GameText.ShortDate(up)}" : Game.IsInstalled ? null : "NÃO INSTALADO";

    public bool IsFavorite => Game.IsFavorite;

    public string LastPlayedText => GameText.LastPlayed(Game.LastPlayedAt, _time.GetUtcNow());

    public string PlayTimeText => GameText.PlayTime(Game.PlayTime);

    public string Status => GameText.Status(Game);

    public string? ReleaseText => Game.ReleaseDate is DateOnly d && !Game.IsUpcoming ? GameText.Date(d) : null;

    public string Description => Game.Description;

    public string? Note => Game.Note;

    /// <summary>Linha de metadados do destaque: "14 h jogadas • Jogado ontem • 120 GB".</summary>
    public string Meta
    {
        get
        {
            var parts = new List<string> { Game.PlayTime > TimeSpan.Zero ? $"{(int)Game.PlayTime.TotalHours} h jogadas" : "Ainda não jogado" };
            if (Game.LastPlayedAt is not null)
            {
                parts.Add(LastPlayedText);
            }

            if (Game.SizeLabel is not null)
            {
                parts.Add(Game.SizeLabel);
            }

            return string.Join("  •  ", parts);
        }
    }

    /// <summary>Gradiente do accent: fundo da capa enquanto a arte carrega ou quando não há rede.</summary>
    public LinearGradientBrush FallbackBrush { get; }

    /// <summary>Enquadramento da arte de fundo (x, y em 0..1).</summary>
    public Point HeroAlignment { get; }

    /// <summary>Largura da capa no palco, já com a escala de foco (204 × 1,08, arredondado para cima).</summary>
    private const int CoverDisplayWidth = 240;

    /// <summary>Maior largura da arte de fundo no palco (telas ultrawide incluídas).</summary>
    private const int HeroDisplayWidth = 2000;

    /// <summary>
    /// Capa decodificada no tamanho da tela. Só é carregada quando alguém lê (a capa entrou na tela): enquanto carrega,
    /// fica nula (a UI mostra o gradiente com o título) e avisa por <see cref="ObservableObject.PropertyChanged"/>.
    /// </summary>
    public ImageSource? Cover
    {
        get
        {
            if (_cover is not null && _cover.TryGetTarget(out var cover))
            {
                return cover;
            }

            if (_coverMissing || Game.CoverUri is null)
            {
                return null;
            }

            if (_art.TryGetCached(Game.CoverUri, CoverDisplayWidth, out var cached))
            {
                Remember(cached);
                return cached;
            }

            _ = LoadCoverAsync();
            return null;
        }
    }

    private async Task LoadCoverAsync()
    {
        if (_coverLoading)
        {
            return;
        }

        _coverLoading = true;
        var uri = Game.CoverUri;
        bool notify;
        try
        {
            // Nunca avisa no meio da leitura que pediu a capa (o binding ainda está lendo).
            await Task.Yield();
            var image = await _art.GetAsync(uri, CoverDisplayWidth).ConfigureAwait(true);
            bool stale = uri != Game.CoverUri; // a capa mudou durante a carga: lê de novo
            if (!stale)
            {
                Remember(image);
            }

            notify = stale || image is not null;
        }
        finally
        {
            _coverLoading = false;
        }

        if (notify)
        {
            OnPropertyChanged(nameof(Cover));
        }
    }

    private void Remember(ImageSource? image)
    {
        _cover = image is null ? null : new WeakReference<ImageSource>(image);
        _coverMissing = image is null;
    }

    public Task<ImageSource?> LoadHeroAsync() => _art.GetAsync(Game.HeroUri, HeroDisplayWidth);

    /// <summary>Recalcula textos derivados do estado do usuário (favorito, tempo jogado) e dos metadados.</summary>
    public void Refresh()
    {
        OnPropertyChanged(string.Empty); // todas as propriedades: favorito, tempos, gêneros, descrição, nota...
    }

    /// <summary>A capa mudou (metadados encontraram a arte, ou o usuário escolheu outra imagem): recarrega se estiver na tela.</summary>
    public void ReloadArt()
    {
        _cover = null;
        _coverMissing = false;
        OnPropertyChanged(nameof(Cover));
    }

    public static Color ToColor(Rgb rgb) => Color.FromRgb(rgb.R, rgb.G, rgb.B);

    private static Point ParseAlignment(string? value)
    {
        if (value is null)
        {
            return new Point(0.65, 0.25);
        }

        var parts = value.Replace("%", string.Empty, StringComparison.Ordinal).Split(' ', StringSplitOptions.RemoveEmptyEntries);
        return new Point(double.Parse(parts[0], CultureInfo.InvariantCulture) / 100, double.Parse(parts[1], CultureInfo.InvariantCulture) / 100);
    }

    public static Brush BrushFor(GameStore store) => StoreBrushes[store];

    private static readonly Dictionary<GameStore, Brush> StoreBrushes = new()
    {
        [GameStore.Steam] = Frozen("#171A21"),
        [GameStore.Epic] = Frozen("#2A2A2A"),
        [GameStore.Gog] = Frozen("#7A3FD1"), // store-gog (DESIGN.md)
        [GameStore.Xbox] = Frozen("#107C10"),
        [GameStore.Manual] = Frozen("#3D5A80"),
        [GameStore.Emulation] = Frozen(Core.Emulation.EmulationLibrarySource.Accent),
    };

    private static SolidColorBrush Frozen(string hex)
    {
        var brush = new SolidColorBrush(ToColor(Rgb.Parse(hex)));
        brush.Freeze();
        return brush;
    }
}
