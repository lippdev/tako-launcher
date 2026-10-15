using System.ComponentModel;
using System.Globalization;
using System.Windows;
using System.Windows.Threading;
using Launcher.Core.Library;
using Launcher.Core.Themes;
using Launcher.PlayniteCompat.P10;
using Launcher.PlayniteCompat.P10.Controls;
using Launcher.PlayniteCompat.P10.Data;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Tema Fullscreen do Playnite aplicado à tela principal do Tako (MVP-018/019). Liga o modelo do adaptador
/// (<see cref="P10MainModel"/>) à biblioteca real: Jogar, opções e o menu principal passam pelo mesmo caminho da UI do
/// Tako (sessão de jogo, diálogos, Configurações). O tema nunca chama o launcher diretamente.
/// </summary>
public sealed class PlayniteUiViewModel : IP10Host, IDisposable
{
    private readonly ShellViewModel _shell;
    private readonly Dictionary<string, GameItemViewModel> _items = new(StringComparer.Ordinal);
    private readonly Dictionary<string, P10GameEntry> _entries = new(StringComparer.Ordinal);
    private string _signature = string.Empty;
    private P10SubFilterKind? _lastSubFilter;
    private bool _disposed;

    public PlayniteUiViewModel(ShellViewModel shell, P10ThemeReport report, InstalledTheme theme)
    {
        _shell = shell;
        Report = report;
        Theme = theme;
        Model = new P10MainModel(this,
        [
            new P10FilterPreset("Todos", _ => true),
            new P10FilterPreset("Instalados", g => g.IsInstalled),
            new P10FilterPreset("Favoritos", g => g.Favorite),
            new P10FilterPreset("Jogados recentemente", g => g.LastActivity is not null),
        ]);
        Api = new P10Api(Model);
        Model.CurrentTime = shell.Clock;
        Model.PropertyChanged += OnModelChanged;
        shell.Library.Changed += OnLibraryChanged;
        Refresh();
    }

    public P10ThemeReport Report { get; }

    public InstalledTheme Theme { get; }

    public string Name => Report.Name;

    public ResourceDictionary Resources => Report.Resources;

    public P10MainModel Model { get; }

    public P10Api Api { get; }

    /// <summary>Detalhes do jogo abertos dentro do tema (B fecha).</summary>
    public bool IsDetailsOpen => Model.GameDetailsVisible;

    /// <summary>Painel de filtros ou lista de fonte/gênero aberto dentro do tema (B volta um nível).</summary>
    public bool IsFilterOpen => Model.FilterPanelVisible || Model.SubFilterVisible;

    /// <summary>
    /// Onde o foco vai quando os painéis do tema mudam: botão principal dos detalhes, o filtro padrão do painel (ou o
    /// botão da lista de onde se voltou), ou a capa selecionada.
    /// </summary>
    public string? PanelFocusKey => (IsDetailsOpen, Model.SubFilterVisible, Model.FilterPanelVisible) switch
    {
        (true, _, _) => $"{GameDetails.DetailsGroup}:play",
        (_, true, _) => null,
        (_, _, true) => _lastSubFilter switch
        {
            P10SubFilterKind.Sources => $"{Filters.FiltersGroup}:sources",
            P10SubFilterKind.Genres => $"{Filters.FiltersGroup}:genres",
            _ => null,
        },
        _ => SelectedFocusKey,
    };

    /// <summary>Chave de navegação do jogo selecionado (ou nulo).</summary>
    public string? SelectedFocusKey => Model.SelectedGame is { } game ? $"{Main.GamesGroup}:{game.Id}" : null;

    /// <summary>Jogo do tema → jogo do Tako, com os nomes de propriedades que os temas Playnite usam.</summary>
    public static P10GameEntry ToEntry(GameItemViewModel item)
    {
        ArgumentNullException.ThrowIfNull(item);
        var game = item.Game;
        var pc = new P10NamedItem("PC (Windows)");
        return new P10GameEntry(game.Id, game.Title)
        {
            Description = game.Description,
            IsInstalled = game.IsInstalled,
            IsRunning = item.IsRunning,
            Favorite = game.IsFavorite,
            Playtime = (ulong)Math.Max(0, game.PlayTime.TotalSeconds),
            LastActivity = game.LastPlayedAt?.LocalDateTime,
            Added = game.AddedAt == default ? null : game.AddedAt.LocalDateTime,
            ReleaseDate = item.ReleaseText,
            ReleaseYear = game.ReleaseDate?.Year,
            Platforms = [pc],
            Genres = [.. game.Genres.Select(g => new P10NamedItem(g))],
            Developers = game.Developer is null ? [] : [new P10NamedItem(game.Developer)],
            Publishers = game.Publisher is null ? [] : [new P10NamedItem(game.Publisher)],
            LibraryPlugin = new P10LibraryPlugin(LibraryName(game.Store)),
            Source = new P10NamedItem(item.StoreName),
            CriticScore = game.MetacriticScore ?? game.CriticScore,
            CoverSource = () => item.Cover, // só carrega quando o tema mostra a capa
        };
    }

