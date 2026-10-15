using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Página da área de conteúdo.</summary>
public abstract partial class PageViewModel(ShellViewModel shell) : ObservableObject
{
    protected ShellViewModel Shell { get; } = shell;

    public abstract string Title { get; }

    /// <summary>Rota da barra lateral que fica marcada ("home", "library:all", "settings").</summary>
    public abstract string RouteKey { get; }

    /// <summary>Elemento que recebe o foco ao abrir a página (<c>Nav.Key</c>); nulo usa o marcado como padrão.</summary>
    public virtual string? DefaultFocusKey => null;

    /// <summary>Chamado quando uma capa desta página recebe o foco.</summary>
    public virtual void OnGameFocused(GameItemViewModel game)
    {
    }

    /// <summary>Atualiza depois de mudanças na biblioteca (favorito, sessão encerrada).</summary>
    public virtual void Refresh()
    {
    }

    [RelayCommand]
    private void Search() => Shell.Toast("Busca com teclado virtual chega no UX-001 (pós-MVP)");
}

public sealed partial class HomeViewModel : PageViewModel
{
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HeroOverline), nameof(PlayLabel))]
    private GameItemViewModel? _hero;

    private bool _resetHero = true;

    public HomeViewModel(ShellViewModel shell)
        : base(shell)
    {
        Refresh();
    }

    public override string Title => "Início";

    public override string? DefaultFocusKey => IsEmpty ? "home:add" : null;

    public override string RouteKey => "home";

    public ObservableCollection<GameItemViewModel> RecentlyPlayed { get; } = [];

    public ObservableCollection<GameItemViewModel> RecentlyAdded { get; } = [];

    public string RecentCountText => $"{RecentlyPlayed.Count} jogos";

    public string HeroOverline => Hero is not null && RecentlyPlayed.Count > 0 && RecentlyPlayed[0] == Hero ? "Continuar jogando" : Hero?.Overline ?? string.Empty;

    public string PlayLabel => Hero is not null && Hero.IsRunning ? "Voltar ao jogo" : "Jogar";

    public override void OnGameFocused(GameItemViewModel game) => Hero = game;

    /// <summary>Sem jogos: a Início mostra como começar (Steam não encontrada ou nada instalado).</summary>
    public bool IsEmpty => Shell.Library.Games.Count == 0;

    [RelayCommand]
    private void AddGame() => Shell.AddGame();

    [RelayCommand]
    private void Rescan() => Shell.RescanLibrary();

    public override void Refresh()
    {
        OnPropertyChanged(nameof(IsEmpty));
        Replace(RecentlyPlayed, LibraryQuery.RecentlyPlayed(Shell.Library.Games).Select(Shell.ItemFor));
        Replace(RecentlyAdded, LibraryQuery.RecentlyAdded(Shell.Library.Games, 10).Select(Shell.ItemFor));
        OnPropertyChanged(nameof(RecentCountText));
        // Mantém o jogo em foco no destaque; na primeira montagem, abre no último jogado.
        bool keep = !_resetHero && Hero is not null && (RecentlyPlayed.Contains(Hero) || RecentlyAdded.Contains(Hero));
        Hero = keep ? Hero : RecentlyPlayed.FirstOrDefault() ?? RecentlyAdded.FirstOrDefault();
        _resetHero = false;
        OnPropertyChanged(nameof(HeroOverline));
        OnPropertyChanged(nameof(PlayLabel));
    }

    [RelayCommand]
    private void PlayHero()
    {
        if (Hero is not null)
        {
            Shell.Launch(Hero);
        }
    }

    [RelayCommand]
    private void HeroDetails()
    {
        if (Hero is not null)
        {
            Shell.OpenDetails(Hero);
        }
    }

    [RelayCommand]
    private void OpenGame(GameItemViewModel game) => Shell.OpenDetails(game);

    [RelayCommand]
    private void SeeAll() => Shell.OpenLibrary(LibraryFilter.All);

    /// <summary>Substitui o conteúdo só se a ordem mudou (reconstruir a lista tiraria o foco do item).</summary>
    internal static void Replace<T>(ObservableCollection<T> target, IEnumerable<T> items)
    {
        var list = items.ToList();
        if (target.SequenceEqual(list))
        {
            return;
        }

        target.Clear();
        foreach (var item in list)
        {
            target.Add(item);
        }
    }
}

