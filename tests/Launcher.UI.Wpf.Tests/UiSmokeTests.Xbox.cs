using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Xbox;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// STORE-004 / ADR-0019: a linha "Xbox (PC Game Pass)" em Configurações › Lojas avisa que falta o app Xbox (máquina
    /// simulada sem pacotes) e liga/desliga a fonte pelo controle.
    /// </summary>
    private static void XboxLibrarySetting(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        var settings = Assert.IsType<SettingsViewModel>(shell.CurrentPage);
        // A biblioteca fictícia não lê as lojas: lê agora, com o Windows simulado sem app Xbox.
        shell.LibrarySources.Discover();
        settings.RescanCommand.Execute(null);
        Assert.True(settings.XboxStoreEnabled);
        Assert.StartsWith("App Xbox não encontrado", settings.XboxStoreStatus, StringComparison.Ordinal);
        Assert.True(nav.FocusDefault("settings:store:xbox"));
        Pump(0.4);
        Snap(window, "70-configuracoes-xbox");

        Press(window, AppCommand.Accept);
        Assert.False(settings.XboxStoreEnabled);
        Assert.False(shell.Settings.IsStoreEnabled(GameStore.Xbox));
        Assert.StartsWith("Desligada", settings.XboxStoreStatus, StringComparison.Ordinal);
        Press(window, AppCommand.Accept);
        Assert.True(settings.XboxStoreEnabled);
    }

    /// <summary>Windows simulado sem app Xbox nem jogos: o teste nunca lê o registro da máquina.</summary>
    private sealed class EmptyXboxSystem : IXboxSystem
    {
        public IReadOnlyList<string> DriveRoots() => [];

        public IReadOnlyList<InstalledPackage>? UserPackages() => [];

        public IReadOnlyCollection<string> GamingServicesPackages() => [];

        public bool GamingServicesInstalled() => false;

        public string? LoadIndirectString(string indirect) => null;
    }
}
