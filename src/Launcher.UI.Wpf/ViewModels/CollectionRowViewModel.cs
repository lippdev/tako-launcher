using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Presentation;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Fileira de uma coleção na Início (UX-003): título, contagem, capas e "Ver todos" (a biblioteca filtrada).</summary>
public sealed partial class CollectionRowViewModel(ShellViewModel shell, string id) : ObservableObject
{
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Title))]
    private string _name = string.Empty;

    public string Id { get; } = id;

    public string Title => Name;

    /// <summary>Grupo de navegação da fileira (cada coleção é uma fileira separada).</summary>
    public string NavGroup => $"row-col:{Id}";

    public string SeeAllKey => $"home:col:{Id}";

    public ObservableCollection<GameItemViewModel> Games { get; } = [];

    public string CountText => LibraryText.GameCount(Games.Count);

    public void SetGames(IEnumerable<GameItemViewModel> games)
    {
        HomeViewModel.Replace(Games, games);
        OnPropertyChanged(nameof(CountText));
    }

    [RelayCommand]
    private void OpenGame(GameItemViewModel game) => shell.OpenDetails(game);

    [RelayCommand]
    private void SeeAll() => shell.OpenCollection(Id);
}