public sealed partial class LibraryViewModel : PageViewModel
{
    private static readonly (LibrarySort Sort, string Label)[] Sorts =
    [
        (LibrarySort.RecentlyPlayed, "Jogados recentemente"),
        (LibrarySort.Name, "Nome"),
        (LibrarySort.PlayTime, "Tempo de jogo"),
        (LibrarySort.RecentlyAdded, "Adicionados"),
    ];

    private int _sortIndex;

    public LibraryViewModel(ShellViewModel shell, LibraryFilter filter, string title, string routeKey)
        : base(shell)
    {
        Filter = filter;
        Title = title;
        RouteKey = routeKey;
        Refresh();
    }

    public LibraryFilter Filter { get; }

    public override string Title { get; }

    public override string RouteKey { get; }

    public ObservableCollection<GameItemViewModel> Games { get; } = [];

    public string CountText => Games.Count == 1 ? "1 jogo" : $"{Games.Count} jogos";

    public string SortLabel => Sorts[_sortIndex].Label;

    /// <summary>A biblioteca abre na primeira capa, não no botão de ordenar.</summary>
    public override string? DefaultFocusKey => Games.Count > 0 ? $"grid:{Games[0].Id}" : null;

    public override void OnGameFocused(GameItemViewModel game) => Shell.SetBackdrop(game);

    public override void Refresh()
    {
        HomeViewModel.Replace(Games, LibraryQuery.Apply(Shell.Library.Games, Filter, Sorts[_sortIndex].Sort).Select(Shell.ItemFor));
        OnPropertyChanged(nameof(CountText));
        OnPropertyChanged(nameof(SortLabel));
    }

    [RelayCommand]
    private void OpenGame(GameItemViewModel game) => Shell.OpenDetails(game);

    [RelayCommand]
    private void AddGame() => Shell.AddGame();

    [RelayCommand]
    private void CycleSort()
    {
        _sortIndex = (_sortIndex + 1) % Sorts.Length;
        Refresh();
        Shell.RequestFocus("lib:sort");
    }
}

public sealed record StatViewModel(string Label, string Value);

public sealed partial class DetailsViewModel : PageViewModel
{
    public DetailsViewModel(ShellViewModel shell, GameItemViewModel game)
        : base(shell)
    {
        Game = game;
        Refresh();
    }

    public GameItemViewModel Game { get; }

    public override string Title => Game.Title;

    public override string RouteKey => string.Empty;

    public ObservableCollection<StatViewModel> Stats { get; } = [];

    public string PrimaryLabel => (Game.IsRunning, Game.Game) switch
    {
        (true, _) => "Voltar ao jogo",
        (_, { IsUpcoming: true }) => "Avisar no lançamento",
        (_, { IsInstalled: true }) => "Jogar",
        (_, { Store: GameStore.Manual }) => "Arquivo não encontrado",
        _ => $"Instalar pela {Game.StoreName}",
    };

    public bool PrimaryHasPlayIcon => Game.Game.CanPlay;

    public string FavoriteLabel => Game.IsFavorite ? "Favorito" : "Favoritar";

    public string FavoriteIconKey => Game.IsFavorite ? "Icon.StarFill" : "Icon.Star";

