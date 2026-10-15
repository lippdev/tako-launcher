using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Simulation;
using ConsoleMode.Runtime.Tv;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// MVP-002 parte 2 com áudio e TV simulados: TV configurada e testada nas Configurações, saída de áudio do jogo
    /// escolhida, troca no início do jogo real e volta no fim, TV em espera ao sair.
    /// </summary>
    private static void GameAudioSteamAndTv(ShellViewModel shell, MainWindow window, Game game, SimulatedAudioEndpoints audio, SimulatedTvControl tv)
    {
        var nav = window.Navigator;
        // ADR-0020: TV, áudio e Steam ficam no menu Modo Console, um cartão por recurso.
        shell.OpenConsoleMode();
        Pump(0.8);
        var console = Assert.IsType<ConsoleModeViewModel>(shell.CurrentPage);

        // TV: desligada por padrão; endereço, espera ao sair, Testar e Salvar.
        Assert.False(shell.Settings.TvControl.IsActive);
        Assert.Equal("Desligado", shell.TvSummary);
        Assert.Equal(Launcher.Core.Presentation.FeatureState.Off, console.CardFor("tv").State);
        Assert.True(nav.FocusDefault("console:tv"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        var editor = Assert.IsType<TvSettingsViewModel>(shell.TopOverlay);
        Assert.Equal("tv:enabled", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        Assert.True(editor.Enabled);
        editor.TestCommand.Execute(null);
        Assert.True(editor.StatusIsError); // sem IP
        editor.Host = "192.168.0.50";
        editor.MacAddress = "AA:BB:CC:DD:EE:FF";

        // Tipo de TV: "Detectar pelo IP" acha uma LG; o comando avançado (só Android) some e o texto de ajuda muda.
        Assert.Equal(TvKind.Android, editor.Kind);
        tv.DetectedKind = TvKind.LgWebOs;
        editor.DetectCommand.Execute(null);
        Pump(0.5);
        Assert.Equal(TvKind.LgWebOs, editor.Kind);
        Assert.False(editor.StatusIsError);
        Assert.False(editor.IsAndroid);
        Assert.Contains("LG Connect Apps", editor.Help, StringComparison.Ordinal);
        Assert.Equal(["detectar:192.168.0.50"], tv.Calls);
        Snap(window, "60b-configuracoes-tv-lg");
        editor.Host = "tv.lan";
        editor.SaveCommand.Execute(null);
        Assert.True(editor.StatusIsError); // LG e Samsung pedem o IP
        editor.Host = "192.168.0.50";
        editor.CycleKindCommand.Execute(null);
        Assert.Equal(TvKind.Samsung, editor.Kind);
        editor.CycleKindCommand.Execute(null);
        Assert.Equal(TvKind.Android, editor.Kind);
        Assert.True(editor.IsAndroid);
        tv.Calls.Clear();

        editor.ToggleStandbyCommand.Execute(null);
        editor.TestCommand.Execute(null);
        Pump(0.5);
        Assert.False(editor.StatusIsError);
        Assert.Equal(["ligar:192.168.0.50"], tv.Calls);
        Snap(window, "60-configuracoes-tv");
        editor.SaveCommand.Execute(null);
        Assert.Null(shell.TopOverlay);
        Assert.True(shell.Settings.TvControl is { IsActive: true, StandbyAfterGame: true, HdmiInput: 1, Kind: TvKind.Android });
        Assert.StartsWith("Ligada · 192.168.0.50", shell.TvSummary, StringComparison.Ordinal);
        Pump(0.3);
        Assert.Equal(Launcher.Core.Presentation.FeatureState.On, console.CardFor("tv").State);
        Assert.Equal("console:tv", Nav.GetKey(nav.Focused!)); // o foco volta ao cartão

        // Saída de áudio durante o jogo: lista as saídas ativas; escolher fecha e salva.
        Assert.True(nav.FocusDefault("console:audio"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        var choose = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.Equal("Não alterar ✓", choose.Actions[0].Label);
        Snap(window, "61-saida-de-audio-do-jogo");
        choose.Actions.Single(a => a.Label.StartsWith("Fone", StringComparison.Ordinal)).Command.Execute(null);
        Assert.Null(shell.TopOverlay);
        Assert.Equal("sim-headset", shell.Settings.GameAudioOutput?.DeviceId);

        Pump(0.3);
        Assert.Equal(Launcher.Core.Presentation.FeatureState.On, console.CardFor("audio").State);
        Assert.True(nav.FocusDefault("console:steam"));
        Press(window, AppCommand.Accept); // abre o detalhe do recurso
        Pump(0.5);
        Assert.IsType<SettingsDetailViewModel>(shell.CurrentPage);
        Assert.Equal("console:steam:on", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        Assert.True(shell.Settings.SteamToTrayAfterGame);
        Press(window, AppCommand.Back);
        Pump(0.4);
        Assert.IsType<ConsoleModeViewModel>(shell.CurrentPage);
        Assert.Equal("console:steam", Nav.GetKey(nav.Focused!));
        Snap(window, "62-modo-console-audio-steam-tv");

        // Jogo real: TV ligada e som no fone durante o jogo; ao fechar, a saída volta e a TV vai para espera.
        tv.Calls.Clear();
        shell.OpenDetails(shell.ItemFor(game));
        Pump(0.8);
        Press(window, AppCommand.Accept); // Jogar
        for (int i = 0; i < 10 && audio.Current.DefaultFor(AudioRole.Console) != "sim-headset"; i++)
        {
            Pump(0.5);
        }

        Assert.Equal("sim-headset", audio.Current.DefaultFor(AudioRole.Console));
        Assert.Contains("ligar:192.168.0.50", tv.Calls);
        for (int i = 0; i < 30 && shell.Session != ConsoleMode.Runtime.SessionState.Browsing; i++)
        {
            Pump(1);
        }

        Assert.Equal(ConsoleMode.Runtime.SessionState.Browsing, shell.Session);
        Assert.Equal("sim-speakers", audio.Current.DefaultFor(AudioRole.Console));
        for (int i = 0; i < 10 && !tv.Calls.Contains("espera:192.168.0.50"); i++)
        {
            Pump(0.2);
        }

        Assert.Contains("espera:192.168.0.50", tv.Calls);

        // Volta aos padrões (o teste usa o tako.db da pasta de saída).
        shell.Settings.GameAudioOutput = null;
        shell.Settings.SteamToTrayAfterGame = false;
        shell.Settings.TvControl = TvControlConfig.Default;
    }
}
