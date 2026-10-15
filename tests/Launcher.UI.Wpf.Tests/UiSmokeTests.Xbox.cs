using Launcher.Core.Input;
using Launcher.Core.Library.Xbox;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// STORE-004: a linha "Jogos do Xbox" nas Configurações avisa que falta o app Xbox (máquina simulada sem pacotes) e
    /// liga/desliga a fonte pelo controle.
    /// </summary>
    private static void XboxLibrarySetting(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        Assert.True(shell.Settings.XboxLibraryEnabled);
        Assert.StartsWith("App Xbox não encontrado", shell.XboxLibraryDescription, StringComparison.Ordinal);
        Assert.True(nav.FocusDefault("settings:xbox"));
        Pump(0.4);
        Snap(window, "70-configuracoes-xbox");

        Press(window, AppCommand.Accept);
        Assert.False(shell.Settings.XboxLibraryEnabled);
        Assert.StartsWith("Desligado", shell.XboxLibraryDescription, StringComparison.Ordinal);
        Press(window, AppCommand.Accept);
        Assert.True(shell.Settings.XboxLibraryEnabled);
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
