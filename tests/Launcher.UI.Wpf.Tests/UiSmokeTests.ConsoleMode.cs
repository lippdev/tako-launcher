using System.IO;
using System.Windows;
using ConsoleMode.Runtime.Simulation;
using Launcher.Core.Input;
using Launcher.Core.Session.Devices;
using Launcher.Core.Session.GameScreen;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// Modo console ao abrir (MVP-002/012/036) com a mesa simulada: a opção nas Configurações (desligada por padrão),
    /// a abertura indo para a tela de jogo com "Você consegue ver esta tela?" na primeira vez, "Voltar ao PC" no menu
    /// com confirmação e a volta fora da thread da UI, a tela "Voltando para a mesa…", o atalho de emergência e a
    /// recuperação depois de uma queda. Termina com tudo desligado para não afetar o resto.
    /// </summary>
    private static void ConsoleModeOnLaunch(ShellViewModel shell, MainWindow window, SimulatedEmergencyHotkey hotkey, ConsoleModeFiles files)
    {
        var nav = window.Navigator;
        var size = (window.Left, window.Top, window.Width, window.Height);
        Assert.True(shell.StartEmergencyHotkey());
        Assert.True(hotkey.IsStarted);

        // Modo Console › Tela de jogo (ADR-0020): a opção aparece com a tela de jogo ligada e começa desligada.
        shell.OpenConsoleMode();
        Pump(0.8);
        ((ConsoleModeViewModel)shell.CurrentPage).OpenGameScreen();
        Pump(0.6);
        var detail = Assert.IsType<SettingsDetailViewModel>(shell.CurrentPage);
        Assert.False(shell.Settings.ConsoleModeOnLaunch);
        Assert.True(nav.FocusDefault("console:gamescreen:on"));
        Press(window, AppCommand.Accept);
        Pump(0.4);
        Assert.True(shell.Settings.GameScreenEnabled);
        Assert.True(nav.FocusDefault("console:gamescreen:onlaunch"));
        Press(window, AppCommand.Accept);
        Assert.True(shell.Settings.ConsoleModeOnLaunch);
        Assert.Contains(shell.EmergencyKeys, detail.Rows.Single(r => r.Key == "console:gamescreen:onlaunch").Description, StringComparison.Ordinal);
        Pump(0.4);
        Snap(window, "45-configuracoes-modo-console");

        // Abertura: vai para a tela de jogo; na primeira vez com a tela, pergunta se dá para ver.
        shell.GoHome();
        Pump(0.5);
        var enter = shell.StartConsoleModeAsync(safeMode: false);
        Until(() => shell.TopOverlay is ScreenConfirmViewModel);
        Assert.Equal("overlay", Nav.GetGroup(nav.Focused!));
        Snap(window, "46-modo-console-confirmar");
        Press(window, AppCommand.Accept); // Sim, manter
        Until(() => enter.IsCompleted);
        Assert.True(shell.IsConsoleModeActive);
        Assert.True(File.Exists(files.Marker));
        Assert.Contains(shell.Settings.GameScreen!.SetupKey, shell.Settings.GameScreenConfirmedSetup, StringComparison.Ordinal);
        Assert.False(shell.SidebarItems.Single(s => s.Key == "desk").IsHidden);
        Pump(0.4);
        Snap(window, "47-modo-console-ativo");

        // Jogos dentro do modo console não focam nem restauram a tela de jogo de novo.
        var steps = ConsoleModePolicy.ForGame(shell.IsConsoleModeActive, shell.ResolveSessionSettings("smoke:any"), new SessionDeviceOptions(null, false, shell.Settings.TvControl), shell.Settings.GameScreen, shell.Settings.GameAudioOutput);
        Assert.False(steps.FocusScreen);

        // "Voltar ao PC" no menu: pergunta antes; a volta roda fora da thread da UI.
        Assert.True(nav.FocusDefault("desk"));
        Press(window, AppCommand.Accept);
        var confirm = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.Equal("Voltar ao PC?", confirm.Title);
        Snap(window, "48-voltar-ao-pc");
        Press(window, AppCommand.Accept);
        WaitForNoOverlay(shell);
        Assert.False(shell.IsConsoleModeActive);
        Assert.False(File.Exists(files.Marker));
        Assert.False(File.Exists(files.Environment));
        Assert.True(shell.SidebarItems.Single(s => s.Key == "desk").IsHidden);

        // A tela de andamento não tem ação e B não a interrompe (as telas ficariam pela metade).
        var progress = new ProgressOverlayViewModel("Voltando para a mesa…", "Telas, áudio e TV voltando a como estavam");
        shell.PushOverlay(progress);
        Pump(0.5);
        Snap(window, "49-voltando-para-a-mesa");
        Press(window, AppCommand.Back);
        Assert.Same(progress, shell.TopOverlay);
        shell.CloseOverlay(progress);

        // Atalho de emergência: de novo no modo console (já confirmado, sem pergunta) e Ctrl+Alt+Shift+D.
        var again = shell.EnterConsoleModeAsync();
        Until(() => again.IsCompleted);
        Assert.True(again.Result);
        Assert.True(shell.IsConsoleModeActive);
        hotkey.PressAsync().GetAwaiter().GetResult();
        Until(() => !shell.IsConsoleModeActive && shell.SidebarItems.Single(s => s.Key == "desk").IsHidden);
        Assert.Contains(shell.EmergencyKeys, shell.ToastMessage, StringComparison.Ordinal);
        Assert.False(File.Exists(files.Marker));

        // Queda no modo console: a próxima abertura volta para a mesa sem perguntar.
        Assert.True(new ConsoleModeMarkerStore(files.Marker).TryWrite(new ConsoleModeMarker(DateTimeOffset.Now, null)));
        var recover = shell.RecoverConsoleModeAsync();
        Until(() => recover.IsCompleted);
        Assert.False(File.Exists(files.Marker));
        Assert.StartsWith("O Tako fechou no modo console", shell.ToastMessage, StringComparison.Ordinal);
        Assert.Null(shell.TopOverlay);

        // Volta aos padrões (o teste usa o tako.db da pasta de saída) e ao tamanho de antes da tela de jogo.
        shell.Settings.ConsoleModeOnLaunch = false;
        shell.Settings.GameScreenEnabled = false;
        shell.Settings.GameScreenId = string.Empty;
        shell.Settings.GameScreenName = string.Empty;
        shell.Settings.GameScreenConfirmedSetup = string.Empty;
        window.WindowState = WindowState.Normal;
        (window.Left, window.Top, window.Width, window.Height) = size;
        shell.GoHome();
        Pump(0.8);
    }

    /// <summary>Espera o trabalho fora da thread da UI terminar (a tela de andamento sai sozinha).</summary>
    private static void WaitForNoOverlay(ShellViewModel shell) => Until(() => shell.TopOverlay is null && !shell.IsReturningToDesk);

    private static void Until(Func<bool> condition, double seconds = 15)
    {
        for (double waited = 0; waited < seconds && !condition(); waited += 0.1)
        {
            Pump(0.1);
        }

        Assert.True(condition());
    }
}
