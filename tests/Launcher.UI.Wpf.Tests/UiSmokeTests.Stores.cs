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
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Steam));
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Epic));
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Gog));
        Assert.False(string.IsNullOrWhiteSpace(settings.StoreStatus(GameStore.Epic)));

        Assert.True(nav.FocusDefault("settings:store:epic"));
        Assert.Equal("biblioteca", settings.SelectedCategory.Id); // a linha estava em outra categoria: ela abre sozinha
        Press(window, AppCommand.Accept);
        Assert.False(shell.Settings.IsStoreEnabled(GameStore.Epic));
        Assert.StartsWith("Epic Games desligada", shell.ToastMessage, StringComparison.Ordinal);
        Assert.Equal("settings:store:epic", Nav.GetKey(nav.Focused!));
        Pump(0.4);
        Snap(window, "75-configuracoes-lojas");

        Press(window, AppCommand.Accept);
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Epic));
        Assert.StartsWith("Epic Games ligada", shell.ToastMessage, StringComparison.Ordinal);

        Assert.True(nav.FocusDefault("settings:store:gog"));
        Assert.Equal("settings:store:gog", Nav.GetKey(nav.Focused!));
    }
}
