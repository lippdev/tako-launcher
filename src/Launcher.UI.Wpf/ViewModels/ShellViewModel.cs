using System.Collections.ObjectModel;
using System.Globalization;
using System.Windows.Media;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Pedido para a view focar um elemento (por <c>Nav.Key</c>) ou o padrão do escopo atual.</summary>
public sealed class FocusRequestEventArgs(string? key) : EventArgs
{
    public string? Key { get; } = key;
}

/// <summary>
/// Raiz da UI: páginas e histórico, sobreposições, sessão de jogo, fundo dinâmico, dicas, status e avisos.
/// A view (MainWindow) cuida de foco, entrada e animações; aqui fica o estado.
/// </summary>
public sealed partial class ShellViewModel : ObservableObject
{
    private static readonly TimeSpan LaunchDelay = TimeSpan.FromSeconds(2.6);
    private static readonly TimeSpan ToastDuration = TimeSpan.FromSeconds(2.6);

    private readonly ArtService _art;
    private readonly ThemeService _themes;
    private readonly PlayniteThemeService _playniteThemes;
    private readonly TimeProvider _time;
    private readonly ILogger<ShellViewModel> _logger;
    private readonly Dictionary<string, GameItemViewModel> _items = [];
    private readonly Stack<(PageViewModel Page, string? FocusKey)> _history = new();
    private readonly DispatcherTimer _clockTimer = new() { Interval = TimeSpan.FromSeconds(15) };
    private readonly DispatcherTimer _toastTimer = new() { Interval = ToastDuration };
    private readonly DispatcherTimer _launchTimer = new() { Interval = LaunchDelay };
    private string? _backdropGameId;
    private object? _focusedContext;
    private string? _focusedGroup;
    private bool _focusedIsAdjustable;

    [ObservableProperty]
    private PageViewModel _currentPage;

    [ObservableProperty]
    private bool _isSidebarExpanded;

    [ObservableProperty]
    private BackdropState? _backdrop;

    [ObservableProperty]
    private string _clock = string.Empty;

