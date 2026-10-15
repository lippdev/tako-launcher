using System.Windows;
using System.Windows.Controls;
using ConsoleMode.Runtime.SessionTools;
using ConsoleMode.Runtime.Simulation;
using Launcher.Core.Input;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

/// <summary>
/// Correções da Central de Controle (MVP-002 / UX-005): cartões de janela com títulos longos (sem corte em cima, até 2
/// linhas com reticências), rolagem que mantém o cartão focado inteiro na tela, Central ao vivo sobre o jogo e os
/// atalhos configuráveis.
/// </summary>
public sealed partial class UiSmokeTests
{
    private const double SideMargin = 72;

    /// <summary>
    /// Com a Central aberta (vinda de <see cref="ControlCenterTools"/>): janelas com títulos e nomes longos, → até a
    /// última, capturas em 1080p/1440p/4K e em 16:9, 21:9 e 4:3. Termina no ícone do ControlFS, como começou.
    /// </summary>
    private static void ControlCenterWindowsLayout(MainWindow window, ControlCenterViewModel center, SimulatedWindowSwitcher windows)
    {
        var nav = window.Navigator;
        string[][] longWindows =
        [
            ["CONTROL Resonant — Remedy Entertainment · Edição definitiva com todas as expansões", "CONTROL Resonant"],
            ["Console Mode — Configurações avançadas do modo console e da tela de jogo", "ConsoleMode"],
            ["ControlFS — C:\\Users\\Jogador\\Downloads\\Capturas de tela da sessão de ontem", "ControlFS"],
            ["WhatsApp", "WhatsApp"],
            ["Guia de conquistas com um título muito, muito longo mesmo — Navegador", "UmProgramaComNomeDeProcessoMuitoComprido"],
            ["Discord | #geral | Servidor da comunidade", "Discord"],
            ["Bloco de notas", "notepad"],
        ];
        for (int i = 0; i < longWindows.Length; i++)
        {
            windows.Add(new OpenWindow(200 + i, longWindows[i][0], longWindows[i][1], null));
        }

        int expected = windows.List().Count;
        Assert.Same(center, window.DataContext is ShellViewModel s ? s.TopOverlay : null);
        for (int i = 0; i < 20 && center.Windows.Count != expected; i++)
        {
            center.RefreshWindows(); // a lista chega em segundo plano
            Pump(0.5);
        }

        Assert.True(expected == center.Windows.Count, $"janelas: {center.Windows.Count} na Central, {expected} abertas ({string.Join(", ", center.Windows.Select(w => w.Window.Handle))})");
        Assert.Equal($"JANELAS ABERTAS · {center.Windows.Count}", center.WindowsTitle);

        Pump(0.5);
        Assert.True(nav.FocusDefault(center.Windows[0].NavKey));
        var trail = new List<string>();
        for (int i = 1; i < center.Windows.Count; i++)
        {
            Press(window, AppCommand.Right);
            Pump(0.35);
            trail.Add(Nav.GetKey(nav.Focused!) ?? "?");
            AssertWindowCardLayout(window, "→ na fileira de janelas");
        }

        // Chegou ao último, inteiro na tela. Na falha: o caminho e os cartões navegáveis da fileira (chave e posição).
        var row = nav.Navigables().Where(e => Nav.GetGroup(e) == "qwin")
            .Select(e => $"{Nav.GetKey(e)}@{e.TransformToAncestor(window.StageElement).Transform(default):0}").ToList();
        Assert.True(center.Windows[^1].NavKey == Nav.GetKey(nav.Focused!), $"caminho: {string.Join(" ", trail)} · fileira: {string.Join(" ", row)}");
        Snap(window, "11e-central-janelas-titulos-longos");
        Snap(window, "11f-central-janelas-1440p", scale: 4.0 / 3);
        Snap(window, "11g-central-janelas-4k", scale: 2);

        // Volta ao primeiro: a fileira anda de volta e o primeiro cartão também fica inteiro.
        Assert.True(nav.FocusDefault(center.Windows[0].NavKey));
        Pump(0.5);
        AssertWindowCardLayout(window, "volta ao primeiro");

        // Outras proporções (o palco tem 1080 de altura e a largura da tela): 21:9 e 4:3. Escala 100–200% do Windows não
        // muda o layout: o palco inteiro é escalado (Viewbox), então as capturas acima em 1440p e 4K cobrem a densidade.
        foreach (var (width, name) in new[] { (1680d, "21x9"), (960d, "4x3") })
        {
            window.Width = width;
            Pump(0.6);
            Assert.True(nav.FocusDefault(center.Windows[^1].NavKey));
            Pump(0.6);
            AssertWindowCardLayout(window, $"último cartão em {name}");
            Snap(window, $"11h-central-janelas-{name}");
        }

        window.Width = 1280;
        Pump(0.6);
        Assert.True(nav.FocusDefault("bar:files"));
    }

