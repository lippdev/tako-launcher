using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Presentation;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Painel de filtros da biblioteca (UX-002/004): instalação, favoritos, jogados recentemente, fonte, sistema (jogos de
/// emulador, EMU-007), gênero (quando os metadados trazem) e ordenação. Cada escolha vale na hora e a grade atrás do painel acompanha; B fecha. As linhas
/// são as mesmas dos painéis da Central de Controle (marcador, chave, valor no fim).
/// </summary>
public sealed partial class LibraryFiltersViewModel : ObservableObject, IOverlayViewModel
{
    private readonly Func<LibraryCriteria> _get;
    private readonly Func<LibraryCriteria, int> _set;
    private readonly PanelRowViewModel _install;
    private readonly PanelRowViewModel _favorites;
    private readonly PanelRowViewModel _played;
    private readonly PanelRowViewModel _clear;
    private readonly List<(LibrarySort Sort, PanelRowViewModel Row)> _sorts = [];
    private readonly List<(string Value, PanelRowViewModel Row)> _sources = [];
    private readonly List<(string Value, PanelRowViewModel Row)> _genres = [];
    private readonly List<(string Id, PanelRowViewModel Row)> _collections = [];
    private readonly List<(string Key, PanelRowViewModel Row)> _systems = [];

    [ObservableProperty]
    private string _countText = string.Empty;

    /// <param name="games">Jogos da página (a rota da barra lateral), de onde saem as fontes e os gêneros.</param>
    /// <param name="get">Critério atual.</param>
    /// <param name="set">Aplica o critério e devolve quantos jogos aparecem.</param>
    /// <param name="collections">Coleções do usuário (UX-003); as que têm jogos nesta página viram filtro.</param>
    public LibraryFiltersViewModel(IReadOnlyList<Game> games, Func<LibraryCriteria> get, Func<LibraryCriteria, int> set, IReadOnlyList<CollectionInfo>? collections = null)
    {
        ArgumentNullException.ThrowIfNull(games);
        _get = get;
        _set = set;
        _install = Row("Instalação", () => Change(c => c.NextInstall()));
        _favorites = Row("Só favoritos", () => Change(c => c with { FavoritesOnly = !c.FavoritesOnly }), toggle: true);
        _played = Row("Jogados recentemente", () => Change(c => c with { PlayedOnly = !c.PlayedOnly }), toggle: true);
        _clear = Row("Limpar filtros", () => Change(c => c.ClearFilters()));

        foreach (var sort in LibraryText.SortOrder)
        {
            _sorts.Add((sort, Row(LibraryText.SortLabel(sort), () => Change(c => c with { Sort = sort }), radio: true)));
        }

        foreach (var source in LibraryQuery.Sources(games, LibraryFacts.Of))
        {
            _sources.Add((source.Name, Row(source.Name, () => Change(c => c.ToggleSource(source.Name)), toggle: true, end: source.Count.ToString(System.Globalization.CultureInfo.InvariantCulture))));
        }

        foreach (var genre in LibraryQuery.Genres(games, LibraryFacts.Of))
        {
            _genres.Add((genre.Name, Row(genre.Name, () => Change(c => c.ToggleGenre(genre.Name)), toggle: true, end: genre.Count.ToString(System.Globalization.CultureInfo.InvariantCulture))));
        }

        // Sistemas de emulador (EMU-007): "SNES", "PlayStation"… quando a página tem jogos de mais de um.
        foreach (var system in LibraryQuery.Systems(games, LibraryFacts.Of))
        {
            string name = Core.Library.Metadata.ScreenScraper.ScreenScraperSystems.ForKey(system.Name)?.Name ?? system.Name;
            _systems.Add((system.Name, Row(name, () => Change(c => c.ToggleSystem(system.Name)), toggle: true, end: system.Count.ToString(System.Globalization.CultureInfo.InvariantCulture))));
        }

        foreach (var collection in collections ?? [])
        {
            int count = games.Count(g => collection.Contains(g.Id));
            if (count > 0 || _get().Collections.Contains(collection.Id))
            {
                _collections.Add((collection.Id, Row(collection.Name, () => Change(c => c.ToggleCollection(collection.Id)), toggle: true, end: count.ToString(System.Globalization.CultureInfo.InvariantCulture))));
            }
        }

        var sections = new List<PanelSection>
        {
            new("Mostrar", null, [_install, _favorites, _played]),
            new("Ordenar por", null, [.. _sorts.Select(s => s.Row)]),
        };
        if (_collections.Count > 0)
        {
            sections.Add(new PanelSection("Coleção", null, [.. _collections.Select(c => c.Row)]));
        }

        if (_sources.Count > 1)
        {
            sections.Add(new PanelSection("Fonte", null, [.. _sources.Select(s => s.Row)]));
        }

        if (_systems.Count > 1 || (_systems.Count == 1 && _get().Systems.Count > 0))
        {
            sections.Add(new PanelSection("Sistema", null, [.. _systems.Select(s => s.Row)]));
        }

        if (_genres.Count > 0)
        {
            sections.Add(new PanelSection("Gênero", "Vem das informações dos jogos; aparece quando os dados chegam.", [.. _genres.Select(g => g.Row)]));
        }

        sections.Add(new PanelSection("Filtros", null, [_clear]));
        Sections = sections;
        Update(_set(_get()));
    }

    public IReadOnlyList<PanelSection> Sections { get; }

    private void Change(Func<LibraryCriteria, LibraryCriteria> change) => Update(_set(change(_get())));

    private void Update(int count)
    {
        var criteria = _get();
        _install.EndText = LibraryText.InstallLabel(criteria.Install);
        _favorites.IsOn = criteria.FavoritesOnly;
        _played.IsOn = criteria.PlayedOnly;
        foreach (var (sort, row) in _sorts)
        {
            row.IsOn = criteria.Sort == sort;
        }

        foreach (var (value, row) in _sources)
        {
            row.IsOn = criteria.Sources.Contains(value);
        }

        foreach (var (value, row) in _genres)
        {
            row.IsOn = criteria.Genres.Contains(value);
        }

        foreach (var (id, row) in _collections)
        {
            row.IsOn = criteria.Collections.Contains(id);
        }

        foreach (var (key, row) in _systems)
        {
            row.IsOn = criteria.Systems.Contains(key);
        }

        _clear.EndText = criteria.ActiveFilterCount switch
        {
            0 => string.Empty,
            1 => "1 ativo",
            int n => $"{n} ativos",
        };
        CountText = LibraryText.GameCount(count);
    }

    private static PanelRowViewModel Row(string label, Action action, bool toggle = false, bool radio = false, string end = "") =>
        new() { Label = label, Command = new RelayCommand(action), IsToggle = toggle, IsRadio = radio, EndText = end };
}
