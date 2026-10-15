using System.Collections.ObjectModel;
using System.Windows.Input;
using System.Windows.Media;
using Launcher.Core.Library;

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

    /// <summary>Busca (<c>PART_ButtonSearch</c>): o launcher abre o próprio teclado virtual e escreve em <see cref="P10MainModel.SearchText"/>.</summary>
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

/// <summary>Busca e filtro ativos (<c>AppSettings.Fullscreen.FilterSettings</c> nos temas): <c>Name</c> é o texto buscado.</summary>
public sealed class P10FilterSettings : P10Observable
{
    private string _name = string.Empty;
    private bool _isActive;

    public string Name
    {
        get => _name;
        internal set => SetField(ref _name, value);
    }

    /// <summary>Há busca ou filtro do painel ligado.</summary>
    public bool IsActive
    {
        get => _isActive;
        internal set => SetField(ref _isActive, value);
    }
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
/// lista de jogos com filtro rápido, busca e painel de filtros, jogo selecionado, detalhes e os comandos dos botões
/// <c>PART_</c>. Busca, filtros e ordenação seguem as regras da biblioteca do Tako (<see cref="LibraryCriteria"/>).
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
    private LibraryCriteria _criteria = LibraryCriteria.None with { Sort = LibrarySort.Name };
    private bool _filterPanelVisible;
    private P10SubFilter? _subFilter;

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
        ToggleFiltersCommand = new P10Command(ToggleFilterPanel);
        NextFilterPresetCommand = new P10Command(NextFilterPreset);
        ClearFiltersCommand = new P10Command(() => Criteria = Criteria.ClearFilters());
        ClearSearchCommand = new P10Command(() => SearchText = string.Empty);
        FilterPanel = new P10FilterPanel(this);
        FilterPanel.Update(_criteria, hasGenres: false);
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

    /// <summary>Busca, filtros do painel e ordenação da grade. Mudar reaplica a lista na hora.</summary>
    public LibraryCriteria Criteria
    {
        get => _criteria;
        set
        {
            ArgumentNullException.ThrowIfNull(value);
            if (!SetField(ref _criteria, value))
            {
                return;
            }

            AppSettings.Fullscreen.FilterSettings.Name = value.Search;
            AppSettings.Fullscreen.FilterSettings.IsActive = value.HasSearch || value.HasFilters;
            OnPropertyChanged(nameof(SearchText));
            OnPropertyChanged(nameof(IsSearchActive));
            OnPropertyChanged(nameof(IsExtraFilterActive));
            UpdatePanel();
            ApplyFilter();
        }
    }

    /// <summary>Texto da busca (sem acento e sem diferenciar maiúsculas); escrito pelo teclado do launcher.</summary>
    public string SearchText
    {
        get => Criteria.Search;
        set => Criteria = Criteria with { Search = value?.Trim() ?? string.Empty };
    }

    /// <summary>Painel de filtros aberto (<c>PART_ElemFilters</c>). Abrir fecha a lista de fonte/gênero.</summary>
    public bool FilterPanelVisible
    {
        get => _filterPanelVisible;
        set
        {
            if (value)
            {
                SubFilter = null;
            }

            SetField(ref _filterPanelVisible, value);
        }
    }

    /// <summary>Lista de fonte ou gênero aberta (<c>PART_ContentFilterItems</c>); o painel principal fica oculto.</summary>
    public P10SubFilter? SubFilter
    {
        get => _subFilter;
        private set
        {
            if (SetField(ref _subFilter, value))
            {
                OnPropertyChanged(nameof(SubFilterVisible));
            }
        }
    }

    public bool SubFilterVisible => SubFilter is not null;

    /// <summary>Filtros adicionais do Playnite (<c>PART_ElemFiltersAdditional</c>): as listas ficam no próprio painel.</summary>
    public bool FilterAdditionalPanelVisible => false;

    public P10FilterPanel FilterPanel { get; }

    /// <summary>Há texto buscado (<c>PART_ElemSearchActive</c>).</summary>
    public bool IsSearchActive => Criteria.HasSearch;

    /// <summary>Há filtro do painel sem busca (<c>PART_ElemExtraFilterActive</c>, "lista filtrada").</summary>
    public bool IsExtraFilterActive => Criteria.HasFilters && !Criteria.HasSearch;

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

    /// <summary><c>PART_ButtonFilter</c>: abre e fecha o painel de filtros.</summary>
    public ICommand ToggleFiltersCommand { get; }