    /// <summary>O cartão de janela focado: inteiro dentro das margens do palco; título abaixo do ícone, em até 2 linhas.</summary>
    private static void AssertWindowCardLayout(MainWindow window, string step)
    {
        var card = window.Navigator.Focused!;
        Assert.IsType<WindowCardViewModel>(card.DataContext);
        var stage = window.StageElement;
        var bounds = card.TransformToAncestor(stage).TransformBounds(new Rect(0, 0, card.ActualWidth, card.ActualHeight));
        Assert.True(bounds.Left >= SideMargin - 0.5 && bounds.Right <= stage.ActualWidth - SideMargin + 0.5,
            $"{step}: cartão fora da tela ({bounds.Left:0}–{bounds.Right:0} de {stage.ActualWidth:0})");

        var button = (Button)card;
        var icon = (FrameworkElement)button.Template.FindName("IconBox", button);
        var title = (TextBlock)button.Template.FindName("TitleText", button);
        var process = (TextBlock)button.Template.FindName("ProcessText", button);
        var iconBox = icon.TransformToAncestor(card).TransformBounds(new Rect(0, 0, icon.ActualWidth, icon.ActualHeight));
        var titleBox = title.TransformToAncestor(card).TransformBounds(new Rect(0, 0, title.ActualWidth, title.ActualHeight));
        var processBox = process.TransformToAncestor(card).TransformBounds(new Rect(0, 0, process.ActualWidth, process.ActualHeight));
        Assert.True(titleBox.Top >= iconBox.Bottom, $"{step}: título sobre o ícone");
        Assert.True(title.ActualHeight <= 50.5, $"{step}: título com mais de 2 linhas ({title.ActualHeight:0})");
        Assert.True(processBox.Top >= titleBox.Bottom, $"{step}: nome do programa sobre o título");
        Assert.True(processBox.Bottom <= card.ActualHeight, $"{step}: nome do programa cortado");
    }

    /// <summary>
    /// Jogo real (Ping) na frente: o atalho abre a Central ao vivo (janela transparente, sempre no topo, sem a arte do
    /// jogo) e fechar devolve a janela ao normal; em tela cheia exclusiva, a Central abre com a arte (comportamento antigo).
    /// </summary>
    private static void LiveControlCenter(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        Assert.True(shell.CanOpenControlCenterFromBackground);

        window.OpenControlCenterOverGame(0, live: true);
        Pump(0.8);
        Assert.True(shell.IsLiveOverlay);
        Assert.True(window.IsLivePresentation);
        Assert.True(window.Topmost);
        Assert.IsType<ControlCenterViewModel>(shell.TopOverlay);
        Assert.DoesNotContain(shell.Overlays, o => o is GameSessionViewModel); // sem a arte: o jogo aparece atrás
        Assert.Equal("card:game", Nav.GetKey(nav.Focused!));
        Assert.Contains(shell.Hints, h => h.Label == "Fechar" && string.Join(" ", h.Glyphs) == "View Y");
        Snap(window, "21a-central-ao-vivo");

        Press(window, AppCommand.Back);
        Pump(0.6);
        Assert.False(shell.IsLiveOverlay);
        Assert.False(window.IsLivePresentation);
        Assert.False(window.Topmost);
        Assert.Null(shell.TopOverlay);

        // Tela cheia exclusiva: a Central vem com a arte do jogo no fundo, como antes.
        window.OpenControlCenterOverGame(0, live: false);
        Pump(0.8);
        Assert.False(shell.IsLiveOverlay);
        Assert.Contains(shell.Overlays, o => o is GameSessionViewModel { IsRealGame: true });
        Press(window, AppCommand.Guide); // o atalho fecha
        Pump(0.4);
        Assert.Null(shell.TopOverlay);
    }

