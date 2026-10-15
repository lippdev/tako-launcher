using System.Windows;
using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Simulation;
using Launcher.Core.Input;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// ADR-0017: "Mostrar FPS" esmaece com o controle (o UAC não aceita o controle); com teclado e mouse explica, pede
    /// a permissão (UAC simulado) e avisa para sair e entrar no Windows. A janela do contador abre sem foco, fora do
    /// Alt+Tab e sem receber clique.
    /// </summary>
    private static void FpsCounterSettings(ShellViewModel shell, MainWindow window, SimulatedPerfLogAccess access)
    {
        var nav = window.Navigator;
        // ADR-0020: o contador fica no menu Modo Console › Contador de FPS, não nas Configurações.
        shell.OpenConsoleMode();
        Pump(0.8);
        var settings = Assert.IsType<ConsoleModeViewModel>(shell.CurrentPage);
        Assert.Equal(Launcher.Core.Presentation.FeatureState.Off, settings.CardFor("fps").State);
        Assert.True(nav.FocusDefault("console:fps"));
        Press(window, AppCommand.Accept);
        Pump(0.6);
        var detail = Assert.IsType<SettingsDetailViewModel>(shell.CurrentPage);
        var toggle = detail.Rows.Single(r => r.Key == "console:fps:on");
        Assert.False(settings.FpsCounterOn);
        Assert.False(settings.CanRevokeFpsPermission);

        // Controle: a linha fica esmaecida e A só explica; nada de UAC.
        Assert.Equal("console:fps:on", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        Assert.True(settings.FpsCounterDimmed);
        Assert.True(toggle.IsDimmed);
        Assert.Equal(ConsoleModeViewModel.UseKeyboardMessage, settings.FpsCounterDetail);
        Assert.Equal(ConsoleModeViewModel.UseKeyboardMessage, shell.ToastMessage);
        Assert.False(settings.FpsCounterOn);
        Assert.Equal(0, access.Prompts);
        Pump(0.4);
        Snap(window, "23-mostrar-fps-controle");

        // Teclado e mouse: explica antes do UAC; "Agora não" não pede nada.
        shell.InputKind = InputKind.Keyboard;
        Assert.False(settings.FpsCounterDimmed);
        toggle.Command!.Execute(null);
        Pump(0.4);
        var dialog = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.StartsWith(ConsoleModeViewModel.WhyPermissionMessage, dialog.Text, StringComparison.Ordinal);
        Snap(window, "24-mostrar-fps-permissao");
        dialog.Actions.Single(a => a.Label == "Agora não").Command.Execute(null);
        Assert.Null(shell.TopOverlay);
        Assert.Equal(0, access.Prompts);

        // UAC cancelado: nada muda.
        access.NextAnswer = PerfLogChange.Cancelled;
        toggle.Command!.Execute(null);
        Assert.IsType<DialogViewModel>(shell.TopOverlay).Actions.Single(a => a.Label == "Continuar").Command.Execute(null);
        Pump(0.4);
        Assert.Equal(1, access.Prompts);
        Assert.False(settings.FpsCounterOn);

        // UAC aceito: ligado, e o Windows só põe o grupo no token no próximo login.
        access.NextAnswer = PerfLogChange.Done;
        toggle.Command!.Execute(null);
        Assert.IsType<DialogViewModel>(shell.TopOverlay).Actions.Single(a => a.Label == "Continuar").Command.Execute(null);
        Pump(0.4);
        Assert.True(settings.FpsCounterOn);
        Assert.Equal(FpsCounterState.NeedsSignOut, shell.SessionTools.CounterState);
        Assert.Equal(ConsoleModeViewModel.SignOutMessage, settings.FpsCounterDetail);
        Assert.Equal(ConsoleModeViewModel.SignOutMessage, shell.ToastMessage);
        Assert.True(settings.CanRevokeFpsPermission);
        Pump(0.3);
        Assert.Equal(Launcher.Core.Presentation.FeatureState.NeedsSetup, detail.Feature.State);
        Assert.True(detail.Rows.Single(r => r.Key == "console:fps:revoke").IsShown);
        Snap(window, "25-mostrar-fps-sair-e-entrar");

        // Depois do login: pronto. Desligar não pede nada; "Remover permissão" pede o UAC e desliga.
        access.Permission = PerfLogStatus.Ready;
        Assert.Equal(PerfLogStatus.Ready, shell.SessionTools.RefreshPermission());
        Assert.Equal(FpsCounterState.Ready, shell.SessionTools.CounterState);
        detail.Rows.Single(r => r.Key == "console:fps:revoke").Command!.Execute(null);
        Pump(0.4);
        Assert.False(settings.FpsCounterOn);
        Assert.Equal(PerfLogStatus.NotGranted, shell.SessionTools.PermissionState);
        Assert.Equal(3, access.Prompts);

        // Janela do contador: sem foco, fora do Alt+Tab, sem clique, no canto pedido.
        var frames = new FrameStats(144, 6.9, 118, "DXGI", 1234);
        var sample = new HardwareSample { CpuUsage = 41, GpuUsage = 97, RamUsedMb = 9200, RamTotalMb = 16384 };
        var overlay = new FpsOverlayWindow();
        overlay.Update(FpsOverlay.Rows(FpsOverlay.Detailed, frames, sample), new FpsOverlayLayout { Position = FpsOverlay.TopRight, Size = FpsOverlay.Medium }, 0);
        Pump(0.4);
        Assert.True(overlay.IsVisible);
        Assert.False(overlay.IsActive);
        Assert.True(overlay.IsClickThroughToolWindow);
        Assert.True(overlay.Left + overlay.ActualWidth > SystemParameters.PrimaryScreenWidth / 2); // canto direito
        overlay.Close();
        shell.InputKind = InputKind.Gamepad;
        Press(window, AppCommand.Back); // de volta aos cartões, com o foco no cartão de origem
        Assert.IsType<ConsoleModeViewModel>(shell.CurrentPage);
        Assert.Equal("console:fps", Nav.GetKey(nav.Focused!));
    }
}
