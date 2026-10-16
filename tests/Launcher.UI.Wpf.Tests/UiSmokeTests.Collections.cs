using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Session;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// UX-003/005/007 só com o controle: ☰ numa capa abre o menu rápido; uma coleção nasce pelo teclado virtual, ganha
    /// jogos pelos detalhes, vira fileira na Início e filtro na biblioteca; "Configurações deste jogo" muda o que a
    /// sessão usa só para aquele jogo. No fim, tudo volta como estava (o resto do teste usa a Início sem coleções).
    /// </summary>
    private static void CollectionsAndGameSettings(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        foreach (var leftover in shell.Collections.Collections.ToList())
        {
            shell.Collections.Delete(leftover.Id); // execução anterior na mesma máquina
        }

        shell.OpenLibrary(LibraryFilter.All);
        Pump(1);
        var library = Assert.IsType<LibraryViewModel>(shell.CurrentPage);
        var first = library.Games[0];
        var second = library.Games[1];
        Assert.Equal(first.Id, Nav.GetKey(nav.Focused!));
        Assert.Contains(shell.Hints, h => h.Label == "Ações" && h.Command == AppCommand.Menu);

        // UX-005: ☰ numa capa abre as ações rápidas do jogo.
        Press(window, AppCommand.Menu);
        Pump(0.6);
        var quick = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.Equal(first.Title, quick.Title);
        foreach (string label in (string[])["Adicionar à coleção", "Configurações deste jogo", "Detalhes"])
        {
            Assert.Contains(quick.Actions, a => a.Label == label);
        }

        AssertFocused(window, "Menu rápido");
        Snap(window, "80-menu-rapido");

        // Adicionar à coleção → Nova coleção: nome pelo teclado virtual; vazio não salva e explica.
        quick.Actions.Single(a => a.Label == "Adicionar à coleção").Command.Execute(null);
        Pump(0.6);
        var choose = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        choose.Actions.Single(a => a.Label == "Nova coleção").Command.Execute(null);
        Pump(0.6);
        var name = Assert.IsType<SearchViewModel>(shell.TopOverlay);
        Assert.Equal("Nova coleção", name.Title);
        Assert.Equal("keyboard", Nav.GetGroup(nav.Focused!));
        Press(window, AppCommand.Menu);
        Assert.Same(name, shell.TopOverlay);
        Assert.Equal("Dê um nome à coleção.", shell.ToastMessage);
        name.Type("Sofá com amigos");
        Pump(0.3);
        Snap(window, "81-nova-colecao");
        Press(window, AppCommand.Menu); // ☰ salva
        Pump(0.8);
        Assert.Null(shell.TopOverlay);
        var couch = Assert.Single(shell.Collections.Collections);
        Assert.Equal("Sofá com amigos", couch.Name);
        Assert.Contains(couch.Id, first.Game.Collections);
        Assert.Equal(first.Id, Nav.GetKey(nav.Focused!)); // o foco volta à capa depois da cadeia de diálogos
        AssertFocused(window, "Biblioteca depois de criar a coleção");

        // Detalhes › Coleções: marca o segundo jogo; o diálogo continua aberto mostrando o ✓.
        shell.OpenDetails(second);
        Pump(1);
        Assert.True(nav.FocusDefault("details:collections"));
        Press(window, AppCommand.Accept);
        Pump(0.6);
        var detailsChoose = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        detailsChoose.Actions.Single(a => a.Label == couch.Name).Command.Execute(null);
        Pump(0.6);
        var marked = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.Contains(marked.Actions, a => a.Label == $"{couch.Name} ✓");
        Snap(window, "82-detalhes-colecoes");
        marked.Actions.Single(a => a.Label == "Pronto").Command.Execute(null);
        Pump(0.6);
        Assert.Equal("Coleções (1)", ((DetailsViewModel)shell.CurrentPage).CollectionsLabel);
        AssertFocused(window, "Detalhes depois das coleções");

        // Início: a coleção vira uma fileira; "Ver todos" abre a biblioteca filtrada por ela.
        shell.GoHome();
        Pump(1.2);
        var home = Assert.IsType<HomeViewModel>(shell.CurrentPage);
        var row = Assert.Single(home.Collections);
        Assert.Equal(couch.Name, row.Title);
        Assert.Equal(2, row.Games.Count);
        Assert.True(nav.FocusDefault($"{row.NavGroup}:{first.Id}"));
        Press(window, AppCommand.Right);
        Assert.Equal(row.NavGroup, Nav.GetGroup(nav.Focused!));
        Pump(0.6);
        Snap(window, "83-inicio-colecao");
        row.SeeAllCommand.Execute(null);
        Pump(1);
        library = Assert.IsType<LibraryViewModel>(shell.CurrentPage);
        Assert.Equal(2, library.Games.Count);
        Assert.Equal("Filtros (1)", library.FiltersLabel);

        // Painel de filtros: a seção Coleção liga e desliga o mesmo filtro.
        shell.OpenLibraryFilters();
        Pump(0.8);
        var filters = Assert.IsType<LibraryFiltersViewModel>(shell.TopOverlay);
        var couchRow = Assert.Single(filters.Sections.Single(s => s.Title == "Coleção").Rows);
        Assert.True(couchRow.IsOn);
        couchRow.Command.Execute(null);
        Assert.Equal(shell.Library.Games.Count, library.Games.Count);
        couchRow.Command.Execute(null);
        Assert.Equal(2, library.Games.Count);
        Pump(0.4);
        Snap(window, "84-filtro-colecao");
        Press(window, AppCommand.Back);
        library.ClearCriteriaCommand.Execute(null);
        Pump(0.6);

        // UX-007: Configurações deste jogo — começa em "Padrão: ...", cada A troca e salva na hora.
        shell.OpenDetails(first);
        Pump(1);
        Assert.True(nav.FocusDefault("details:settings"));
        Press(window, AppCommand.Accept);
        Pump(0.8);
        var settings = Assert.IsType<GameSettingsViewModel>(shell.TopOverlay);
        Assert.True(settings.Current.IsDefault);
        Assert.Equal("game-settings", Nav.GetGroup(nav.Focused!));
        var rows = settings.Sections.SelectMany(s => s.Rows).ToDictionary(r => r.Key!);
        Assert.All(rows.Values.Where(r => r.Key is not ("gs:args" or "gs:reset")), r => Assert.StartsWith("Padrão", r.EndText, StringComparison.Ordinal));
        Press(window, AppCommand.Accept); // primeira linha: usar a Tela de jogo → Sim
        Assert.True(settings.Current.UseGameScreen);
        rows["gs:fps"].Command.Execute(null); // Sem limite
        rows["gs:fps"].Command.Execute(null); // 30 FPS
        rows["gs:show-fps"].Command.Execute(null);
        Assert.Equal(30, settings.Current.FpsLimit);
        Assert.Equal("30 FPS", rows["gs:fps"].EndText);
        Assert.True(settings.Current.ShowFps);

        // Argumentos extras pelo teclado virtual.
        rows["gs:args"].Command.Execute(null);
        Pump(0.6);
        var args = Assert.IsType<SearchViewModel>(shell.TopOverlay);
        Assert.Equal("Argumentos extras", args.Title);
        args.Type("-dx11");
        Press(window, AppCommand.Menu);
        Pump(0.6);
        Assert.Same(settings, shell.TopOverlay);
        Assert.Equal("-dx11", settings.Current.ExtraArguments);
        Assert.Equal("4 mudanças só para este jogo", settings.Summary);
        Snap(window, "85-configuracoes-do-jogo");
        // Linha focada com valor próprio do jogo (negrito) e a última linha: legíveis e acima da barra de dicas.
        foreach (string key in (string[])["gs:fps", "gs:reset"])
        {
            shell.RequestFocus(key);
            Pump(0.6);
            Assert.Equal(key, Nav.GetKey(nav.Focused!));
            Snap(window, $"85a-configuracoes-do-jogo-foco-{key[3..]}");
            AssertFocusedRowReadable(window, $"Configurações deste jogo › {key}", requireSolid: true);
            AssertAboveHintBar(window, $"Configurações deste jogo › {key}");
        }

        // O resolvedor (ponto único da sessão) vê as escolhas, gravadas no banco; o outro jogo segue o padrão.
        Assert.Equal(settings.Current, shell.LoadGameSettings(first.Id));
        var effective = shell.ResolveSessionSettings(first.Id);
        Assert.Equal(30, effective.FpsLimit);
        Assert.True(effective.FpsLimitFromGame);
        Assert.True(effective.ShowFps);
        Assert.Equal("-dx11", effective.Arguments(null));
        Assert.True(effective.GameScreen is not null || effective.GameScreenProblem == GameSettingsResolver.NoScreenChosen);
        Assert.False(shell.ResolveSessionSettings(second.Id).FpsLimitFromGame);

        // "Usar tudo do Tako" volta ao padrão (a linha some do banco).
        rows["gs:reset"].Command.Execute(null);
        Assert.True(settings.Current.IsDefault);
        Assert.True(shell.LoadGameSettings(first.Id).IsDefault);
        Press(window, AppCommand.Back);
        Pump(0.6);
        Assert.Null(shell.TopOverlay);
        AssertFocused(window, "Detalhes depois das configurações do jogo");

        // Configurações › Coleções: renomear e excluir pelo controle.
        shell.ManageCollections();
        Pump(0.6);
        var manage = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        manage.Actions.First(a => a.Label.StartsWith(couch.Name, StringComparison.Ordinal)).Command.Execute(null);
        Pump(0.6);
        Assert.IsType<DialogViewModel>(shell.TopOverlay).Actions.Single(a => a.Label == "Renomear").Command.Execute(null);
        Pump(0.6);
        var rename = Assert.IsType<SearchViewModel>(shell.TopOverlay);
        Assert.Equal(couch.Name, rename.Text);
        rename.Type(" 2");
        rename.Finish();
        Pump(0.6);
        Assert.Equal("Sofá com amigos 2", Assert.Single(shell.Collections.Collections).Name);
        shell.ManageCollections();
        Pump(0.6);
        Assert.IsType<DialogViewModel>(shell.TopOverlay).Actions.First(a => a.Label.StartsWith("Sofá com amigos 2", StringComparison.Ordinal)).Command.Execute(null);
        Pump(0.4);
        Assert.IsType<DialogViewModel>(shell.TopOverlay).Actions.Single(a => a.Label == "Excluir").Command.Execute(null);
        Pump(0.4);
        Snap(window, "86-excluir-colecao");
        Assert.IsType<DialogViewModel>(shell.TopOverlay).Actions.Single(a => a.Label == "Excluir").Command.Execute(null);
        Pump(0.6);
        Assert.Null(shell.TopOverlay);
        Assert.Empty(shell.Collections.Collections);
        Assert.Empty(first.Game.Collections);
        AssertFocused(window, "depois de excluir a coleção");
        shell.LibraryCriteria = LibraryCriteria.None;
    }
}
