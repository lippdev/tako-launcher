using System.Collections.ObjectModel;
using System.Windows.Media;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Input;
using Launcher.Core.Library.Metadata;
using Launcher.Core.Library.Metadata.Art;
using Launcher.Core.Library.Metadata.SteamGridDb;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Uma imagem na grade do seletor: miniatura sob demanda, se é a que está em uso e a chave de foco.</summary>
public sealed partial class ArtTileViewModel(ArtCandidate candidate, int index, ArtService art, int thumbWidth) : LazyThumb(art, candidate.Preview, thumbWidth), INavKeyed
{
    [ObservableProperty]
    private bool _isCurrent;

    public ArtCandidate Candidate { get; } = candidate;

    public string NavKey { get; } = $"art:{index}";

    public bool IsAnimated => Candidate.IsAnimated;
}

/// <summary>
/// Seletor de imagens (ADR-0023): modal sobre os detalhes ou a grade, 100% pelo controle. Abas de tipo (Capa, Fundo,
/// Logo, Ícone) trocadas com LB/RB; uma linha de fontes (só as ligadas); filtros do SteamGridDB (X); grade de
/// miniaturas carregadas sob demanda e prévia grande da focada (com autor, tamanho, estilo e animação). A aplica e
/// fixa por tipo, ☰ volta para a automática, Y busca outro jogo no SteamGridDB, B fecha.
/// </summary>
public sealed partial class ArtPickerViewModel : ObservableObject, IOverlayViewModel, IOverlayInput, IOverlayHints, IDisposable
{
    public const int PageSize = 50;

    private readonly IArtCatalog _catalog;
    private readonly ArtService _art;
    private readonly AnimatedArtService _animated;
    private readonly ArtPickerHost _host;
    private CancellationTokenSource _loading = new();
    private System.Windows.Threading.DispatcherTimer? _previewTimer;
    private int _previewToken;
    private int _page;
    private bool _hasMore;
    private bool _loadingMore;

    [ObservableProperty]
    private ArtKind _kind;

