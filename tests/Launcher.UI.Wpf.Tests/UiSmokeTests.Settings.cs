using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using Launcher.Core.Input;
using Launcher.Core.Presentation;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Controls;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>Topo da barra de dicas no palco de 1080 (88 de altura).</summary>
    private const double HintBarTop = 1080 - 88;

    /// <summary>
    /// ADR-0020 (UX-006/009): Configurações em categorias e o menu Modo Console, só com o controle. Em cada categoria,
    /// desce por todas as linhas conferindo que a linha focada fica inteira e acima da barra de dicas; B volta às
    /// categorias, LB/RB trocam de categoria; as opções de desenvolvedor ficam escondidas até serem ligadas. No Modo
    /// Console, percorre os cartões e o assistente "Configurar para jogar na TV" do começo ao fim. Repete as capturas em
    /// 4:3, 16:10, 21:9 e 4K, com a barra de dicas inteira em todas.
    /// </summary>
    private static void SettingsAndConsoleTour(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        var placement = (window.Left, window.Top, window.Width, window.Height);
        shell.OpenSettings();
        Pump(0.8);
        var settings = Assert.IsType<SettingsViewModel>(shell.CurrentPage);
        Assert.Equal("settings-cat:geral", Nav.GetKey(nav.Focused!));
        Assert.Equal(["geral", "biblioteca", "aparencia", "controles", "sistema", "sobre"], settings.Categories.Select(c => c.Id));

        // Nenhum texto de desenvolvedor visível: sem IDs internos do PRD/ADR nem nomes técnicos.
        foreach (var row in settings.Categories.SelectMany(c => c.Rows))
        {
            string text = $"{row.Title} {row.Description} {row.ValueText}";
            Assert.DoesNotMatch(@"MVP-\d|ADR-\d|\(H\d\)|Marco \d|carregador XAML|ThemeApiVersion", text);
        }

        // Cada categoria: ↓ na coluna mostra o conteúdo; → entra; ↓ até a última linha, sempre inteira e acima das dicas.
        for (int i = 0; i < settings.Categories.Count; i++)
        {
            var category = settings.Categories[i];
            Assert.True(nav.FocusDefault(category.NavKey));
            Pump(0.3);
            Assert.Same(category, settings.SelectedCategory);
            Snap(window, $"cfg80-configuracoes-{i + 1}-{category.Id}");
            Press(window, AppCommand.Right);
            Assert.Equal(category.Rows.First(r => !r.IsHeader).Key, Nav.GetKey(nav.Focused!)); // → entra pela primeira linha
            WalkRows(window, category.Rows.Count(r => r.IsShown && !r.IsHeader), $"Configurações › {category.Title}");
            Snap(window, $"cfg81-configuracoes-{i + 1}-{category.Id}-fim");
            Press(window, AppCommand.Back);
            Assert.Equal(category.NavKey, Nav.GetKey(nav.Focused!)); // B no conteúdo volta à categoria
            AssertHintBarFits(window, $"Configurações › {category.Title}");
        }

        // LB/RB trocam de categoria (dão a volta) e a dica do rodapé diz isso.
        Assert.True(nav.FocusDefault("settings-cat:geral"));
        Press(window, AppCommand.NextSection);
        Assert.Equal("biblioteca", settings.SelectedCategory.Id);
        Assert.Equal("settings-cat:biblioteca", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.PreviousSection);
        Press(window, AppCommand.PreviousSection);
        Assert.Equal("sobre", settings.SelectedCategory.Id);
        Assert.Contains(shell.Hints, h => h.Label == SettingsViewModel.CategoryHint);

        // Opções de desenvolvedor: desligadas por padrão, escondem recarregar tema, prévia do Playnite e teste de recuperação.
        Assert.False(shell.Settings.DeveloperOptions);
        var system = settings.Category("sistema")!;
        Assert.All(system.Rows.Where(r => r.Key is "settings:theme" or "settings:playnite" or "settings:crash"), r => Assert.False(r.IsShown));
        Assert.True(nav.FocusDefault("settings:developer"));
        Press(window, AppCommand.Accept);
        Pump(0.3);
        Assert.True(shell.Settings.DeveloperOptions);
        Assert.All(system.Rows.Where(r => r.Key is "settings:theme" or "settings:playnite" or "settings:crash"), r => Assert.True(r.IsShown));
        Press(window, AppCommand.Down);
        AssertInsideBand(window, "Opções de desenvolvedor");
        Snap(window, "cfg82-configuracoes-desenvolvedor");
        Assert.True(nav.FocusDefault("settings:developer"));
        Press(window, AppCommand.Accept);
        Assert.False(shell.Settings.DeveloperOptions);

        // Segredos (IGDB e ScreenScraper) aparecem como ● até pedir "Mostrar"; as duas linhas ficam em Biblioteca e lojas.
        Assert.True(nav.FocusDefault("settings:screenscraper"));
        Assert.Equal("biblioteca", settings.SelectedCategory.Id);
        Assert.True(nav.FocusDefault("settings:igdb"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        var igdb = Assert.IsType<IgdbSettingsViewModel>(shell.TopOverlay);
        Assert.False(igdb.IsSecretVisible);
        igdb.ClientSecret = "segredo-de-teste";
        Pump(0.3);
        Snap(window, "cfg82b-segredo-mascarado");
        igdb.ToggleSecretVisibleCommand.Execute(null);
        Assert.True(igdb.IsSecretVisible);
        Assert.Equal("Ocultar", igdb.SecretToggleLabel);
        igdb.CancelCommand.Execute(null);
        Pump(0.3);
        Assert.Null(shell.TopOverlay);

        // --- Modo Console: assistente no topo e um cartão por recurso, com o estado.
        shell.OpenConsoleMode();
        Pump(0.8);
        var console = Assert.IsType<ConsoleModeViewModel>(shell.CurrentPage);
        Assert.Equal("console:wizard", Nav.GetKey(nav.Focused!));
        Assert.Equal(["console:gamescreen", "console:tv", "console:audio", "console:steam", "console:fps", "console:fpslimit"], console.Cards.Select(c => c.Key));
        Assert.All(console.Cards, c => Assert.NotNull(c.State));
        Assert.Equal(FeatureState.Off, console.CardFor("gamescreen").State);
        Snap(window, "cfg83-modo-console");
        Press(window, AppCommand.Down);
        Assert.Equal("console:gamescreen", Nav.GetKey(nav.Focused!)); // ↓ do assistente vai ao primeiro cartão
        foreach (var card in console.Cards)
        {
            Assert.True(nav.FocusDefault(card.Key));
            Pump(0.35);
            AssertInsideBand(window, $"Modo Console › {card.Title}");
        }

        Snap(window, "cfg84-modo-console-cartoes");
        AssertHintBarFits(window, "Modo Console");

        // Assistente: tela (a TV é sugerida) → testar a imagem (confirma na "TV") → pular a TV → áudio → pronto.
        Assert.True(nav.FocusDefault("console:wizard"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        var wizard = Assert.IsType<SetupWizardViewModel>(shell.TopOverlay);
        Assert.Equal("Passo 1 de 5", wizard.StepText);
        Assert.Equal("TV da sala", Nav.GetKey(nav.Focused!));
        Snap(window, "cfg85-assistente-tela");
        Press(window, AppCommand.Accept);
        Pump(0.4);
        Assert.Equal("Passo 2 de 5", wizard.StepText);
        Assert.True(shell.Settings.GameScreenEnabled);
        Assert.Equal("SIM\\TV", shell.Settings.GameScreenId);
        Snap(window, "cfg86-assistente-imagem");
        string before = Nav.GetKey(nav.Focused!) ?? "-";
        Press(window, AppCommand.Accept); // Testar agora
        for (double waited = 0; waited < 20 && shell.TopOverlay is not ScreenConfirmViewModel && wizard.Title == "Vamos testar a imagem"; waited += 0.1)
        {
            Pump(0.1);
        }

        Assert.True(shell.TopOverlay is ScreenConfirmViewModel,
            $"Sem a pergunta na tela de jogo: foco antes={before} topo={shell.TopOverlay?.GetType().Name} ativa={shell.GameScreen.IsActive} {wizard.StepText} · {wizard.Title} · {wizard.Text}");
        Snap(window, "cfg87-assistente-confirmar");
        Press(window, AppCommand.Accept); // Sim, manter
        Until(() => wizard.StepText == "Passo 3 de 5");
        Assert.False(shell.GameScreen.IsActive); // o teste devolve as telas
        Assert.False(string.IsNullOrEmpty(shell.Settings.GameScreenConfirmedSetup));
        Snap(window, "cfg88-assistente-tv");
        Press(window, AppCommand.Back); // B volta um passo
        Assert.Equal("Passo 2 de 5", wizard.StepText);
        wizard.Actions.Single(a => a.Label == "Pular").Command.Execute(null);
        Assert.Equal("Passo 3 de 5", wizard.StepText);
        wizard.Actions.Single(a => a.Label == "Pular").Command.Execute(null);
        Pump(0.4);
        Assert.Equal("Passo 4 de 5", wizard.StepText);
        Snap(window, "cfg89-assistente-audio");
        Press(window, AppCommand.Accept); // a saída sugerida (ou "Não alterar o som")
        Pump(0.4);
        Assert.Equal("Passo 5 de 5", wizard.StepText);
        Snap(window, "cfg90-assistente-pronto");
        Press(window, AppCommand.Accept); // Concluir
        Pump(0.5);
        Assert.Null(shell.TopOverlay);
        (window.Left, window.Top, window.Width, window.Height) = placement; // o teste da imagem moveu o Tako para a "TV"
        Pump(0.8);
        Assert.Equal(FeatureState.On, console.CardFor("gamescreen").State);
        Snap(window, "cfg91-modo-console-configurado");

        // Detalhe da Tela de jogo: todas as linhas aparecem com a tela ligada; B volta ao cartão.
        Assert.True(nav.FocusDefault("console:gamescreen"));
        Press(window, AppCommand.Accept);
        Pump(0.6);
        var detail = Assert.IsType<SettingsDetailViewModel>(shell.CurrentPage);
        Assert.All(detail.Rows, r => Assert.True(r.IsShown, r.Key));
        WalkRows(window, detail.Rows.Count, "Modo Console › Tela de jogo");
        Snap(window, "cfg92-modo-console-tela-de-jogo");
        Press(window, AppCommand.Back);
        Assert.Equal("console:gamescreen", Nav.GetKey(nav.Focused!));

        // Proporções e escalas: nada cortado e a barra de dicas inteira (teclado: glifos mais largos).
        shell.InputKind = InputKind.Keyboard;
        foreach (var (width, height, name) in new[] { (720.0, 540.0, "4x3"), (576.0, 360.0, "16x10"), (840.0, 360.0, "21x9") })
        {
            window.Width = width;
            window.Height = height;
            shell.OpenSettings();
            Pump(1);
            Assert.True(nav.FocusDefault("settings:fse"));
            Pump(0.5);
            AssertInsideBand(window, $"Configurações {name}");
            AssertHintBarFits(window, $"Configurações {name}");
            Snap(window, $"cfg93-configuracoes-{name}");
            shell.OpenConsoleMode();
            Pump(1);
            Assert.True(nav.FocusDefault("console:fpslimit"));
            Pump(0.5);
            AssertInsideBand(window, $"Modo Console {name}");
            AssertHintBarFits(window, $"Modo Console {name}");
            Snap(window, $"cfg94-modo-console-{name}");
        }

        window.Width = 640; // 16:9
        window.Height = 360;
        shell.OpenSettings();
        Pump(1);
        Assert.True(nav.FocusDefault("settings:store:xbox"));
        Pump(0.5);
        Snap(window, "cfg95-configuracoes-4k", scale: 2);
        shell.InputKind = InputKind.Gamepad;
        (window.Left, window.Top, window.Width, window.Height) = placement;
        Pump(0.8);

        // Volta aos padrões (o teste usa o tako.db da pasta de saída).
        shell.Settings.GameScreenEnabled = false;
        shell.Settings.GameScreenId = string.Empty;
        shell.Settings.GameScreenName = string.Empty;
        shell.Settings.GameScreenConfirmedSetup = string.Empty;
        shell.Settings.GameAudioOutput = null;
        shell.GoHome();
        Pump(0.5);
    }

    /// <summary>↓ por todas as linhas visíveis: cada uma fica inteira e acima da barra de dicas.</summary>
    private static void WalkRows(MainWindow window, int count, string screen)
    {
        var nav = window.Navigator;
        var seen = new HashSet<string?>();
        for (int i = 0; i < count; i++)
        {
            if (i > 0)
            {
                Press(window, AppCommand.Down);
            }

            Pump(0.35); // a rolagem anima
            AssertInsideBand(window, $"{screen} (linha {i + 1})");
            seen.Add(Nav.GetKey(nav.Focused!));
        }

        Assert.True(seen.Count == count, $"{screen}: ↓ passou por {seen.Count} de {count} linhas");
    }

    /// <summary>O elemento focado está inteiro no palco, abaixo do topo e acima da barra de dicas.</summary>
    private static void AssertInsideBand(MainWindow window, string screen)
    {
        var focused = window.Navigator.Focused!;
        var stage = window.StageElement;
        stage.UpdateLayout();
        var bounds = focused.TransformToAncestor(stage).TransformBounds(new Rect(0, 0, focused.ActualWidth, focused.ActualHeight));
        Assert.True(bounds.Top >= 0, $"{screen}: {Nav.GetKey(focused)} cortado em cima ({bounds.Top:0})");
        Assert.True(bounds.Bottom <= HintBarTop - 8, $"{screen}: {Nav.GetKey(focused)} atrás da barra de dicas ({bounds.Bottom:0} > {HintBarTop - 8})");
        Assert.True(bounds.Right <= stage.ActualWidth, $"{screen}: {Nav.GetKey(focused)} cortado à direita ({bounds.Right:0} > {stage.ActualWidth:0})");
    }

    /// <summary>Cada dica visível está inteira dentro do palco (abreviar ou esconder, nunca cortar).</summary>
    private static void AssertHintBarFits(MainWindow window, string screen)
    {
        var bar = window.HintBarElement;
        var stage = window.StageElement;
        stage.UpdateLayout();
        int visible = 0;
        for (int i = 0; i < bar.Items.Count; i++)
        {
            if (bar.ItemContainerGenerator.ContainerFromIndex(i) is not FrameworkElement hint)
            {
                continue;
            }

            var bounds = hint.TransformToAncestor(stage).TransformBounds(new Rect(0, 0, hint.ActualWidth, hint.ActualHeight));
            if (bounds.Right < 0)
            {
                continue; // escondida pelo HintBarPanel (fora do painel, sem desenho)
            }

            visible++;
            Assert.True(bounds.Left >= 0 && bounds.Right <= stage.ActualWidth - 71, $"{screen}: dica {i} cortada ({bounds.Left:0}–{bounds.Right:0} de {stage.ActualWidth:0})");
        }

        Assert.True(visible >= Math.Min(2, bar.Items.Count), $"{screen}: só {visible} dicas visíveis");
        if (bar.Items.Count > 0)
        {
            Assert.IsType<HintBarPanel>(VisualTreeHelper.GetParent(bar.ItemContainerGenerator.ContainerFromIndex(0)));
        }
    }
}
