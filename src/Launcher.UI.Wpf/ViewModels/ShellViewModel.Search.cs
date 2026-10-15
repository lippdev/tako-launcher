using Launcher.Core.Library;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Busca, filtros e ordenação da biblioteca (frente E: UX-001/002/004). O critério vale para todas as rotas da
/// biblioteca na sessão; trocar de rota pela barra lateral limpa só a busca. Com um tema Playnite na tela inicial, a
/// busca escreve no modelo do tema (<c>PART_ElemSearchActive</c>) e os filtros ficam no painel do próprio tema.
/// </summary>
public sealed partial class ShellViewModel
{
    private LibraryCriteria _libraryCriteria = LibraryCriteria.None;

    /// <summary>Busca, filtros do painel e ordenação da biblioteca do Tako. Mudar atualiza a página na hora.</summary>
    public LibraryCriteria LibraryCriteria
    {
        get => _libraryCriteria;
        set
        {
            ArgumentNullException.ThrowIfNull(value);
            _libraryCriteria = value;
            if (CurrentPage is LibraryViewModel library)
            {
                library.Refresh();
            }
        }
    }

    /// <summary>
    /// Y, "/" ou o botão Buscar: teclado virtual. Na tela de um tema Playnite busca na grade do tema; senão abre (ou
    /// continua em) uma página da biblioteca e busca nela.
    /// </summary>
    public void OpenSearch()
    {
        if (TopOverlay is not null)
        {
            return;
        }

        if (IsPlayniteUiVisible && PlayniteUi is { } ui)
        {
            ui.Model.CloseFilterPanels();
            ui.Model.GameDetailsVisible = false;
            SearchViewModel? themeSearch = null;
            themeSearch = new SearchViewModel(
                ui.Model.SearchText,
                text =>
                {
                    ui.Model.SearchText = text;
                    return ui.Model.Games.Count;
                },
                () =>
                {
                    CloseOverlay(themeSearch!);
                    FocusPlayniteLater(ui.SelectedFocusKey);
                });
            PushOverlay(themeSearch);
            return;
        }

        if (CurrentPage is not LibraryViewModel)
        {
            OpenLibrary(LibraryFilter.All);
        }

        var library = (LibraryViewModel)CurrentPage;
        SearchViewModel? search = null;
        search = new SearchViewModel(
            LibraryCriteria.Search,
            text =>
            {
                LibraryCriteria = LibraryCriteria with { Search = text.Trim() };
                return library.Games.Count;
            },
            () =>
            {
                CloseOverlay(search!);
                RequestFocus(library.DefaultFocusKey);
            });
        PushOverlay(search);
    }

    /// <summary>Painel de filtros da página da biblioteca aberta.</summary>
    public void OpenLibraryFilters()
    {
        if (TopOverlay is not null || CurrentPage is not LibraryViewModel library)
        {
            return;
        }

        var games = Library.Games.Where(library.Filter.Matches).ToList();
        PushOverlay(new LibraryFiltersViewModel(
            games,
            () => LibraryCriteria,
            criteria =>
            {
                LibraryCriteria = criteria;
                return library.Games.Count;
            }));
    }

    /// <summary>A busca do tema Playnite começou ou foi limpa: a dica "Limpar busca" aparece ou some.</summary>
    internal void OnPlayniteSearchChanged() => UpdateHints();

    /// <summary>B na biblioteca com busca ativa: limpa a busca antes de voltar de página.</summary>
    private bool TryClearLibrarySearch()
    {
        if (TopOverlay is not null || CurrentPage is not LibraryViewModel || !LibraryCriteria.HasSearch)
        {
            return false;
        }

        LibraryCriteria = LibraryCriteria with { Search = string.Empty };
        Toast("Busca limpa");
        RequestFocus(CurrentPage.DefaultFocusKey);
        return true;
    }
}