    /// <summary>Próximo filtro rápido (<c>FilterPresetSelector</c>).</summary>
    public ICommand NextFilterPresetCommand { get; }

    public ICommand ApplyFilterPresetCommand { get; }

    public ICommand ClearFiltersCommand { get; }

    public ICommand ClearSearchCommand { get; }

    /// <summary>Troca os jogos (biblioteca mudou), mantendo o selecionado quando ele continua na lista.</summary>
    public void SetGames(IEnumerable<P10GameEntry> games)
    {
        _all.Clear();
        _all.AddRange(games);
        UpdatePanel();
        ApplyFilter();
    }

    /// <summary>
    /// B/Esc, do mais interno para fora: lista de fonte/gênero → painel de filtros → detalhes do jogo → busca ativa.
    /// Falso quando não havia nada do tema para fechar.
    /// </summary>
    public bool HandleBack()
    {
        if (SubFilter is not null)
        {
            FilterPanelVisible = true;
            return true;
        }

        if (FilterPanelVisible)
        {
            FilterPanelVisible = false;
            return true;
        }

        if (GameDetailsVisible)
        {
            GameDetailsVisible = false;
            return true;
        }

        if (IsSearchActive)
        {
            SearchText = string.Empty;
            return true;
        }

        return false;
    }

    /// <summary>Fecha o painel de filtros e a lista aberta (ex.: antes de abrir a busca).</summary>
    public void CloseFilterPanels()
    {
        SubFilter = null;
        FilterPanelVisible = false;
    }

    /// <summary>Fatos do jogo do tema para as regras de filtro e ordenação do Tako.</summary>
    public static LibraryFacts Facts(P10GameEntry game)
    {
        ArgumentNullException.ThrowIfNull(game);
        return new LibraryFacts(
            game.Name,
            game.IsInstalled,
            game.Favorite,
            game.Source?.Name ?? game.Library ?? string.Empty,
            [.. game.Genres.Select(g => g.Name)],
            game.LastActivity is { } last ? new DateTimeOffset(last) : null,
            TimeSpan.FromSeconds(game.Playtime),
            game.CriticScore,
            game.Added is { } added ? new DateTimeOffset(added) : default);
    }

    /// <summary>Abre a lista de fontes ou de gêneros no lugar do painel.</summary>
    public void OpenSubFilter(P10SubFilterKind kind)
    {
        bool sources = kind == P10SubFilterKind.Sources;
        var values = sources ? LibraryQuery.Sources(_all, Facts) : LibraryQuery.Genres(_all, Facts);
        var selected = sources ? Criteria.Sources : Criteria.Genres;
        var options = values.Select(v => new P10FilterOption(v.Name, new P10Command(() => Criteria = sources ? Criteria.ToggleSource(v.Name) : Criteria.ToggleGenre(v.Name)))
        {
            Name = $"{v.Name} ({v.Count})",
            IsChecked = selected.Contains(v.Name),
        }).ToList();
        var clear = new P10Command(() => Criteria = sources
            ? Criteria with { Sources = LibraryCriteria.None.Sources }
            : Criteria with { Genres = LibraryCriteria.None.Genres });
        SetField(ref _filterPanelVisible, false, nameof(FilterPanelVisible)); // a lista entra no lugar do painel
        SubFilter = new P10SubFilter(kind, sources ? "Fonte" : "Gênero", options, new P10Command(() => FilterPanelVisible = true), clear);
    }

    private void ToggleFilterPanel()
    {
        if (FilterPanelVisible || SubFilter is not null)
        {
            CloseFilterPanels();
            return;
        }

        GameDetailsVisible = false;
        FilterPanelVisible = true;
    }

    private void UpdatePanel()
    {
        FilterPanel.Update(Criteria, _all.Any(g => g.Genres.Count > 0));
        if (SubFilter is { } sub)
        {
            var selected = sub.Kind == P10SubFilterKind.Sources ? Criteria.Sources : Criteria.Genres;
            foreach (var option in sub.Options)
            {
                option.IsChecked = selected.Contains(option.Key);
            }
        }
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
        var visible = LibraryQuery.Apply(_all.Where(g => ActiveFilterPreset?.Matches(g) ?? true), Facts, Criteria);
        if (!Games.SequenceEqual(visible))
        {
            Games.Clear();
            foreach (var game in visible)
            {
                Games.Add(game);
            }
        }

        SelectedGame = visible.FirstOrDefault(g => g.Id == selectedId) ?? (visible.Count > 0 ? visible[0] : null);
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
