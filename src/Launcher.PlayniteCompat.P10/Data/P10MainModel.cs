using System.Collections.ObjectModel;
using System.Windows.Input;
using System.Windows.Media;

namespace Launcher.PlayniteCompat.P10.Data;

#pragma warning disable CA1822 // Bindings do WPF só leem propriedades de instância; valores fixos ficam como propriedades de instância.

/// <summary>
/// O que o launcher oferece à tela principal de um tema Playnite. O adaptador nunca chama serviços do launcher
/// diretamente: só pede estas ações, e o launcher decide (ex.: Jogar passa pela mesma sessão da UI do Tako).
/// </summary>
public interface IP10Host
{
    /// <summary>Ação principal do jogo (Jogar, Voltar ao jogo ou explicar por que não dá).</summary>
    void Activate(P10GameEntry game);

    /// <summary>Texto da ação principal ("Jogar", "Instalar pela Steam"...).</summary>
    string ActionLabel(P10GameEntry game);

    void ShowGameOptions(P10GameEntry game);

    /// <summary>Menu principal do tema: abre as Configurações do Tako (saída garantida do tema).</summary>
    void OpenMainMenu();

    void OpenSearch();

    void OpenNotifications();

    /// <summary>O jogo selecionado mudou (o launcher carrega a arte de fundo).</summary>
    void SelectionChanged(P10GameEntry? game);
}

/// <summary>Filtro rápido (o <c>FilterPresetSelector</c> do Playnite).</summary>
public sealed class P10FilterPreset(string name, Func<P10GameEntry, bool> matches) : P10Observable
{
    private bool _isActive;

    public string Name { get; } = name;

    public bool IsActive
    {
        get => _isActive;
        set => SetField(ref _isActive, value);
    }

    public bool Matches(P10GameEntry game) => matches(game);

    public override string ToString() => Name;
}

/// <summary>Texto da busca/filtro ativo (<c>AppSettings.Fullscreen.FilterSettings.Name</c> nos temas).</summary>
public sealed class P10FilterSettings
{
    public string Name => string.Empty;
}

public sealed class P10FullscreenAppSettings
{
    public P10FilterSettings FilterSettings { get; } = new();
}

/// <summary>Configurações do Playnite que os temas leem pelo modelo principal (<c>AppSettings.Fullscreen...</c>).</summary>
public sealed class P10AppSettings
{
    public P10FullscreenAppSettings Fullscreen { get; } = new();
}

/// <summary>
/// Modelo da tela principal do tema (o <c>FullscreenAppViewModel</c> do Playnite, subconjunto MVP-018/019):
/// lista de jogos com filtro rápido, jogo selecionado, detalhes e os comandos dos botões <c>PART_</c>.
/// É o DataContext do controle <see cref="Controls.Main"/> e o <c>MainModel</c> da extensão <c>{Api}</c>.
/// </summary>
public sealed class P10MainModel : P10Observable
{
    private readonly IP10Host _host;
    private readonly List<P10GameEntry> _all = [];
    private P10GameEntry? _selectedGame;
    private P10FilterPreset? _activeFilterPreset;
    private bool _gameDetailsVisible;
    private string _currentTime = string.Empty;
    private ImageSource? _backgroundImage;

    public P10MainModel(IP10Host host, IEnumerable<P10FilterPreset> presets)
    {
        _host = host;
        foreach (var preset in presets)
        {
            FilterPresets.Add(preset);
        }

        ActivateSelectedCommand = new P10Command(_ => Run(_host.Activate), _ => SelectedGame is not null);
        ToggleGameDetailsCommand = new P10Command(parameter =>
        {
            if (parameter is P10GameEntry game)
            {
                SelectedGame = game;
            }

            GameDetailsVisible = !GameDetailsVisible && SelectedGame is not null;
        });
        CloseGameDetailsCommand = new P10Command(() => GameDetailsVisible = false);
        ToggleGameOptionsCommand = new P10Command(_ => Run(_host.ShowGameOptions), _ => SelectedGame is not null);
        ToggleMainMenuCommand = new P10Command(_host.OpenMainMenu);
        ToggleNotificationsCommand = new P10Command(_host.OpenNotifications);
        OpenSearchCommand = new P10Command(_host.OpenSearch);
        ToggleFiltersCommand = new P10Command(NextFilterPreset);
        ApplyFilterPresetCommand = new P10Command(parameter => ActiveFilterPreset = parameter as P10FilterPreset ?? ActiveFilterPreset);
        _activeFilterPreset = FilterPresets.FirstOrDefault();
        if (_activeFilterPreset is not null)
        {
            _activeFilterPreset.IsActive = true;
        }
    }