    /// <summary>
    /// Configurações › atalhos da Central (UX-005): captura do controle ("Segure os botões"), recusa de combinação que
    /// já navega, captura do teclado com conflito (Shift+Tab da Steam) e aceita, e "Restaurar atalhos padrão".
    /// </summary>
    private static void ControlCenterShortcutSettings(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        Assert.Equal(ControlCenterChord.Default, shell.Settings.ControlCenterChord);
        Assert.Equal(KeyShortcut.Default, shell.Settings.ControlCenterKey);
        Assert.Equal("View + Y", shell.ChordLabel);

        // Controle: A na linha abre a captura; o A ainda pressionado é ignorado até soltar.
        Assert.True(nav.FocusDefault("settings:shortcut-pad"));
        Pump(0.4);
        Snap(window, "45-configuracoes-atalhos");
        Press(window, AppCommand.Accept);
        var pad = Assert.IsType<ShortcutCaptureViewModel>(shell.TopOverlay);
        Assert.True(pad.IsGamepad);
        var t = TimeSpan.FromSeconds(100);
        pad.Feed(GamepadButtons.A, t);
        pad.Feed(GamepadButtons.None, t += TimeSpan.FromMilliseconds(50));

        // LB + RB já trocam de seção: recusado com o motivo; a captura recomeça.
        pad.Feed(GamepadButtons.LeftShoulder | GamepadButtons.RightShoulder, t += TimeSpan.FromMilliseconds(50));
        pad.Feed(GamepadButtons.LeftShoulder | GamepadButtons.RightShoulder, t += TimeSpan.FromSeconds(1.1));
        Assert.True(pad.HasError);
        Assert.Contains("já servem para navegar", pad.Error, StringComparison.Ordinal);
        Assert.Equal(ControlCenterChord.Default, shell.Settings.ControlCenterChord);
        Pump(0.3);
        Snap(window, "46-captura-controle-recusada");

        // View + X segurado por 1 s: salvo.
        pad.Feed(GamepadButtons.None, t += TimeSpan.FromMilliseconds(50));
        pad.Feed(GamepadButtons.View | GamepadButtons.X, t += TimeSpan.FromMilliseconds(50));
        pad.Feed(GamepadButtons.View | GamepadButtons.X, t += TimeSpan.FromSeconds(0.5));
        Assert.Equal(0.5, pad.Progress, 2);
        Snap(window, "47-captura-controle"); // sem Pump: a leitura real do controle (nenhum botão) recomeçaria a captura
        pad.Feed(GamepadButtons.View | GamepadButtons.X, t += TimeSpan.FromSeconds(0.6));
        Assert.Equal(GamepadButtons.View | GamepadButtons.X, shell.Settings.ControlCenterChord);
        Assert.Null(shell.TopOverlay);
        Assert.Equal("View + X", shell.ChordLabel);

        // Teclado: Shift+Tab é recusado (Steam); Ctrl+Alt+Shift+D é do Tako; Ctrl+Alt+F10 é aceito.
        Assert.True(nav.FocusDefault("settings:shortcut-key"));
        Press(window, AppCommand.Accept);
        var keys = Assert.IsType<ShortcutCaptureViewModel>(shell.TopOverlay);
        Assert.False(keys.IsGamepad);
        keys.Key("LeftShift", KeyModifiers.Shift);
        keys.Key("Tab", KeyModifiers.Shift);
        Assert.Contains("Steam", keys.Error, StringComparison.Ordinal);
        keys.Key("D", KeyModifiers.Ctrl | KeyModifiers.Alt | KeyModifiers.Shift);
        Assert.Contains("no Tako", keys.Error, StringComparison.Ordinal);
        Pump(0.3);
        Snap(window, "48-captura-teclado-recusada");
        keys.Key("F10", KeyModifiers.Ctrl | KeyModifiers.Alt);
        Assert.Equal(new KeyShortcut(KeyModifiers.Ctrl | KeyModifiers.Alt, "F10"), shell.Settings.ControlCenterKey);
        Assert.Null(shell.TopOverlay);
        Assert.Equal("Ctrl+Alt+F10", shell.KeyLabel);

        // Esc cancela sem mudar nada.
        Pump(0.4);
        Assert.True(nav.FocusDefault("settings:shortcut-key"));
        Press(window, AppCommand.Accept);
        Assert.IsType<ShortcutCaptureViewModel>(shell.TopOverlay).Key("Escape", KeyModifiers.None);
        Assert.Null(shell.TopOverlay);
        Assert.Equal("Ctrl+Alt+F10", shell.KeyLabel);

        // Restaurar atalhos padrão.
        Assert.True(nav.FocusDefault("settings:shortcut-reset"));
        Press(window, AppCommand.Accept);
        Assert.Equal(ControlCenterChord.Default, shell.Settings.ControlCenterChord);
        Assert.Equal(KeyShortcut.Default, shell.Settings.ControlCenterKey);
        Assert.Equal("Ctrl+Shift+T", shell.KeyLabel);
        Assert.True(nav.FocusDefault("settings:theme"));
    }
}
