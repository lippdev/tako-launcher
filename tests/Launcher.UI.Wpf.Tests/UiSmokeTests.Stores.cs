using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// ADR-0019: Configurações › Lojas. Steam, Epic Games e GOG aparecem com o estado da última leitura; A no controle
    /// desliga a loja (a biblioteca é lida de novo) e A de novo religa.
    /// </summary>
    private static void StoreSettings(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        shell.OpenSettings();
        Pump(0.8);
        var settings = Assert.IsType<SettingsViewModel>(shell.CurrentPage);
        Assert.True(settings.SteamStoreEnabled);
        Assert.True(settings.EpicStoreEnabled);
        Assert.True(settings.GogStoreEnabled);
        Assert.False(string.IsNullOrWhiteSpace(settings.EpicStoreStatus));

        Assert.True(nav.FocusDefault("settings:store:epic"));
        Press(window, AppCommand.Accept);
        Assert.False(settings.EpicStoreEnabled);
        Assert.False(shell.Settings.IsStoreEnabled(GameStore.Epic));
        Assert.StartsWith("Epic Games desligada", shell.ToastMessage, StringComparison.Ordinal);
        Assert.Equal("settings:store:epic", Nav.GetKey(nav.Focused!));
        Pump(0.4);
        Snap(window, "75-configuracoes-lojas");

        Press(window, AppCommand.Accept);
        Assert.True(settings.EpicStoreEnabled);
        Assert.StartsWith("Epic Games ligada", shell.ToastMessage, StringComparison.Ordinal);

        Assert.True(nav.FocusDefault("settings:store:gog"));
        Assert.Equal("settings:store:gog", Nav.GetKey(nav.Focused!));
    }
}