    /// <summary>Jogos do filtro ativo, na ordem da grade.</summary>
    public ObservableCollection<P10GameEntry> Games { get; } = [];

    public ObservableCollection<P10FilterPreset> FilterPresets { get; } = [];

    public P10GameEntry? SelectedGame
    {
        get => _selectedGame;
        set
        {
            if (SetField(ref _selectedGame, value))
            {
                OnPropertyChanged(nameof(SelectedGameActionLabel));
                _host.SelectionChanged(value);
            }
        }
    }

    public string SelectedGameActionLabel => SelectedGame is null ? string.Empty : _host.ActionLabel(SelectedGame);

    public P10FilterPreset? ActiveFilterPreset
    {
        get => _activeFilterPreset;
        set
        {
            var previous = _activeFilterPreset;
            if (SetField(ref _activeFilterPreset, value))
            {
                if (previous is not null)
                {
                    previous.IsActive = false;
                }

                if (value is not null)
                {
                    value.IsActive = true;
                }

                ApplyFilter();
            }
        }
    }

    public bool GameDetailsVisible
    {
        get => _gameDetailsVisible;
        set => SetField(ref _gameDetailsVisible, value && SelectedGame is not null);
    }

    /// <summary>Status do jogo em execução (tela própria do Playnite); no Tako, a sessão tem tela própria.</summary>
    public bool GameStatusVisible => false;

    public bool MainMenuVisible => false;

    public bool FilterPanelVisible => false;

    public bool IsSearchActive => false;

    public bool IsExtraFilterActive => false;

    public bool ProgressActive => false;

    public string ProgressStatus => string.Empty;

    public bool UpdatesAvailable => false;

    /// <summary>Bateria no topo do tema (<c>Fullscreen.ShowBattery</c>): o Tako não lê a bateria.</summary>
    public bool ShowBattery => false;

    /// <summary>Bateria (<c>PowerStatus.Charge</c>, <c>PowerStatus.IsCharging</c>): sem leitura no Tako, fica nula.</summary>
    public object? PowerStatus => null;

    public P10AppSettings AppSettings { get; } = new();

    public string CurrentTime
    {
        get => _currentTime;
        set => SetField(ref _currentTime, value);
    }

    /// <summary>Arte de fundo do jogo selecionado (<c>PART_ImageBackground</c>).</summary>
    public ImageSource? BackgroundImage
    {
        get => _backgroundImage;
        set => SetField(ref _backgroundImage, value);
    }

    public ICommand ActivateSelectedCommand { get; }

    public ICommand ToggleGameDetailsCommand { get; }

    public ICommand CloseGameDetailsCommand { get; }

    public ICommand ToggleGameOptionsCommand { get; }

    public ICommand ToggleMainMenuCommand { get; }

    public ICommand ToggleNotificationsCommand { get; }

    public ICommand OpenSearchCommand { get; }

    public ICommand ToggleFiltersCommand { get; }

    public ICommand ApplyFilterPresetCommand { get; }

    /// <summary>Troca os jogos (biblioteca mudou), mantendo o selecionado quando ele continua na lista.</summary>
    public void SetGames(IEnumerable<P10GameEntry> games)
    {
        _all.Clear();
        _all.AddRange(games);
        ApplyFilter();
    }

    /// <summary>B/Esc: fecha os detalhes do jogo. Falso quando não havia nada do tema para fechar.</summary>
    public bool HandleBack()
    {
        if (!GameDetailsVisible)
        {
            return false;
        }

        GameDetailsVisible = false;
        return true;
    }

    /// <summary>Recalcula textos que dependem do estado do jogo (ex.: "Jogar" → "Voltar ao jogo").</summary>
    public void RefreshSelection() => OnPropertyChanged(nameof(SelectedGameActionLabel));

    private void NextFilterPreset()
    {
        if (FilterPresets.Count == 0)
        {
            return;
        }

        int index = ActiveFilterPreset is null ? -1 : FilterPresets.IndexOf(ActiveFilterPreset);
        ActiveFilterPreset = FilterPresets[(index + 1) % FilterPresets.Count];
    }

    private void ApplyFilter()
    {
        string? selectedId = SelectedGame?.Id;
        var visible = _all.Where(g => ActiveFilterPreset?.Matches(g) ?? true).ToList();
        if (!Games.SequenceEqual(visible))
        {
            Games.Clear();
            foreach (var game in visible)
            {
                Games.Add(game);
            }
        }

        SelectedGame = visible.FirstOrDefault(g => g.Id == selectedId) ?? visible.FirstOrDefault();
        if (SelectedGame is null)
        {
            GameDetailsVisible = false;
        }
    }

    private void Run(Action<P10GameEntry> action)
    {
        if (SelectedGame is { } game)
        {
            action(game);
        }
    }
}
