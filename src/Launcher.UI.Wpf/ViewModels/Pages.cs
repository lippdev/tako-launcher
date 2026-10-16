using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Presentation;
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

    /// <summary>A página entrou na tela (nova ou de volta pelo histórico).</summary>
    public virtual void OnShown()
    {
    }

    /// <summary>A página saiu da tela (pode voltar pelo histórico).</summary>
    public virtual void OnHidden()
    {
    }

    /// <summary>Busca com teclado virtual (UX-001): na Início, abre Todos os jogos com o teclado.</summary>
    [RelayCommand]
    private void Search() => Shell.OpenSearch();
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

    /// <summary>Uma fileira por coleção do usuário com jogos (UX-003), em ordem de nome.</summary>
    public ObservableCollection<CollectionRowViewModel> Collections { get; } = [];

    public string RecentCountText => $"{RecentlyPlayed.Count} jogos";

    public string HeroOverline => Hero is not null && RecentlyPlayed.Count > 0 && RecentlyPlayed[0] == Hero ? "Continuar jogando" : Hero?.Overline ?? string.Empty;

    public string PlayLabel => Hero is not null && Hero.IsRunning ? "Voltar ao jogo" : "Jogar";

    public override void OnGameFocused(GameItemViewModel game) => Hero = game;

    /// <summary>Sem jogos: a Início mostra como começar (Steam não encontrada ou nada instalado).</summary>
    public bool IsEmpty => Shell.Library.Games.Count == 0;

    [RelayCommand]
    private void AddGame() => Shell.AddGame();

    [RelayCommand]
    private void Rescan()
    {
        Shell.RescanLibrary();
        OnPropertyChanged(string.Empty); // avisos das lojas (Configurações › Lojas)
    }

    public override void Refresh()
    {
        OnPropertyChanged(nameof(IsEmpty));
        Replace(RecentlyPlayed, LibraryQuery.RecentlyPlayed(Shell.Library.Games).Select(Shell.ItemFor));
        Replace(RecentlyAdded, LibraryQuery.RecentlyAdded(Shell.Library.Games, 10).Select(Shell.ItemFor));
        RefreshCollections();
        OnPropertyChanged(nameof(RecentCountText));
        // Mantém o jogo em foco no destaque; na primeira montagem, abre no último jogado.
        bool keep = !_resetHero && Hero is not null && (RecentlyPlayed.Contains(Hero) || RecentlyAdded.Contains(Hero) || Collections.Any(c => c.Games.Contains(Hero)));
        Hero = keep ? Hero : RecentlyPlayed.FirstOrDefault() ?? RecentlyAdded.FirstOrDefault();
        _resetHero = false;
        OnPropertyChanged(nameof(HeroOverline));
        OnPropertyChanged(nameof(PlayLabel));
    }

    /// <summary>Fileiras das coleções: as mesmas fileiras continuam (o foco não se perde); só muda o que mudou.</summary>
    private void RefreshCollections()
    {
        var wanted = Shell.Collections.Collections
            .Select(c => (Collection: c, Games: LibraryQuery.Apply(Shell.Library.Games.Where(g => c.Contains(g.Id)), LibraryFilter.All, LibrarySort.RecentlyPlayed)))
            .Where(c => c.Games.Count > 0)
            .ToList();
        if (!Collections.Select(r => r.Id).SequenceEqual(wanted.Select(w => w.Collection.Id)))
        {
            Collections.Clear();
            foreach (var (collection, _) in wanted)
            {
                Collections.Add(new CollectionRowViewModel(Shell, collection.Id));
            }
        }

        for (int i = 0; i < wanted.Count; i++)
        {
            Collections[i].Name = wanted[i].Collection.Name;
            Collections[i].SetGames(wanted[i].Games.Select(Shell.ItemFor));
        }
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

/// <summary>
/// Grade da biblioteca de uma rota da barra lateral, com busca, filtros e ordenação (<see cref="ShellViewModel.LibraryCriteria"/>).
/// O cabeçalho mostra o que está valendo: o texto buscado, quantos filtros e a ordem.
/// </summary>
public sealed partial class LibraryViewModel : PageViewModel
{
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

    public LibraryCriteria Criteria => Shell.LibraryCriteria;

    /// <summary>"19 jogos" ou, com busca/filtro, "3 de 19 jogos".</summary>
    public string CountText => Shell.Library.Games.Count(Filter.Matches) is int total && HasCriteria && total > 0
        ? $"{Games.Count} de {LibraryText.GameCount(total)}"
        : LibraryText.GameCount(Games.Count);

    public string SortLabel => LibraryText.SortLabel(Criteria.Sort);

    public bool HasSearch => Criteria.HasSearch;

    public bool HasFilters => Criteria.HasFilters;

    public bool HasCriteria => HasSearch || HasFilters;

    /// <summary>Texto do botão de busca: o que foi buscado, ou "Buscar".</summary>
    public string SearchLabel => HasSearch ? $"“{Criteria.Search}”" : "Buscar";

    public string FiltersLabel => LibraryText.FiltersLabel(Criteria.ActiveFilterCount);

    public bool IsEmptyResult => Games.Count == 0;

    public string EmptyTitle => HasCriteria ? "Nenhum jogo encontrado" : "Nenhum jogo aqui ainda";

    public string EmptyText => (HasSearch, HasFilters) switch
    {
        (true, true) => "Tente outra busca ou limpe os filtros.",
        (true, false) => "Confira o nome ou tente só uma parte dele. Acentos e maiúsculas não importam.",
        (false, true) => "Nenhum jogo combina com os filtros escolhidos.",
        _ => "Adicione um jogo ou procure os jogos instalados em Configurações › Biblioteca e lojas.",
    };

    /// <summary>A biblioteca abre na primeira capa; sem resultados, na busca.</summary>
    public override string? DefaultFocusKey => Games.Count > 0 ? $"grid:{Games[0].Id}" : "lib:search";

    public override void OnGameFocused(GameItemViewModel game) => Shell.SetBackdrop(game);

    public override void Refresh()
    {
        HomeViewModel.Replace(Games, LibraryQuery.Apply(Shell.Library.Games, Filter, Criteria).Select(Shell.ItemFor));
        foreach (string name in (string[])[nameof(Criteria), nameof(CountText), nameof(SortLabel), nameof(HasSearch), nameof(HasFilters), nameof(HasCriteria),
            nameof(SearchLabel), nameof(FiltersLabel), nameof(IsEmptyResult), nameof(EmptyTitle), nameof(EmptyText)])
        {
            OnPropertyChanged(name);
        }
    }

    [RelayCommand]
    private void OpenGame(GameItemViewModel game) => Shell.OpenDetails(game);

    [RelayCommand]
    private void AddGame() => Shell.AddGame();

    [RelayCommand]
    private void CycleSort()
    {
        Shell.LibraryCriteria = Criteria with { Sort = LibraryText.NextSort(Criteria.Sort) };
        Shell.RequestFocus("lib:sort");
    }

    [RelayCommand]
    private void OpenFilters() => Shell.OpenLibraryFilters();

    /// <summary>Limpa a busca e os filtros (a ordem continua).</summary>
    [RelayCommand]
    private void ClearCriteria()
    {
        Shell.LibraryCriteria = Criteria.ClearFilters() with { Search = string.Empty };
        Shell.RequestFocus(DefaultFocusKey);
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
        (_, { IsRom: true }) => "Abrir no emulador (em breve)",
        (_, { IsInstalled: true }) => "Jogar",
        (_, { Store: GameStore.Manual }) => "Arquivo não encontrado",
        _ => $"Instalar {GameText.StoreVia(Game.Game.Store)}",
    };

    public bool PrimaryHasPlayIcon => Game.Game.CanPlay;

    public string FavoriteLabel => Game.IsFavorite ? "Favorito" : "Favoritar";

    public string FavoriteIconKey => Game.IsFavorite ? "Icon.StarFill" : "Icon.Star";

    /// <summary>Aviso discreto quando o jogo está sem dados e a IGDB poderia trazê-los (ADR-0021).</summary>
    public string? MetadataHint => Shell.MetadataHintFor(Game.Game);

    /// <summary>"Coleções" ou, com o jogo em alguma, "Coleções (2)" (UX-003).</summary>
    public string CollectionsLabel => Game.Game.Collections.Count is int n and > 0 ? $"Coleções ({n})" : "Coleções";

    public override void Refresh()
    {
        var g = Game.Game;
        HomeViewModel.Replace(Stats, new[]
        {
            new StatViewModel("Tempo de jogo", Game.PlayTimeText),
            new StatViewModel("Última vez", Game.LastPlayedText),
            Game.MetacriticText is null ? null : new StatViewModel("Metacritic", Game.MetacriticText),
            Game.MetacriticText is null && g.CriticScore is int critic ? new StatViewModel("Crítica", critic.ToString(System.Globalization.CultureInfo.InvariantCulture)) : null,
            g.SizeLabel is null ? null : new StatViewModel("Tamanho", g.SizeLabel),
            Game.ReleaseText is null ? null : new StatViewModel("Lançamento", Game.ReleaseText),
            g.Developer is null ? null : new StatViewModel("Desenvolvedora", g.Developer),
            g.Franchise is null ? null : new StatViewModel("Série", g.Franchise),
            Core.Library.Metadata.ScreenScraper.ScreenScraperSystems.ForKey(g.Platform) is { } system ? new StatViewModel("Sistema", system.Name) : null,
            g.Players is null ? null : new StatViewModel("Jogadores", g.Players),
            new StatViewModel("Status", Game.Status),
            g.AlsoInstalledIn.Count == 0 ? null : new StatViewModel("Também em", string.Join(", ", g.AlsoInstalledIn.Select(Core.Presentation.GameText.StoreName))),
        }.OfType<StatViewModel>());
        OnPropertyChanged(nameof(PrimaryLabel));
        OnPropertyChanged(nameof(FavoriteLabel));
        OnPropertyChanged(nameof(FavoriteIconKey));
        OnPropertyChanged(nameof(CollectionsLabel));
        OnPropertyChanged(nameof(MetadataHint));
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
        else if (g.IsRom)
        {
            Shell.Toast(ShellViewModel.RomLaunchNotice);
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

    [RelayCommand]
    private void ChooseCollections() => Shell.ChooseCollections(Game);

    [RelayCommand]
    private void GameSettings() => Shell.OpenGameSettings(Game);
}