    public override void Refresh()
    {
        var g = Game.Game;
        HomeViewModel.Replace(Stats, new[]
        {
            new StatViewModel("Tempo de jogo", Game.PlayTimeText),
            new StatViewModel("Última vez", Game.LastPlayedText),
            Game.MetacriticText is null ? null : new StatViewModel("Metacritic", Game.MetacriticText),
            g.SizeLabel is null ? null : new StatViewModel("Tamanho", g.SizeLabel),
            Game.ReleaseText is null ? null : new StatViewModel("Lançamento", Game.ReleaseText),
            g.Developer is null ? null : new StatViewModel("Desenvolvedora", g.Developer),
            new StatViewModel("Status", Game.Status),
        }.OfType<StatViewModel>());
        OnPropertyChanged(nameof(PrimaryLabel));
        OnPropertyChanged(nameof(FavoriteLabel));
        OnPropertyChanged(nameof(FavoriteIconKey));
    }

    [RelayCommand]
    private void Primary()
    {
        var g = Game.Game;
        if (g.CanPlay || Game.IsRunning)
        {
            Shell.Launch(Game);
        }
        else if (g.IsUpcoming)
        {
            Shell.Toast($"Você será avisado quando {g.Title} for lançado");
        }
        else if (g.Store == GameStore.Manual)
        {
            Shell.Toast($"Arquivo não encontrado: {g.LaunchTarget}. Use Opções para remover ou corrigir");
        }
        else
        {
            Shell.Toast($"Abrindo a {Game.StoreName} para instalar (integração com lojas é pós-MVP)");
        }
    }

    [RelayCommand]
    private void ToggleFavorite() => Shell.ToggleFavorite(Game);

    [RelayCommand]
    private void Options() => Shell.ShowGameOptions(Game);
}

public sealed partial class SettingsViewModel : PageViewModel
{
    public SettingsViewModel(ShellViewModel shell, SettingsService settings, ThemeService themes)
        : base(shell)
    {
        Settings = settings;
        Themes = themes;
    }

    public SettingsService Settings { get; }

    public ThemeService Themes { get; }

    public override string Title => "Configurações";

    public override string RouteKey => "settings";

    public string ThemeLabel => Themes.CurrentTheme == ThemeService.DefaultTheme ? "Padrão" : Themes.CurrentTheme;

    [RelayCommand]
    private void ReapplyTheme()
    {
        var result = Themes.Apply(ThemeService.DefaultTheme);
        OnPropertyChanged(nameof(ThemeLabel));
        Shell.Toast(result.Succeeded ? "Tema padrão recarregado pelo carregador XAML restrito" : $"Falha ao recarregar: {result.Error}");
    }

    [RelayCommand]
    private void ToggleDynamicColor()
    {
        Settings.DynamicColor = !Settings.DynamicColor;
        Shell.RefreshBackdrop();
    }

    [RelayCommand]
    private void ToggleReducedMotion() => Settings.ReducedMotion = !Settings.ReducedMotion;

    [RelayCommand]
    private void AddGame() => Shell.AddGame();

    [RelayCommand]
    private void Rescan() => Shell.RescanLibrary();

    [RelayCommand]
    private void RefreshMetadata() => Shell.RefreshMetadata();

    /// <summary>Abre no Explorer a pasta dos logs de diagnóstico (MVP-033).</summary>
    [RelayCommand]
    private void OpenLogs()
    {
        try
        {
            System.IO.Directory.CreateDirectory(AppPaths.LogsDirectory);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(AppPaths.LogsDirectory) { UseShellExecute = true })?.Dispose();
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or System.IO.IOException or UnauthorizedAccessException)
        {
            Shell.Toast($"Não foi possível abrir a pasta de logs: {ex.Message}");
        }
    }

    [RelayCommand]
    private void ToggleStartup() => Settings.StartFullscreenOnLogin = !Settings.StartFullscreenOnLogin;

    [RelayCommand]
    private void ToggleRestoreEnvironment() => Settings.RestoreEnvironmentAfterGame = !Settings.RestoreEnvironmentAfterGame;

    [RelayCommand]
    private void PreviewPlayniteTheme() => Shell.ShowPlaynitePreview();

    [RelayCommand]
    private void SimulateThemeFailure()
    {
        Shell.ShowRecovery(Themes.Apply(ThemeService.InvalidSampleTheme));
        OnPropertyChanged(nameof(ThemeLabel));
    }
}
