using System.Windows.Input;
using Launcher.Core.Library;

namespace Launcher.PlayniteCompat.P10.Data;

/// <summary>
/// Uma linha do painel de filtros do tema (caixa de seleção, botão que abre uma lista ou ação). O tema só desenha a
/// linha com o próprio estilo (<c>FilterPanelCheckBoxEx</c>, <c>FilterPanelNagivationButton</c>...); o estado e a ação
/// vêm do adaptador.
/// </summary>
public sealed class P10FilterOption(string key, ICommand command) : P10Observable
{
    private string _name = string.Empty;
    private bool _isChecked;
    private bool _isVisible = true;

    /// <summary>Chave estável da linha (navegação por controle).</summary>
    public string Key { get; } = key;

    public string Name
    {
        get => _name;
        set => SetField(ref _name, value);
    }

    /// <summary>Marcada (caixa de seleção) ou com algo escolhido (botão de lista).</summary>
    public bool IsChecked
    {
        get => _isChecked;
        set => SetField(ref _isChecked, value);
    }

    public bool IsVisible
    {
        get => _isVisible;
        set => SetField(ref _isVisible, value);
    }

    public ICommand Command { get; } = command;

    public override string ToString() => Name;
}

/// <summary>
/// Painel de filtros do tema (o <c>Filters</c>/<c>PART_ElemFilters</c> do Playnite): instalação, favoritos, jogados
/// recentemente, listas de fonte e gênero, ordenação e limpar. A regra de cada filtro é a da biblioteca do Tako
/// (<see cref="LibraryCriteria"/>).
/// </summary>
public sealed class P10FilterPanel
{
    internal P10FilterPanel(P10MainModel model)
    {
        Installed = new P10FilterOption("installed", new P10Command(() => model.Criteria = model.Criteria with
        {
            Install = model.Criteria.Install == InstallFilter.Installed ? InstallFilter.Any : InstallFilter.Installed,
        }));
        Uninstalled = new P10FilterOption("uninstalled", new P10Command(() => model.Criteria = model.Criteria with
        {
            Install = model.Criteria.Install == InstallFilter.NotInstalled ? InstallFilter.Any : InstallFilter.NotInstalled,
        }));
        Favorite = new P10FilterOption("favorite", new P10Command(() => model.Criteria = model.Criteria with { FavoritesOnly = !model.Criteria.FavoritesOnly }));
        Played = new P10FilterOption("played", new P10Command(() => model.Criteria = model.Criteria with { PlayedOnly = !model.Criteria.PlayedOnly }));
        Sources = new P10FilterOption("sources", new P10Command(() => model.OpenSubFilter(P10SubFilterKind.Sources)));
        Genres = new P10FilterOption("genres", new P10Command(() => model.OpenSubFilter(P10SubFilterKind.Genres)));
        Sort = new P10FilterOption("sort", new P10Command(() => model.Criteria = model.Criteria with { Sort = Core.Presentation.LibraryText.NextSort(model.Criteria.Sort) }));
        Clear = new P10FilterOption("clear", new P10Command(() => model.Criteria = model.Criteria.ClearFilters())) { Name = "Limpar filtros" };
        Installed.Name = "Instalados";
        Uninstalled.Name = "Não instalados";
        Favorite.Name = "Favoritos";
        Played.Name = "Jogados recentemente";
    }

    public P10FilterOption Installed { get; }

    public P10FilterOption Uninstalled { get; }

    public P10FilterOption Favorite { get; }

    public P10FilterOption Played { get; }

    /// <summary>Abre a lista de fontes (Steam, Adicionado...).</summary>
    public P10FilterOption Sources { get; }

    /// <summary>Abre a lista de gêneros (só aparece quando os metadados trouxeram gêneros).</summary>
    public P10FilterOption Genres { get; }

    /// <summary>Troca a ordenação (Nome → Último jogado...).</summary>
    public P10FilterOption Sort { get; }

    public P10FilterOption Clear { get; }

    /// <summary>Caixas de seleção, na ordem do painel.</summary>
    public IReadOnlyList<P10FilterOption> Checks => [Installed, Uninstalled, Favorite, Played];

    /// <summary>Botões que abrem uma lista, na ordem do painel.</summary>
    public IReadOnlyList<P10FilterOption> Lists => [Sources, Genres];

    internal void Update(LibraryCriteria criteria, bool hasGenres)
    {
        Installed.IsChecked = criteria.Install == InstallFilter.Installed;
        Uninstalled.IsChecked = criteria.Install == InstallFilter.NotInstalled;
        Favorite.IsChecked = criteria.FavoritesOnly;
        Played.IsChecked = criteria.PlayedOnly;
        Sources.IsChecked = criteria.Sources.Count > 0;
        Sources.Name = criteria.Sources.Count > 0 ? $"Fonte ({criteria.Sources.Count})" : "Fonte";
        Genres.IsChecked = criteria.Genres.Count > 0;
        Genres.Name = criteria.Genres.Count > 0 ? $"Gênero ({criteria.Genres.Count})" : "Gênero";
        Genres.IsVisible = hasGenres || criteria.Genres.Count > 0;
        Sort.Name = $"Ordenar: {Core.Presentation.LibraryText.SortLabel(criteria.Sort)}";
        Clear.IsVisible = criteria.HasFilters;
    }
}

public enum P10SubFilterKind
{
    Sources,
    Genres,
}

/// <summary>
/// Lista de um filtro (o <c>FilterDbItemtSelection</c> do Playnite em <c>PART_ContentFilterItems</c>): uma caixa por
/// fonte ou gênero, Voltar (<c>PART_ButtonBack</c>) e Limpar (<c>PART_ButtonClear</c>).
/// </summary>
public sealed class P10SubFilter
{
    internal P10SubFilter(P10SubFilterKind kind, string title, IReadOnlyList<P10FilterOption> options, ICommand back, ICommand clear)
    {
        Kind = kind;
        Title = title;
        Options = options;
        BackCommand = back;
        ClearCommand = clear;
    }

    public P10SubFilterKind Kind { get; }

    /// <summary>Título da lista (<c>{Binding Title}</c> no template do tema).</summary>
    public string Title { get; }

    public IReadOnlyList<P10FilterOption> Options { get; }

    public ICommand BackCommand { get; }

    public ICommand ClearCommand { get; }

    public override string ToString() => Title;
}