    [ObservableProperty]
    private InputKind _inputKind = InputKind.Keyboard;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(InputStatus))]
    private string? _controllerName;

    /// <summary>Controle PlayStation conectado: dicas com ✕ ○ □ △ e PS.</summary>
    [ObservableProperty]
    private bool _isPlayStationPad;

    [ObservableProperty]
    private string? _toastMessage;

    [ObservableProperty]
    private bool _isToastVisible;

    [ObservableProperty]
    private GameItemViewModel? _runningGame;

    [ObservableProperty]
    private SessionState _session = SessionState.Browsing;

    public ShellViewModel(
        IGameLibrary library,
        ArtService art,
        SettingsService settings,
        ThemeService themes,
        PlayniteThemeService playniteThemes,
        PerformanceSimulator performance,
        TimeProvider time,
        ILogger<ShellViewModel> logger)
    {
        Library = library;
        _art = art;
        Settings = settings;
        _themes = themes;
        _playniteThemes = playniteThemes;
        Performance = performance;
        _time = time;
        _logger = logger;
        foreach (var game in library.Games)
        {
            var item = new GameItemViewModel(game, art, time);
            _items[game.Id] = item;
            _ = item.LoadCoverAsync();
        }

        SidebarItems = BuildSidebar();
        SidebarMain = SidebarItems.Where(s => !s.IsBottom && s.Key != "exit").ToList();
        SidebarFooter = SidebarItems.Except(SidebarMain).ToList();
        _currentPage = new HomeViewModel(this);
        SyncSidebar();
        UpdateClock();
        _clockTimer.Tick += (_, _) => UpdateClock();
        _clockTimer.Start();
        _toastTimer.Tick += (_, _) => { IsToastVisible = false; _toastTimer.Stop(); };
        _launchTimer.Tick += (_, _) => { _launchTimer.Stop(); StartSession(); };
        library.Changed += (_, _) => RefreshAll();
        if (_currentPage is HomeViewModel { Hero: { } hero })
        {
            SetBackdrop(hero);
        }
    }

    public event EventHandler<FocusRequestEventArgs>? FocusRequested;

    /// <summary>Comando vindo de uma dica clicada; a view o trata como se viesse do teclado/controle.</summary>
    public event EventHandler<AppCommand>? CommandRequested;

    public IGameLibrary Library { get; }

    public SettingsService Settings { get; }

    public PerformanceSimulator Performance { get; }

    public SessionOptions SessionOptions { get; } = new();

    public ObservableCollection<SidebarItemViewModel> SidebarItems { get; }

    public IReadOnlyList<SidebarItemViewModel> SidebarMain { get; }

    public IReadOnlyList<SidebarItemViewModel> SidebarFooter { get; }

    public ObservableCollection<object> Overlays { get; } = [];

    /// <summary>Nos detalhes a arte do jogo ocupa a tela inteira.</summary>
    public bool IsBackdropLarge => CurrentPage is DetailsViewModel;

    public ObservableCollection<HintViewModel> Hints { get; } = [];

    public object? TopOverlay => Overlays.Count > 0 ? Overlays[^1] : null;

    /// <summary>O jogo está em primeiro plano: a UI do launcher (status, dicas) some.</summary>
    public bool IsInGame => TopOverlay is GameSessionViewModel;

    public bool IsQuickMenuOpen => Overlays.Any(o => o is ControlCenterViewModel);

    public string InputStatus => ControllerName ?? "Teclado";

    public GameItemViewModel ItemFor(Game game) => _items[game.Id];

    partial void OnCurrentPageChanged(PageViewModel value) => OnPropertyChanged(nameof(IsBackdropLarge));

    // ---------------------------------------------------------------- Navegação entre páginas

    public void GoHome() => Navigate(new HomeViewModel(this), replace: true, clearHistory: true);

    public void OpenLibrary(LibraryFilter filter)
    {
        var item = SidebarItems.FirstOrDefault(s => s.Filter == filter);
        Navigate(new LibraryViewModel(this, filter, LibraryTitle(filter), item?.Key ?? "library:all"), replace: CurrentPage is LibraryViewModel);
    }

    public void OpenDetails(GameItemViewModel game) => Navigate(new DetailsViewModel(this, game), replace: false);

    public void OpenSettings() => Navigate(new SettingsViewModel(this, Settings, _themes), replace: CurrentPage is SettingsViewModel);

    [RelayCommand]
    private void Hint(AppCommand command) => CommandRequested?.Invoke(this, command);

    /// <summary>B/Esc: fecha a sobreposição do topo, ou volta uma página no histórico.</summary>
    [RelayCommand]
    public void Back()
    {
        if (TopOverlay is IOverlayViewModel overlay)
        {
            if (!overlay.HandleBack())
            {
                CloseOverlay(overlay);
            }

            return;
        }

        if (_history.Count > 0)
        {
            var (page, focusKey) = _history.Pop();
            page.Refresh();
            CurrentPage = page;
            SyncSidebar();
            RequestFocus(focusKey);
        }
        else if (CurrentPage is not HomeViewModel)
        {
            GoHome();
        }
    }

    /// <summary>LB/RB: Início → Todos → Instalados → Favoritos → Configurações.</summary>
    public void CycleSection(int step)
    {
        if (TopOverlay is not null)
        {
            return;
        }

        string[] order = ["home", "library:all", "library:installed", "library:favorites", "settings"];
        int i = Array.IndexOf(order, CurrentPage.RouteKey);
        string next = order[((i < 0 ? 0 : i) + step + order.Length) % order.Length];
        _history.Clear();
        SidebarItems.First(s => s.Key == next).Command.Execute(null);
    }

    public void RequestFocus(string? key) => FocusRequested?.Invoke(this, new FocusRequestEventArgs(key));

    private void Navigate(PageViewModel page, bool replace, bool clearHistory = false)
    {
        if (clearHistory)
        {
            _history.Clear();
        }
        else if (!replace)
        {
            _history.Push((CurrentPage, FocusKeyOfCurrent()));
        }

        CurrentPage = page;
        SyncSidebar();
        if (page is DetailsViewModel details)
        {
            SetBackdrop(details.Game);
        }
        else if (page is HomeViewModel { Hero: { } hero })
        {
            SetBackdrop(hero);
        }
        else if (page is LibraryViewModel library && library.Games.Count > 0)
        {
            SetBackdrop(library.Games[0]);
        }

        RequestFocus(page.DefaultFocusKey);
    }

    private string? FocusKeyOfCurrent() => _focusedContext is GameItemViewModel g && _focusedGroup is not null ? $"{_focusedGroup}:{g.Id}" : null;

    // ---------------------------------------------------------------- Biblioteca

    public void ToggleFavorite(GameItemViewModel game)
    {
        Library.ToggleFavorite(game.Id);
        Toast(game.IsFavorite ? $"{game.Title} adicionado aos favoritos" : $"{game.Title} removido dos favoritos");
    }

    private void RefreshAll()
    {
        foreach (var item in _items.Values)
        {
            item.Refresh();
        }

        UpdateSidebarCounts();
        CurrentPage.Refresh();

        // Se a lista foi reconstruída (ex.: favorito removido do filtro "Favoritos"), o foco volta ao mesmo jogo ou ao padrão.
        if (TopOverlay is null && _focusedContext is GameItemViewModel)
        {
            RequestFocus(FocusKeyOfCurrent());
        }
    }

    // ---------------------------------------------------------------- Sessão de jogo (simulada no Marco 0)

    public void Launch(GameItemViewModel game)
    {
        if (RunningGame is not null)
        {
            if (RunningGame == game)
            {
                ResumeGame();
            }
            else
            {
                Toast($"Encerre {RunningGame.Title} antes de abrir outro jogo");
            }

            return;
        }

        Session = SessionState.LaunchingGame;
        PushOverlay(new LaunchOverlayViewModel(game, new RelayCommand(CancelLaunch)));
        _launchTimer.Start();
    }

    public void ResumeGame()
    {
        var session = Overlays.OfType<GameSessionViewModel>().FirstOrDefault();
        if (session is null)
        {
            return;
        }

        while (TopOverlay != session)
        {
            CloseOverlay(TopOverlay!);
        }

        Session = SessionState.Playing;
        OnOverlaysChanged();
    }

    /// <summary>Volta ao launcher sem fechar o jogo; Guia retorna a ele.</summary>
    public void GoHomeKeepingGame()
    {
        var game = RunningGame;
        foreach (var overlay in Overlays.OfType<ControlCenterViewModel>().ToList())
        {
            CloseOverlay(overlay);
        }

        // A sessão continua na pilha, mas o launcher fica em primeiro plano até o Guia.
        if (Overlays.OfType<GameSessionViewModel>().FirstOrDefault() is { } session)
        {
            Overlays.Remove(session);
            _backgroundSession = session;
        }

        Session = SessionState.Browsing;
        GoHome();
        if (game is not null)
        {
            Toast($"{game.Title} continua rodando · {GuideName} volta ao jogo");
        }

        OnOverlaysChanged();
    }

    public void EndSession()
    {
        var game = RunningGame;
        var session = Overlays.OfType<GameSessionViewModel>().FirstOrDefault() ?? _backgroundSession;
        Session = SessionState.Returning;
        Overlays.Clear();
        _backgroundSession = null;
        Performance.Stop();
        if (game is not null)
        {
            game.IsRunning = false;
            RunningGame = null;
            var played = session is null ? TimeSpan.FromMinutes(1) : _time.GetUtcNow() - session.Started;
            Library.RecordSession(game.Id, played < TimeSpan.FromMinutes(1) ? TimeSpan.FromMinutes(1) : played);
            LogSessionEnded(_logger, game.Id);
        }

        Session = SessionState.Restoring;
        GoHome();
        Session = SessionState.Browsing;
        if (game is not null)
        {
            Toast($"Bem-vindo de volta · sessão de {game.Title} encerrada");
        }

        OnOverlaysChanged();
    }

    /// <summary>Botão Guia / PS (ou View / Shift+Tab, como na Steam).</summary>
    public void Guide()
    {
        switch (TopOverlay)
        {
            case GameSessionViewModel session:
                PushOverlay(new ControlCenterViewModel(this, session));
                break;
            case ControlCenterViewModel center:
                CloseOverlay(center);
                break;
            case null when _backgroundSession is not null:
                PushOverlay(_backgroundSession);
                _backgroundSession = null;
                Session = SessionState.Playing;
                break;
        }
    }

    public string GuideName => (InputKind, IsPlayStationPad) switch
    {
        (InputKind.Gamepad, true) => "PS",
        (InputKind.Gamepad, false) => "Guia",
        _ => "Shift+Tab",
    };

    private GameSessionViewModel? _backgroundSession;

    private void CancelLaunch()
    {
        _launchTimer.Stop();
        Session = SessionState.Browsing;
        if (TopOverlay is LaunchOverlayViewModel launch)
        {
            CloseOverlay(launch);
        }
    }

    private async void StartSession()
    {
        if (TopOverlay is not LaunchOverlayViewModel launch)
        {
            return;
        }

        var game = launch.Game;
        var session = new GameSessionViewModel(game, SessionOptions, Performance);
        Overlays.Remove(launch);
        RunningGame = game;
        game.IsRunning = true;
        Session = SessionState.Playing;
        Performance.Start();
        PushOverlay(session);
        CurrentPage.Refresh();
        Toast($"Jogando {game.Title} · {GuideName} abre o menu rápido");
        session.Frame = await game.LoadHeroAsync().ConfigureAwait(true);
    }

    // ---------------------------------------------------------------- Sobreposições

    public void PushOverlay(object overlay)
    {
        Overlays.Add(overlay);
        OnOverlaysChanged();
    }

    public void CloseOverlay(object overlay)
    {
        if (overlay is IDisposable disposable)
        {
            disposable.Dispose();
        }

        Overlays.Remove(overlay);
        OnOverlaysChanged();
    }

    public void ConfirmExit()
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            "Sair para o desktop?",
            "O Tako fecha a tela cheia. Monitores e áudio são restaurados pelo Console Runtime (Marco 1).",
            [
                new OverlayAction("Sair", ActionStyle.Primary, new RelayCommand(() => System.Windows.Application.Current.Shutdown()), IsDefault: true),
                new OverlayAction("Cancelar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(dialog!))),
            ]);
        PushOverlay(dialog);
    }

    /// <summary>Tela de recuperação quando um tema é rejeitado; o app já está de volta no tema padrão.</summary>
    public void ShowRecovery(ThemeLoadResult result)
    {
        if (result.Succeeded)
        {
            Toast($"Tema '{result.ThemeFolder}' aplicado");
            return;
        }

        RecoveryViewModel? recovery = null;
        recovery = new RecoveryViewModel(
            result.Error ?? "Erro desconhecido.",
            [
                new OverlayAction("Usar tema padrão", ActionStyle.Primary, new RelayCommand(() =>
                {
                    CloseOverlay(recovery!);
                    Toast(_themes.Apply(ThemeService.DefaultTheme).Succeeded ? "Tema padrão restaurado" : "Tema padrão embutido em uso");
                }), IsDefault: true),
                new OverlayAction("Tentar de novo", ActionStyle.Secondary, new RelayCommand(() =>
                {
                    CloseOverlay(recovery!);
                    ShowRecovery(_themes.Apply(result.ThemeFolder));
                })),
                new OverlayAction("Sair para o desktop", ActionStyle.Secondary, new RelayCommand(ConfirmExit)),
            ]);
        PushOverlay(recovery);
    }

    /// <summary>Erro de tema em uso: fecha a prévia e mostra a recuperação.</summary>
    public void RecoverFromThemeFailure(string reason)
    {
        foreach (var preview in Overlays.OfType<PlaynitePreviewViewModel>().ToList())
        {
            CloseOverlay(preview);
        }

        ShowRecovery(new ThemeLoadResult(PlayniteThemeService.SampleTheme, false, reason));
    }

    /// <summary>Prévia do tema Fullscreen padrão do Playnite 10 com a biblioteca do launcher (H2, MVP-014/017/018).</summary>
    public void ShowPlaynitePreview()
    {
        var report = _playniteThemes.Load(PlayniteThemeService.SampleTheme);
        LogPlaynitePreview(_logger, report.Loaded.Count, report.Rejected.Count);
        PlaynitePreviewViewModel? preview = null;
        preview = new PlaynitePreviewViewModel(report, LibraryQuery.RecentlyPlayed(Library.Games).Select(ItemFor), new RelayCommand(() => CloseOverlay(preview!)));
        PushOverlay(preview);
    }

    private void OnOverlaysChanged()
    {
        OnPropertyChanged(nameof(TopOverlay));
        OnPropertyChanged(nameof(IsInGame));
        OnPropertyChanged(nameof(IsQuickMenuOpen));
        UpdateHints();
    }

    // ---------------------------------------------------------------- Fundo dinâmico

    public async void SetBackdrop(GameItemViewModel game)
    {
        _backdropGameId = game.Id;
        var palette = AccentPalette.For(game.Game.Accent, Settings.DynamicColor);
        var accent = GameItemViewModel.ToColor(palette.Accent);
        var deep = GameItemViewModel.ToColor(palette.Deep);
        // Cor primeiro (imediato); a arte entra quando carregar, se o foco ainda estiver no mesmo jogo.
        Backdrop = new BackdropState(game.Id, accent, deep, Backdrop?.GameId == game.Id ? Backdrop.Hero : null, game.HeroAlignment);
        ImageSource? hero = await game.LoadHeroAsync().ConfigureAwait(true);
        if (_backdropGameId == game.Id && hero is not null)
        {
            Backdrop = new BackdropState(game.Id, accent, deep, hero, game.HeroAlignment);
        }
    }

    public void RefreshBackdrop()
    {
        if (_backdropGameId is not null)
        {
            SetBackdrop(_items[_backdropGameId]);
        }
    }

    // ---------------------------------------------------------------- Foco, entrada, dicas

    /// <summary>A view informa o elemento focado (DataContext e grupo).</summary>
    public void OnFocusChanged(object? context, string? group, bool isAdjustable)
    {
        _focusedContext = context;
        _focusedGroup = group;
        _focusedIsAdjustable = isAdjustable;
        IsSidebarExpanded = group == "sidebar" && TopOverlay is null;
        if (context is GameItemViewModel game && TopOverlay is null)
        {
            CurrentPage.OnGameFocused(game);
            if (CurrentPage is HomeViewModel)
            {
                SetBackdrop(game);
            }
        }

        UpdateHints();
    }

    partial void OnInputKindChanged(InputKind value) => UpdateHints();

    partial void OnIsPlayStationPadChanged(bool value) => UpdateHints();

    private void UpdateHints()
    {
        bool key = InputKind != InputKind.Gamepad;
        var list = new List<(string Buttons, string Label, AppCommand Command)>();
        switch (TopOverlay)
        {
            case GameSessionViewModel:
                break;
            case ControlCenterViewModel center:
                list.Add(("a", "Selecionar", AppCommand.Accept));
                if (_focusedIsAdjustable)
                {
                    list.Add(("left+right", "Ajustar", AppCommand.Right));
                }
                else if (center.IsPopoverOpen)
                {
                    list.Add(("left+right", "Outro painel", AppCommand.Right));
                }

                list.Add(("b", center.IsPopoverOpen ? "Fechar painel" : "Voltar ao jogo", AppCommand.Back));
                list.Add(("guide", "Fechar", AppCommand.Guide));
                break;
            case LaunchOverlayViewModel:
                list.Add(("a", "Cancelar", AppCommand.Accept));
                break;
            case not null:
                list.Add(("a", "Selecionar", AppCommand.Accept));
                list.Add(("b", "Voltar", AppCommand.Back));
                break;
            default:
                bool onGame = _focusedContext is GameItemViewModel;
                list.Add(("a", onGame ? "Detalhes" : "Selecionar", AppCommand.Accept));
                if (onGame)
                {
                    list.Add(("x", "Favoritar", AppCommand.Favorite));
                }

                if (CurrentPage is not HomeViewModel || _history.Count > 0)
                {
                    list.Add(("b", "Voltar", AppCommand.Back));
                }

                if (_backgroundSession is not null)
                {
                    list.Add(("guide", "Voltar ao jogo", AppCommand.Guide));
                }

                list.Add(("y", "Buscar", AppCommand.Search));
                list.Add(("lb+rb", "Seção", AppCommand.NextSection));
                list.Add(("menu", "Configurações", AppCommand.Menu));
                break;
        }

        Hints.Clear();
        foreach (var (buttons, label, command) in list)
        {
            Hints.Add(new HintViewModel(buttons.Split('+').Select(b => Glyph(b, key, IsPlayStationPad)).ToList(), label, key, command));
        }
    }

    private static string Glyph(string button, bool keyboard, bool playStation) => (button, keyboard, playStation) switch
    {
        ("a", false, true) => "✕",
        ("b", false, true) => "○",
        ("x", false, true) => "□",
        ("y", false, true) => "△",
        ("lb", false, true) => "L1",
        ("rb", false, true) => "R1",
        ("menu", false, true) => "Options",
        ("guide", false, true) => "PS",
        _ => Glyph(button, keyboard),
    };

    private static string Glyph(string button, bool keyboard) => (button, keyboard) switch
    {
        ("a", true) => "Enter",
        ("b", true) => "Esc",
        ("x", true) => "F",
        ("y", true) => "/",
        ("lb", true) => "Q",
        ("rb", true) => "E",
        ("menu", true) => "M",
        ("guide", true) => "Shift+Tab",
        ("left", true) => "←",
        ("right", true) => "→",
        ("a", false) => "A",
        ("b", false) => "B",
        ("x", false) => "X",
        ("y", false) => "Y",
        ("lb", false) => "LB",
        ("rb", false) => "RB",
        ("menu", false) => "☰",
        ("guide", false) => "Guia",
        ("left", false) => "◀",
        ("right", false) => "▶",
        _ => button.ToUpperInvariant(),
    };

    // ---------------------------------------------------------------- Status, avisos

    public void Toast(string message)
    {
        ToastMessage = message;
        IsToastVisible = true;
        _toastTimer.Stop();
        _toastTimer.Start();
    }

    private void UpdateClock() => Clock = _time.GetLocalNow().ToString("HH:mm", CultureInfo.GetCultureInfo("pt-BR"));

    // ---------------------------------------------------------------- Barra lateral

    private ObservableCollection<SidebarItemViewModel> BuildSidebar()
    {
        SidebarItemViewModel Library(string key, string label, string icon, LibraryFilter filter, string? section = null, Brush? badge = null) =>
            new(key, label, icon, new RelayCommand(() => OpenLibrary(filter))) { Filter = filter, SectionTitle = section, BadgeBrush = badge };

        Brush StoreBrush(GameStore store) => _items.Values.First(i => i.Game.Store == store).StoreBrush;

        var items = new ObservableCollection<SidebarItemViewModel>
        {
            new("home", "Início", "Icon.Home", new RelayCommand(GoHome)),
            Library("library:all", "Todos os jogos", "Icon.Grid", LibraryFilter.All, section: "Biblioteca"),
            Library("library:installed", "Instalados", "Icon.Check", LibraryFilter.Installed),
            Library("library:favorites", "Favoritos", "Icon.Star", LibraryFilter.Favorites),
            Library("library:steam", "Steam", "Store.Steam", LibraryFilter.ForStore(GameStore.Steam), section: "Fontes", badge: StoreBrush(GameStore.Steam)),
            Library("library:epic", "Epic Games", "Store.Epic", LibraryFilter.ForStore(GameStore.Epic), badge: StoreBrush(GameStore.Epic)),
            Library("library:gog", "GOG", "Store.Gog", LibraryFilter.ForStore(GameStore.Gog), badge: StoreBrush(GameStore.Gog)),
            Library("library:xbox", "Xbox", "Store.Xbox", LibraryFilter.ForStore(GameStore.Xbox), badge: StoreBrush(GameStore.Xbox)),
            Library("library:manual", "Adicionados", "Store.Manual", LibraryFilter.ForStore(GameStore.Manual), badge: StoreBrush(GameStore.Manual)),
            new("settings", "Configurações", "Icon.Gear", new RelayCommand(OpenSettings)) { IsBottom = true },
            new("exit", "Sair para o desktop", "Icon.Power", new RelayCommand(ConfirmExit)),
        };
        return items;
    }

    private void SyncSidebar()
    {
        foreach (var item in SidebarItems)
        {
            item.IsSelected = item.Key == CurrentPage.RouteKey;
        }

        UpdateSidebarCounts();
    }

    private void UpdateSidebarCounts()
    {
        foreach (var item in SidebarItems.Where(s => s.Filter is not null))
        {
            item.Count = Library.Games.Count(item.Filter!.Value.Matches);
        }
    }

    private string LibraryTitle(LibraryFilter filter) =>
        SidebarItems.FirstOrDefault(s => s.Filter == filter)?.Label switch
        {
            "Adicionados" => "Adicionados manualmente",
            string label => label,
            null => "Todos os jogos",
        };

    [LoggerMessage(Level = LogLevel.Information, Message = "Tema Playnite: {Loaded} arquivos carregados, {Rejected} recusados")]
    private static partial void LogPlaynitePreview(ILogger logger, int loaded, int rejected);

    [LoggerMessage(Level = LogLevel.Information, Message = "Sessão de {GameId} encerrada")]
    private static partial void LogSessionEnded(ILogger logger, string gameId);
}
