using System.Globalization;
using CommunityToolkit.Mvvm.Input;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Configurações › Coleções (UX-003): resumo na linha; A abre criar, renomear e excluir.</summary>
public sealed partial class SettingsViewModel
{
    public string CollectionsSummary => Shell.Collections.Collections.Count switch
    {
        0 => "Criar",
        1 => "1 coleção",
        int n => string.Create(CultureInfo.InvariantCulture, $"{n} coleções"),
    };

    /// <summary>Uma coleção foi criada, renomeada ou excluída.</summary>
    public void OnCollectionsChanged() => OnPropertyChanged(nameof(CollectionsSummary));

    [RelayCommand]
    private void ManageCollections() => Shell.ManageCollections();
}