    /// <summary>Nomes das bibliotecas como os plugins do Playnite (os temas montam caminhos de ícones com eles).</summary>
    public static string LibraryName(GameStore store) => store switch
    {
        GameStore.Steam => "Steam",
        GameStore.Epic => "Epic",
        GameStore.Gog => "GOG",
        GameStore.Xbox => "Xbox",
        _ => "Playnite",
    };

    /// <summary>Reconstrói a lista quando a biblioteca muda (jogo novo, favorito, tempo jogado); mantém a seleção.</summary>
    public void Refresh()
    {
        var games = LibraryQuery.Apply(_shell.Library.Games, LibraryFilter.All, LibrarySort.Name);
        string signature = string.Join('\n', games.Select(g => string.Create(CultureInfo.InvariantCulture, $"{g.Id}|{g.Title}|{g.IsInstalled}|{g.IsFavorite}|{g.PlayTime.Ticks}|{g.LastPlayedAt?.Ticks}|{g.Description.Length}")));
        if (signature == _signature)
        {
            return;
        }

        _signature = signature;
        Unsubscribe();
        _entries.Clear();
        var entries = new List<P10GameEntry>();
        foreach (var game in games)
        {
            var item = _shell.ItemFor(game);
            var entry = ToEntry(item);
            _items[game.Id] = item;
            _entries[game.Id] = entry;
            item.PropertyChanged += OnItemChanged;
            entries.Add(entry);
        }

        Model.SetGames(entries);
    }

    /// <summary>B/Esc dentro do tema: fecha os detalhes. Falso quando não havia o que fechar.</summary>
    public bool HandleBack() => Model.HandleBack();

    public void UpdateClock(string clock) => Model.CurrentTime = clock;

    // ---------------------------------------------------------------- IP10Host

    public void Activate(P10GameEntry game)
    {
        if (Item(game) is { } item)
        {
            // Mesmo comportamento do botão principal dos detalhes do Tako (Jogar, Voltar ao jogo, instalar...).
            new DetailsViewModel(_shell, item).PrimaryCommand.Execute(null);
            Model.RefreshSelection();
        }
    }

    public string ActionLabel(P10GameEntry game) => Item(game) is { } item ? new DetailsViewModel(_shell, item).PrimaryLabel : string.Empty;

    public void ShowGameOptions(P10GameEntry game)
    {
        if (Item(game) is { } item)
        {
            _shell.ShowGameOptions(item);
        }
    }

    public void OpenMainMenu() => _shell.OpenSettings();

    /// <summary>Busca do tema (<c>PART_ButtonSearch</c> ou Y): o teclado virtual do Tako escreve no texto buscado do tema.</summary>
    public void OpenSearch() => _shell.OpenSearch();

    public void OpenNotifications() => _shell.Toast("Nenhuma notificação");

    public async void SelectionChanged(P10GameEntry? game)
    {
        if (game is null || Item(game) is not { } item)
        {
            Model.BackgroundImage = null;
            return;
        }

        var hero = game.BackgroundImage ?? await item.LoadHeroAsync().ConfigureAwait(true);
        if (_disposed || Model.SelectedGame != game)
        {
            return;
        }

        game.BackgroundImage = hero;
        Model.BackgroundImage = hero;
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        Unsubscribe();
        Model.PropertyChanged -= OnModelChanged;
        _shell.Library.Changed -= OnLibraryChanged;
    }

    private GameItemViewModel? Item(P10GameEntry game) => _items.GetValueOrDefault(game.Id);

    private void OnItemChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (sender is not GameItemViewModel item || !_entries.TryGetValue(item.Id, out var entry))
        {
            return;
        }

        if (e.PropertyName is nameof(GameItemViewModel.Cover) or null or "")
        {
            entry.NotifyCoverChanged();
        }

        if (e.PropertyName is nameof(GameItemViewModel.IsRunning) or null or "")
        {
            entry.IsRunning = item.IsRunning;
            Model.RefreshSelection();
        }
    }

    private void OnLibraryChanged(object? sender, EventArgs e)
    {
        // A biblioteca pode mudar fora da thread da UI (varredura, metadados).
        var dispatcher = Application.Current?.Dispatcher ?? Dispatcher.CurrentDispatcher;
        dispatcher.BeginInvoke(() =>
        {
            if (!_disposed)
            {
                Refresh();
                _shell.OnPlayniteUiListChanged();
            }
        });
    }

    private void OnModelChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(P10MainModel.SubFilter):
                _lastSubFilter = Model.SubFilter?.Kind ?? _lastSubFilter;
                _shell.OnPlayniteDetailsChanged();
                break;
            case nameof(P10MainModel.FilterPanelVisible):
                if (!Model.FilterPanelVisible && !Model.SubFilterVisible)
                {
                    _lastSubFilter = null; // painel fechado: na próxima abertura o foco vai ao primeiro filtro
                }

                _shell.OnPlayniteDetailsChanged();
                break;
            case nameof(P10MainModel.GameDetailsVisible):
                _shell.OnPlayniteDetailsChanged();
                break;
            case nameof(P10MainModel.IsSearchActive):
                _shell.OnPlayniteSearchChanged();
                break;
        }
    }

    private void Unsubscribe()
    {
        foreach (var item in _items.Values)
        {
            item.PropertyChanged -= OnItemChanged;
        }

        _items.Clear();
    }
}
