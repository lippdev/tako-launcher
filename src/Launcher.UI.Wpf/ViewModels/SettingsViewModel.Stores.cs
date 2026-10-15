using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Sources;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Lojas (ADR-0019): ligar e desligar cada loja e ver o que a última leitura encontrou, inclusive cliente
/// ausente (STORE-012). Ativar a linha (A/Enter/clique) liga ou desliga.
/// </summary>
public sealed partial class SettingsViewModel
{
    public bool SteamStoreEnabled => Settings.IsStoreEnabled(GameStore.Steam);

    public bool EpicStoreEnabled => Settings.IsStoreEnabled(GameStore.Epic);

    public bool GogStoreEnabled => Settings.IsStoreEnabled(GameStore.Gog);

    public bool XboxStoreEnabled => Settings.IsStoreEnabled(GameStore.Xbox);

    public string SteamStoreStatus => StoreStatus(GameStore.Steam);

    public string EpicStoreStatus => StoreStatus(GameStore.Epic);

    public string GogStoreStatus => StoreStatus(GameStore.Gog);

    /// <summary>Inclui o aviso de app Xbox ou Serviços de Jogos ausentes (STORE-004/012).</summary>
    public string XboxStoreStatus => StoreStatus(GameStore.Xbox);

    [RelayCommand]
    private void ToggleSteamStore() => ToggleStore(GameStore.Steam);

    [RelayCommand]
    private void ToggleEpicStore() => ToggleStore(GameStore.Epic);

    [RelayCommand]
    private void ToggleGogStore() => ToggleStore(GameStore.Gog);

    [RelayCommand]
    private void ToggleXboxStore() => ToggleStore(GameStore.Xbox);

    // Desligada vale na hora, mesmo antes da próxima leitura das lojas.
    private string StoreStatus(GameStore store) => !Settings.IsStoreEnabled(store)
        ? new LibrarySourceReport(store, Enabled: false, 0, ClientInstalled: false, Notice: null, Failed: false).Summary
        : Shell.LibrarySources.Reports.GetValueOrDefault(store) is { Enabled: true } report ? report.Summary : "Ainda não verificada; use \"Procurar jogos instalados\"";

    private void ToggleStore(GameStore store)
    {
        Shell.ToggleStore(store);
        OnPropertyChanged(string.Empty); // todas as lojas: a leitura nova pode mudar o aviso das outras
    }
}