    [ObservableProperty]
    private string _source = ArtSources.File;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasStatus))]
    private string? _status;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasPreview))]
    private ArtTileViewModel? _focused;

    [ObservableProperty]
    private ImageSource? _preview;

    [ObservableProperty]
    private string _steamGridDbGame = string.Empty;

    [ObservableProperty]
    private SteamGridDbFilter _filter = new(ArtKind.Cover);

    private string? _steamGridDbGameId;

    public ArtPickerViewModel(GameItemViewModel item, ArtKind kind, IArtCatalog catalog, ArtService art, AnimatedArtService animated, ArtPickerHost host)
    {
        ArgumentNullException.ThrowIfNull(item);
        ArgumentNullException.ThrowIfNull(host);
        Item = item;
        _catalog = catalog;
        _art = art;
        _animated = animated;
        _host = host;
        _kind = kind;
        _filter = new SteamGridDbFilter(kind) { Animated = host.ShowAnimated };
        Kinds = [.. ArtKinds.All.Select(k => new ModalTab($"art-kind:{ArtKinds.Key(k)}", ArtKinds.Label(k), new RelayCommand(() => SelectKind(k))) { IsSelected = k == kind })];
        Sources = [.. catalog.Sources(item.Game).Select(s => new ModalTab($"art-source:{s}", s, new RelayCommand(() => SelectSource(s))))];
        _source = Sources.FirstOrDefault(s => ArtSources.Offers(s.Label, kind))?.Label ?? ArtSources.File;
        MarkSelected();
        RebuildFilters();
        _ = LoadAsync();
    }

    public GameItemViewModel Item { get; }

    public string Title => $"Imagens de {Item.Title}";

    public IReadOnlyList<ModalTab> Kinds { get; }

    public IReadOnlyList<ModalTab> Sources { get; }

    /// <summary>Filtros do SteamGridDB (estilo, formato, animadas); só com a fonte SteamGridDB.</summary>
    public ObservableCollection<ModalTab> Filters { get; } = [];

    public ObservableCollection<ArtTileViewModel> Items { get; } = [];

    public bool IsSteamGridDb => Source == ArtSources.SteamGridDb;

    public bool IsFile => Source == ArtSources.File;

    public bool HasStatus => !string.IsNullOrEmpty(Status);

    public bool HasPreview => Focused is not null;

    /// <summary>A imagem em uso deste tipo foi escolhida pelo usuário ("Voltar para a automática" faz sentido).</summary>
    public bool IsPinned => Item.Game.PinnedArt.ContainsKey(Kind);

    /// <summary>Largura e altura das miniaturas, pelo formato do tipo.</summary>
    public double TileWidth => Kind switch
    {
        ArtKind.Hero => 300,
        ArtKind.Logo => 260,
        ArtKind.Icon => 140,
        _ => Filter.Horizontal && IsSteamGridDb ? 300 : 150,
    };

    public double TileHeight => Kind switch
    {
        ArtKind.Hero => 100,
        ArtKind.Logo => 120,
        ArtKind.Icon => 140,
        _ => Filter.Horizontal && IsSteamGridDb ? 140 : 225,
    };

    public string PreviewCaption => Focused?.Candidate.Caption ?? string.Empty;

    /// <summary>Crédito pedido pelo SteamGridDB, na prévia.</summary>
    public string? PreviewCredit => Focused?.Candidate is { Source: ArtSources.SteamGridDb, Author: { } author }
        ? $"Arte de {author} no SteamGridDB" + (Focused.Candidate.IsAnimated ? " · animada" : string.Empty)
        : Focused?.Candidate.IsAnimated == true ? "Animada" : null;

    public string AutomaticLabel => IsPinned ? "Voltar para a automática" : "Já é a automática";

    public IReadOnlyList<(string Buttons, string Label, AppCommand Command)> Hints
    {
        get
        {
            var list = new List<(string, string, AppCommand)> { ("a", "Aplicar", AppCommand.Accept), ("b", "Fechar", AppCommand.Back), ("lb+rb", "Tipo", AppCommand.NextSection) };
            if (IsSteamGridDb)
            {
                list.Add(("x", "Filtros", AppCommand.Favorite));
                list.Add(("y", "Buscar", AppCommand.Search));
            }

            list.Add(("menu", "Automática", AppCommand.Menu));
            return list;
        }
    }

    public bool HandleCommand(AppCommand command)
    {
        switch (command)
        {
            case AppCommand.PreviousSection or AppCommand.NextSection:
                int index = ArtKinds.All.ToList().IndexOf(Kind);
                int step = command == AppCommand.NextSection ? 1 : -1;
                SelectKind(ArtKinds.All[(index + step + ArtKinds.All.Count) % ArtKinds.All.Count]);
                return true;
            case AppCommand.Favorite when IsSteamGridDb:
                _host.Focus(Filters.FirstOrDefault()?.Key);
                return true;
            case AppCommand.Search:
                Search();
                return true;
            case AppCommand.Menu:
                ResetToAutomatic();
                return true;
            default:
                return false;
        }
    }

    /// <summary>A grade levou o foco a uma miniatura: prévia grande (animada, se for) e mais páginas perto do fim.</summary>
    public void OnTileFocused(ArtTileViewModel tile)
    {
        ArgumentNullException.ThrowIfNull(tile);
        Focused = tile;
        OnPropertyChanged(nameof(PreviewCaption));
        OnPropertyChanged(nameof(PreviewCredit));
        _ = LoadPreviewAsync(tile);
        if (_hasMore && !_loadingMore && Items.IndexOf(tile) >= Items.Count - 8)
        {
            _ = LoadMoreAsync();
        }
    }

    public void SelectKind(ArtKind kind)
    {
        Kind = kind;
        Filter = new SteamGridDbFilter(kind) { Animated = Filter.Animated };
        if (!ArtSources.Offers(Source, kind))
        {
            Source = Sources.FirstOrDefault(s => ArtSources.Offers(s.Label, kind))?.Label ?? ArtSources.File;
        }

        MarkSelected();
        RebuildFilters();
        _ = LoadAsync();
        _host.Focus($"art-kind:{ArtKinds.Key(kind)}");
    }

    public void SelectSource(string source)
    {
        Source = source;
        MarkSelected();
        RebuildFilters();
        _ = LoadAsync();
        _host.Focus($"art-source:{source}");
    }

    /// <summary>A: fixa a imagem para o tipo (a escolha fica até o usuário trocar ou voltar para a automática).</summary>
    [RelayCommand]
    private async Task ApplyAsync(ArtTileViewModel? tile)
    {
        if ((tile ?? Focused) is not { } chosen)
        {
            return;
        }

        if (await _host.Apply(chosen.Candidate).ConfigureAwait(true))
        {
            foreach (var other in Items)
            {
                other.IsCurrent = other == chosen;
            }

            OnPropertyChanged(nameof(IsPinned));
            OnPropertyChanged(nameof(AutomaticLabel));
        }
    }

    /// <summary>☰: solta a escolha do tipo; a imagem automática volta.</summary>
    [RelayCommand]
    private void ResetToAutomatic()
    {
        if (!IsPinned)
        {
            _host.Toast($"{ArtKinds.Label(Kind)} já é a automática");
            return;
        }

        _host.Reset(Kind);
        foreach (var other in Items)
        {
            other.IsCurrent = false;
        }

        OnPropertyChanged(nameof(IsPinned));
        OnPropertyChanged(nameof(AutomaticLabel));
    }

    /// <summary>Y: procurar outro jogo (ou outro termo) no SteamGridDB, pelo teclado virtual.</summary>
    [RelayCommand]
    private void Search()
    {
        if (!Sources.Any(s => s.Label == ArtSources.SteamGridDb))
        {
            _host.Toast("Ligue o SteamGridDB em Configurações › Capas e metadados para buscar capas de outros jogos");
            return;
        }

        _host.EditText("Buscar no SteamGridDB", "Nome do jogo", SteamGridDbGameTitle ?? Item.Title, term => _ = SearchSteamGridDbAsync(term));
    }

    /// <summary>"Outra imagem do PC…": a janela do Windows (teclado e mouse), para qualquer pasta.</summary>
    [RelayCommand]
    private async Task BrowseFileAsync()
    {
        if (_host.PickFile() is { } path)
        {
            await _host.Apply(new ArtCandidate(ArtSources.File, Kind, new Uri(path)) { Label = System.IO.Path.GetFileName(path) }).ConfigureAwait(true);
            OnPropertyChanged(nameof(IsPinned));
            OnPropertyChanged(nameof(AutomaticLabel));
        }
    }

    [RelayCommand]
    private void Close() => _host.Close();

    public void Dispose()
    {
        StopPreview();
        _loading.Cancel();
        _loading.Dispose();
    }

    private string? SteamGridDbGameTitle { get; set; }

    private async Task SearchSteamGridDbAsync(string term)
    {
        Status = $"Buscando \"{term}\" no SteamGridDB…";
        var results = await _catalog.SearchSteamGridDbAsync(term, CancellationToken.None).ConfigureAwait(true);
        if (results.Count == 0)
        {
            Status = $"Nada encontrado para \"{term}\" no SteamGridDB.";
            return;
        }

        _host.Choose("Qual jogo do SteamGridDB?", results, match =>
        {
            _steamGridDbGameId = match.Id;
            SteamGridDbGameTitle = match.Title;
            SteamGridDbGame = $"Imagens de: {Core.Presentation.GameText.MatchLabel(match)} · Y troca";
            Source = ArtSources.SteamGridDb;
            MarkSelected();
            RebuildFilters();
            _ = LoadAsync();
        });
    }

    private void MarkSelected()
    {
        foreach (var tab in Kinds)
        {
            tab.IsSelected = tab.Key == $"art-kind:{ArtKinds.Key(Kind)}";
        }

        foreach (var tab in Sources)
        {
            tab.IsSelected = tab.Label == Source;
        }

        OnPropertyChanged(nameof(IsSteamGridDb));
        OnPropertyChanged(nameof(IsFile));
        OnPropertyChanged(nameof(IsPinned));
        OnPropertyChanged(nameof(AutomaticLabel));
        OnPropertyChanged(nameof(TileWidth));
        OnPropertyChanged(nameof(TileHeight));
        _host.RefreshHints();
    }

    private void RebuildFilters()
    {
        Filters.Clear();
        if (!IsSteamGridDb)
        {
            return;
        }

        var styles = new List<string?> { null };
        styles.AddRange(SteamGridDbQueries.Styles(Kind));
        Filters.Add(new ModalTab("art-filter:style", $"Estilo: {SteamGridDbQueries.StyleLabel(Filter.Style)}", new RelayCommand(() =>
        {
            int i = styles.IndexOf(Filter.Style);
            ChangeFilter(Filter with { Style = styles[(i + 1) % styles.Count] }, "art-filter:style");
        })));
        if (Kind == ArtKind.Cover)
        {
            Filters.Add(new ModalTab("art-filter:shape", Filter.Horizontal ? "Formato: horizontal" : "Formato: vertical", new RelayCommand(() =>
                ChangeFilter(Filter with { Horizontal = !Filter.Horizontal }, "art-filter:shape"))));
        }

        Filters.Add(new ModalTab("art-filter:animated", Filter.Animated ? "Animadas: sim" : "Animadas: não", new RelayCommand(() =>
            ChangeFilter(Filter with { Animated = !Filter.Animated }, "art-filter:animated"))));
        Filters.Add(new ModalTab("art-filter:safe", "Adulto e humor: ocultos", new RelayCommand(() =>
            _host.Toast("Imagens adultas e de humor ficam sempre de fora no Tako"))));
    }

    private void ChangeFilter(SteamGridDbFilter filter, string focusKey)
    {
        Filter = filter;
        OnPropertyChanged(nameof(TileWidth));
        OnPropertyChanged(nameof(TileHeight));
        RebuildFilters();
        _ = LoadAsync();
        _host.Focus(focusKey);
    }

    private async Task LoadAsync()
    {
        await Task.Yield(); // depois de o modal entrar na tela (o foco vai para a primeira miniatura)
        await _loading.CancelAsync().ConfigureAwait(true);
        _loading.Dispose();
        _loading = new CancellationTokenSource();
        var token = _loading.Token;
        StopPreview();
        Items.Clear();
        Focused = null;
        Preview = null;
        _page = 0;
        _hasMore = false;
        Status = $"Carregando {ArtKinds.Label(Kind).ToLowerInvariant()} de {Source}…";
        try
        {
            if (IsSteamGridDb && _steamGridDbGameId is null)
            {
                var match = await _catalog.SteamGridDbGameAsync(Item.Game, token).ConfigureAwait(true);
                _steamGridDbGameId = match?.Id;
                SteamGridDbGameTitle = match?.Title;
                SteamGridDbGame = match is null ? "Jogo não encontrado no SteamGridDB · Y para buscar" : $"Imagens de: {Core.Presentation.GameText.MatchLabel(match)} · Y troca";
            }

            var found = await _catalog.ListAsync(Query(0), token).ConfigureAwait(true);
            if (token.IsCancellationRequested)
            {
                return;
            }

            Add(found);
            _hasMore = IsSteamGridDb && found.Count >= PageSize;
            Status = Items.Count > 0 ? null : EmptyText();
            if (Items.Count > 0)
            {
                _host.Focus(Items[0].NavKey); // A já aplica a primeira
            }
        }
        catch (OperationCanceledException)
        {
            // Trocou de aba antes de terminar.
        }
    }

    private async Task LoadMoreAsync()
    {
        _loadingMore = true;
        var token = _loading.Token;
        try
        {
            var found = await _catalog.ListAsync(Query(_page + 1), token).ConfigureAwait(true);
            if (!token.IsCancellationRequested)
            {
                _page++;
                Add(found);
                _hasMore = found.Count >= PageSize;
            }
        }
        catch (OperationCanceledException)
        {
            // Trocou de aba.
        }
        finally
        {
            _loadingMore = false;
        }
    }

    private ArtQuery Query(int page) => new(Item.Game, Source, Kind)
    {
        Filter = IsSteamGridDb ? Filter : null,
        SteamGridDbGameId = IsSteamGridDb ? _steamGridDbGameId : null,
        Page = page,
    };

    private void Add(IEnumerable<ArtCandidate> candidates)
    {
        var current = ArtPins.Shown(Item.Game, Kind);
        int width = (int)Math.Ceiling(TileWidth);
        foreach (var candidate in candidates.Where(c => !Items.Any(i => i.Candidate.Url == c.Url)))
        {
            Items.Add(new ArtTileViewModel(candidate, Items.Count, _art, width) { IsCurrent = candidate.Url == current });
        }
    }

    private string EmptyText() => Source switch
    {
        ArtSources.File => "Nenhuma imagem em Imagens, Downloads ou Área de Trabalho. Use \"Outra imagem do PC…\".",
        ArtSources.SteamGridDb when _steamGridDbGameId is null => "Este jogo não foi encontrado no SteamGridDB. Toque Y para buscar pelo nome.",
        ArtSources.Steam => "A Steam não tem esta imagem para o jogo.",
        _ => $"{Source} não tem {ArtKinds.Label(Kind).ToLowerInvariant()} para este jogo. Confira o jogo em Opções › Fontes e IDs.",
    };

    private async Task LoadPreviewAsync(ArtTileViewModel tile)
    {
        StopPreview();
        int token = ++_previewToken;
        Preview = tile.Thumb;
        var full = await _art.GetAsync(tile.Candidate.Url, 520).ConfigureAwait(true);
        if (token != _previewToken)
        {
            return;
        }

        Preview = full ?? tile.Thumb;
        if (tile.Candidate.IsAnimated && _host.CanAnimate()
            && await _animated.LoadAsync(tile.Candidate.Url, 520).ConfigureAwait(true) is { } frames && token == _previewToken)
        {
            int index = 0;
            _previewTimer = new System.Windows.Threading.DispatcherTimer { Interval = TimeSpan.FromMilliseconds(frames.Delays[0]) };
            _previewTimer.Tick += (_, _) =>
            {
                index = (index + 1) % frames.Frames.Count;
                Preview = frames.Frames[index];
                _previewTimer!.Interval = TimeSpan.FromMilliseconds(frames.Delays[index]);
            };
            Preview = frames.Frames[0];
            _previewTimer.Start();
        }
    }

    private void StopPreview()
    {
        _previewToken++;
        _previewTimer?.Stop();
        _previewTimer = null;
    }
}

/// <summary>O que o seletor pede à Shell (aplicar, voltar à automática, teclado virtual, foco, dicas...).</summary>
public sealed record ArtPickerHost(
    Func<ArtCandidate, Task<bool>> Apply,
    Action<ArtKind> Reset,
    Action<string, string, string, Action<string>> EditText,
    Action<string, IReadOnlyList<MetadataMatch>, Action<MetadataMatch>> Choose,
    Func<string?> PickFile,
    Action<string> Toast,
    Action<string?> Focus,
    Action RefreshHints,
    Func<bool> CanAnimate,
    Action Close)
{
    public bool ShowAnimated { get; init; }
}
