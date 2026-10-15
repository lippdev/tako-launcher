using System.Diagnostics;
using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Ipc;
using ConsoleMode.Runtime.Simulation;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Resilience;
using Launcher.Core.Session.Restore;
using Launcher.Core.Supervision;
using Launcher.PlayniteCompat.P10.Controls;
using Launcher.Tests;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

/// <summary>
/// Teste de fumaça da UI real (Windows): monta o app, passa por todas as telas com comandos de controle,
/// salva capturas em <c>screenshots/</c> (publicadas pelo CI) e falha com erros de binding do WPF.
/// </summary>
public sealed partial class UiSmokeTests
{
    private static readonly string Screenshots = Path.Combine(AppContext.BaseDirectory, "screenshots");

    [Fact]
    public void Launcher_runs_every_screen_with_gamepad_navigation() => RunOnSta(Journey);

    private static void Journey()
    {
        Directory.CreateDirectory(Screenshots);
        var bindingErrors = new BindingErrorListener();
        PresentationTraceSources.Refresh();
        PresentationTraceSources.DataBindingSource.Listeners.Add(bindingErrors);
        PresentationTraceSources.DataBindingSource.Switch.Level = SourceLevels.Error;

        var app = new App();
        app.InitializeComponent();
        // Biblioteca fictícia: o CI não tem Steam e o teste não depende da rede para os dados. Monitores e áudio
        // simulados: o teste nunca muda a resolução nem o som da máquina (MVP-012).
        var display = new SimulatedDisplayConfiguration();
        var audio = new SimulatedAudioEndpoints();
        string environmentFile = Path.Combine(Path.GetTempPath(), $"tako-smoke-environment-{Guid.NewGuid():N}.json");
        using var services = App.BuildServices(
            new InMemoryGameLibrary(SampleLibrary.Load(TimeProvider.System), TimeProvider.System),
            s => s.AddSingleton<IDisplayConfigurationService>(display)
                .AddSingleton<IAudioEndpointService>(audio)
                .AddSingleton<IDisplayControlService>(new SimulatedDisplayDesk())
                .AddSingleton<IScreenCurtain>(new SimulatedScreenCurtain())
                .AddSingleton(new PendingEnvironmentStore(environmentFile)));
        var theme = services.GetRequiredService<ThemeService>().Apply(ThemeService.DefaultTheme);
        Assert.True(theme.Succeeded, theme.Error);

        var shell = services.GetRequiredService<ShellViewModel>();
        var window = services.GetRequiredService<MainWindow>();
        window.WindowState = WindowState.Normal;
        window.Width = 1280;
        window.Height = 720;
        window.Show();
        var nav = window.Navigator;
        Pump(4); // artes baixam em paralelo
        Snap(window, "01-inicio");

        // Início: foco abre em "Jogar"; ↓ desce para a fileira; → troca o destaque.
        Assert.Equal("home:play", Nav.GetKey(nav.Focused!));
        var recent = ((HomeViewModel)shell.CurrentPage).RecentlyPlayed;
        Press(window, AppCommand.Down);
        Assert.Equal("row-recent", Nav.GetGroup(nav.Focused!));
        Assert.Equal(recent[0].Id, Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Right);
        Assert.Equal(recent[1].Id, ((HomeViewModel)shell.CurrentPage).Hero!.Id);
        Pump(1.2);
        Snap(window, "02-inicio-fileira");

        // ← da primeira capa vai para a barra lateral (expandida); → volta para a mesma capa.
        Press(window, AppCommand.Left);
        Press(window, AppCommand.Left);
        Assert.Equal("sidebar", Nav.GetGroup(nav.Focused!));
        Assert.Equal("home", Nav.GetKey(nav.Focused!)); // entra no item selecionado, não no mais próximo
        Assert.True(shell.IsSidebarExpanded);
        Pump(0.6);
        Snap(window, "03-barra-lateral");
        Press(window, AppCommand.Right);
        Assert.Equal("row-recent", Nav.GetGroup(nav.Focused!));

        // Biblioteca Xbox: o fundo acompanha a capa em foco.
        shell.OpenLibrary(LibraryFilter.ForStore(GameStore.Xbox));
        Pump(1.5);
        Assert.Equal("grid", Nav.GetGroup(nav.Focused!));
        Press(window, AppCommand.Right);
        string focusedGame = Nav.GetKey(nav.Focused!)!;
        Assert.Equal(focusedGame, shell.Backdrop!.GameId);
        Pump(1);
        Snap(window, "04-biblioteca-xbox");

        // Detalhes e volta (B) com o foco restaurado.
        Press(window, AppCommand.Accept);
        Pump(1.2);
        Assert.IsType<DetailsViewModel>(shell.CurrentPage);
        Assert.Equal("details:primary", Nav.GetKey(nav.Focused!));
        Snap(window, "05-detalhes");
        Press(window, AppCommand.Back);
        Assert.IsType<LibraryViewModel>(shell.CurrentPage);
        Assert.Equal(focusedGame, Nav.GetKey(nav.Focused!));

        // Configurações e recuperação de tema inválido (MVP-016/021).
        shell.OpenSettings();
        Pump(0.8);
        Snap(window, "06-configuracoes");
        // Tema Fullscreen padrão do Playnite 10 renderizado sem o Playnite (H2).
        shell.ShowPlaynitePreview();
        Pump(2.5);
        var preview = Assert.IsType<PlaynitePreviewViewModel>(shell.TopOverlay);
        File.WriteAllLines(Path.Combine(Screenshots, "playnite-theme-report.txt"),
            [preview.Summary, .. preview.Report.Loaded.Select(f => "OK    " + f), .. preview.RejectedLines.Select(r => "RECUSA " + r)]);
        Assert.Contains("Constants.xaml", preview.Report.Loaded);
        Assert.Contains("DerivedStyles/ListGameItemStyle.xaml", preview.Report.Loaded);
        Assert.Contains("DefaultControls/ComboBox.xaml", preview.Report.Loaded);
        // Com os adaptadores de dados (MVP-018), Main.xaml ({Api}) e GameDetails.xaml (HtmlTextView seguro) também carregam.
        Assert.Empty(preview.Report.Rejected);
        Assert.True(preview.Report.MainFromTheme);
        Assert.Contains("ListGameItemStyle", preview.Resources.MergedDictionaries.SelectMany(d => d.Keys.OfType<string>()));
        Assert.Equal("p10-games", Nav.GetGroup(nav.Focused!));
        Press(window, AppCommand.Right);
        Assert.Equal("p10-games:1", Nav.GetKey(nav.Focused!));
        Pump(0.5);
        Snap(window, "14-tema-playnite");
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);

        ((SettingsViewModel)shell.CurrentPage).SimulateThemeFailureCommand.Execute(null);
        Pump(0.8);
        Assert.IsType<RecoveryViewModel>(shell.TopOverlay);
        Snap(window, "07-recuperacao");
        Press(window, AppCommand.Accept); // "Usar tema padrão"
        Assert.Null(shell.TopOverlay);

        // Jogar → Iniciando → jogo → Central de Controle (Guia).
        shell.GoHome();
        Pump(1);
        Press(window, AppCommand.Accept);
        Assert.IsType<LaunchOverlayViewModel>(shell.TopOverlay);
        Pump(1);
        Snap(window, "08-iniciando");
        Pump(2.5);
        Assert.True(shell.IsInGame);
        Pump(1);
        Snap(window, "09-jogo");
        Press(window, AppCommand.Guide);
        Pump(1);
        Assert.IsType<ControlCenterViewModel>(shell.TopOverlay);
        Assert.Equal("card:game", Nav.GetKey(nav.Focused!));
        Snap(window, "10-central-de-controle");

        // ↑ vai ao ícone ligado ao cartão; → até Áudio; A abre o painel; → no volume ajusta; ↓ e → trocam de painel.
        Press(window, AppCommand.Up);
        Assert.Equal("bar:game", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Right);
        Press(window, AppCommand.Right);
        Press(window, AppCommand.Right);
        Assert.Equal("bar:audio", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        var center = (ControlCenterViewModel)shell.TopOverlay!;
        Assert.Equal("audio", center.Popover!.Key);
        int volume = shell.SessionOptions.Volume;
        Press(window, AppCommand.Right);
        Assert.Equal(volume + 5, shell.SessionOptions.Volume);
        Pump(0.5);
        Snap(window, "11-painel-audio");
        Press(window, AppCommand.Down);
        Press(window, AppCommand.Right);
        Assert.Equal("controller", center.Popover!.Key);
        Press(window, AppCommand.Back);
        Assert.Null(center.Popover);
        Assert.Equal("bar:controller", Nav.GetKey(nav.Focused!));

        // Encerrar pelo painel: volta à Início com o jogo no topo.
        string played = shell.RunningGame!.Id;
        Press(window, AppCommand.Right);
        Press(window, AppCommand.Right);
        Assert.Equal("bar:quit", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        Snap(window, "12-encerrar");
        Press(window, AppCommand.Accept);
        Pump(1);
        Assert.False(shell.IsInGame);
        Assert.Equal(played, ((HomeViewModel)shell.CurrentPage).RecentlyPlayed[0].Id);

        // Sair para o desktop pede confirmação; B cancela.
        shell.ConfirmExit();
        Pump(0.6);
        Snap(window, "13-sair");
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);

        // Outras proporções: o palco acompanha a largura da janela (sem faixas pretas) e a grade ganha colunas.
        // Jogo adicionado pelo usuário: aparece na biblioteca e tem opções próprias (corrigir dados, capa, remover).
        var manual = shell.Library.AddManual(ManualGameRecord.Create("Meu Jogo Indie", @"C:\Jogos\MeuJogo\jogo.exe", DateTimeOffset.Now));
        Pump(0.5);
        shell.OpenDetails(shell.ItemFor(manual));
        Pump(0.8);
        shell.ShowGameOptions(shell.ItemFor(manual));
        Pump(0.6);
        Snap(window, "19-opcoes-jogo-manual");
        Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);

        // --- Sessão (frente B)
        // Editar jogo adicionado (MVP-024): campos de texto, ↓ entre campos, botão de estratégia, validação da pasta.
        shell.EditGame(shell.ItemFor(manual));
        Pump(0.6);
        var editor = Assert.IsType<EditGameViewModel>(shell.TopOverlay);
        Assert.Equal("edit:name", Nav.GetKey(nav.Focused!));
        editor.Name = "Meu Jogo Indie Editado";
        editor.Arguments = "-windowed";
        Press(window, AppCommand.Down);
        Press(window, AppCommand.Down);
        Press(window, AppCommand.Down);
        Assert.Equal("edit:tracking", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        Assert.Equal("Fim do jogo: Processo iniciado", editor.TrackingLabel);
        Press(window, AppCommand.Down);
        Assert.Equal("edit:save", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept); // a pasta C:\Jogos\MeuJogo não existe no CI
        Assert.True(editor.HasError);
        Pump(0.4);
        Snap(window, "20-editar-jogo");
        editor.WorkingDirectory = string.Empty; // vazio = pasta do executável
        Press(window, AppCommand.Accept);
        Assert.Null(shell.TopOverlay);
        var edited = shell.Library.FindManual(manual.Id)!;
        Assert.Equal("Meu Jogo Indie Editado", edited.Title);
        Assert.Equal("-windowed", edited.Arguments);
        Assert.Equal(Launcher.Core.Session.GameTrackingStrategy.Process, edited.TrackBy);
        Assert.Equal("Meu Jogo Indie Editado", Assert.IsType<DetailsViewModel>(shell.CurrentPage).Game.Title);

        // Jogo real (MVP-026/027): acompanha o processo iniciado até ele fechar, registra o tempo e devolve o foco.
        string ping = Path.Combine(Environment.SystemDirectory, "PING.EXE");
        var real = shell.Library.AddManual(ManualGameRecord.Create("Ping", ping, DateTimeOffset.Now) with { Arguments = "-n 4 127.0.0.1" });
        Pump(0.5);
        shell.OpenDetails(shell.ItemFor(real));
        Pump(0.8);
        Press(window, AppCommand.Accept); // Jogar
        Assert.IsType<LaunchOverlayViewModel>(shell.TopOverlay);
        Pump(3);
        Assert.Equal(ConsoleMode.Runtime.SessionState.Playing, shell.Session);
        Assert.True(shell.IsTrackingRealGame);
        Assert.Equal(real.Id, shell.RunningGame!.Id);
        Assert.Equal("Voltar ao jogo", ((DetailsViewModel)shell.CurrentPage).PrimaryLabel);
        Snap(window, "21-jogo-real");
        for (int i = 0; i < 30 && shell.Session != ConsoleMode.Runtime.SessionState.Browsing; i++)
        {
            Pump(1);
        }

        Assert.Equal(ConsoleMode.Runtime.SessionState.Browsing, shell.Session);
        Assert.Null(shell.RunningGame);
        Assert.False(shell.IsTrackingRealGame);
        Assert.StartsWith("Bem-vindo de volta", shell.ToastMessage, StringComparison.Ordinal);
        Assert.NotNull(shell.Library.Find(real.Id)!.LastPlayedAt);
        Pump(0.5);
        Assert.Equal("details:primary", Nav.GetKey(nav.Focused!));
        Assert.Equal("Jogar", ((DetailsViewModel)shell.CurrentPage).PrimaryLabel);
        Snap(window, "22-retorno-do-jogo");

        // --- Resiliência e foco (frente D)
        ResilienceAndFocus(app, services, shell, window);

        // --- Temas (frente C): Configurações › Temas, importar .pthm, prévia, diagnóstico, remover (MVP-014/015/020)
        // e homologação de três temas reais (MVP-022), tudo pelo controle onde há tela.
        var playnite = services.GetRequiredService<PlayniteThemeService>();
        shell.OpenSettings();
        Pump(0.5);
        shell.OpenThemes();
        Pump(0.8);
        Assert.IsType<ThemesViewModel>(shell.CurrentPage);
        Assert.Equal("themes:import", Nav.GetKey(nav.Focused!));
        Snap(window, "30-temas");
        Press(window, AppCommand.Down);
        Press(window, AppCommand.Down);
        Press(window, AppCommand.Down);
        Assert.Equal("theme:Playnite_builtin_DefaultFullscreen", Nav.GetKey(nav.Focused!));
        Press(window, AppCommand.Accept);
        Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Pump(0.4);
        Snap(window, "31-tema-opcoes");
        Press(window, AppCommand.Back);

        // Pacote recusado (tema do modo Desktop): diálogo com o motivo e nada instalado.
        Assert.False(shell.ImportThemeFrom(ThemeFixtures.DesktopPackage()).Succeeded);
        Pump(0.4);
        Assert.Equal("overlay", Nav.GetGroup(nav.Focused!));
        Snap(window, "32-tema-recusado");
        Press(window, AppCommand.Back);
        Assert.DoesNotContain(playnite.Imported(), t => t.Id == "Tako_Teste_Desktop");

        // Pacote válido: importa, A abre a prévia (ação padrão) e o diagnóstico mostra X de Y arquivos.
        var copy = shell.ImportThemeFrom(ThemeFixtures.CopyOfDefault(playnite.BuiltIn().Directory));
        Assert.True(copy.Succeeded, string.Join('\n', copy.Diagnostics));
        Pump(0.4); // o foco vai para a ação padrão do diálogo (A na página abriria o seletor de arquivo, que é modal)
        Assert.Equal("overlay", Nav.GetGroup(nav.Focused!));
        Press(window, AppCommand.Accept);
        Pump(2);
        var copyPreview = Assert.IsType<PlaynitePreviewViewModel>(shell.TopOverlay);
        Assert.Equal(33, copyPreview.Report.Loaded.Count);
        Press(window, AppCommand.Back);
        var copyItem = ((ThemesViewModel)shell.CurrentPage).Themes.Single(t => t.Theme?.Id == "Tako_Teste_CopiaDoPadrao");
        shell.ShowThemeDiagnostics(copyItem);
        Pump(0.4);
        Assert.StartsWith("33 de 33", copyItem.LoadSummary, StringComparison.Ordinal);
        Snap(window, "33-tema-diagnostico");
        Press(window, AppCommand.Back);

        // Homologação: .pthm das releases (hash fixo), importados e abertos na prévia; o relatório vai para o artefato.
        // Bindings para dados que o adaptador ainda não fornece (MVP-018) são esperados em temas de terceiros: vão para o
        // relatório da homologação, não para a verificação de erros de binding do launcher.
        var homologation = new List<string>();
        var themeExceptions = new List<string>();
        window.Dispatcher.UnhandledException += (_, e) => themeExceptions.Add(e.Exception.ToString());
        foreach (var fixture in ThemeFixtures.Homologated)
        {
            int bindingErrorsBefore = bindingErrors.Errors.Count;
            string? package = ThemeFixtures.TryDownload(fixture, out string? problem);
            if (package is null)
            {
                homologation.Add($"{fixture.Name}: não executado ({problem})");
                continue;
            }

            var installed = shell.ImportThemeFrom(package);
            Assert.True(installed.Succeeded, string.Join('\n', installed.Diagnostics));
            Pump(0.4);
            Assert.Equal("overlay", Nav.GetGroup(nav.Focused!));
            Press(window, AppCommand.Accept);
            Pump(3);
            if (shell.TopOverlay is PlaynitePreviewViewModel themePreview)
            {
                var r = themePreview.Report;
                homologation.Add($"{fixture.Name} [{fixture.License}] · ThemeApiVersion {r.ThemeApiVersion} · {r.Loaded.Count} de {r.Total} carregados · {r.Inherited.Count} herdados · {r.Ignored.Count} ignorados");
                homologation.AddRange(r.Manifest.Diagnostics.Select(d => "    " + d));
                homologation.AddRange(r.Loaded.Select(f => "    OK      " + f));
                homologation.AddRange(r.Rejected.Select(x => $"    RECUSA  {x.File} — {x.Reason}"));
                homologation.AddRange(r.Inherited.Select(f => "    HERDADO " + f));
                homologation.AddRange(r.Ignored.Select(f => "    IGNORADO " + f));
                homologation.Add($"    Main.xaml: {(r.MainFromTheme ? "do tema" : r.HasMainView ? "herdado do tema padrão" : "AUSENTE")}");
                Assert.NotEmpty(r.Loaded);
                // MVP-018: a tela principal carrega (do tema, ou do padrão quando o tema usa algo bloqueado por segurança).
                Assert.True(r.HasMainView, $"{fixture.Name}: Views/Main.xaml ausente");
                Assert.True(fixture.MainLoads == r.MainFromTheme, $"{fixture.Name}: Main.xaml do tema esperado={fixture.MainLoads}; {string.Join(" | ", r.Rejected.Where(x => x.File == "Views/Main.xaml").Select(x => x.Reason))}");
                // Frente H: as regras novas (validação estática e guarda de mídia) não mudam o que os temas homologados carregam.
                Assert.True(fixture.Loaded == r.Loaded.Count && fixture.Total == r.Total, $"{fixture.Name}: esperado {fixture.Loaded} de {fixture.Total}, carregou {r.Loaded.Count} de {r.Total}; {string.Join(" | ", r.Rejected.Select(x => $"{x.File}: {x.Reason}"))}");
                Snap(window, $"34-tema-{fixture.Slug}");
            }
            else
            {
                homologation.Add($"{fixture.Name}: prévia não abriu ({shell.TopOverlay?.GetType().Name ?? "nada"})");
                homologation.AddRange(themeExceptions.SelectMany(x => x.Split('\n').Take(25)).Select(l => "      " + l.TrimEnd()));
                Snap(window, $"34-tema-{fixture.Slug}-falha");
            }

            Press(window, AppCommand.Back);
            while (shell.TopOverlay is not null)
            {
                shell.CloseOverlay(shell.TopOverlay);
            }

            var themeBindingErrors = bindingErrors.Errors.Skip(bindingErrorsBefore).ToList();
            bindingErrors.Errors.RemoveRange(bindingErrorsBefore, themeBindingErrors.Count);
            homologation.Add($"    Erros de binding do tema (dados do Playnite ainda não fornecidos): {themeBindingErrors.Count}");
            homologation.AddRange(themeBindingErrors.Select(e => e.Split(';')[0]).Distinct(StringComparer.Ordinal).Take(15).Select(e => "      " + e));
        }

        File.WriteAllLines(Path.Combine(Screenshots, "theme-homologation.txt"), homologation);

        // Modo seguro (frente D): temas importados não são carregados.
        var page = (ThemesViewModel)shell.CurrentPage;
        page.Refresh();
        shell.IsSafeMode = true;
        shell.ShowThemePreview(page.Themes.Single(t => t.Theme?.Id == "Tako_Teste_CopiaDoPadrao").Theme!);
        Assert.Null(shell.TopOverlay);
        shell.IsSafeMode = false;

        // Usar um tema importado muda a prévia; remover e restaurar voltam ao padrão.
        shell.ShowThemeOptions(page.Themes.Single(t => t.Theme?.Id == "Tako_Teste_CopiaDoPadrao"));
        var options = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        options.Actions.Single(a => a.Label == "Usar este tema").Command.Execute(null);
        Assert.Equal("Tako_Teste_CopiaDoPadrao", playnite.SelectedId);
        Pump(0.4);
        Snap(window, "35-temas-importados");
        foreach (var item in page.Themes.Where(t => t.CanRemove).ToList())
        {
            shell.RemoveTheme(item);
        }

        shell.RestoreDefaultTheme();
        Pump(0.5);
        Assert.Null(playnite.SelectedId);
        Assert.DoesNotContain(((ThemesViewModel)shell.CurrentPage).Themes, t => t.Kind == ThemeKind.Imported);
        Assert.Equal("themes:import", Nav.GetKey(nav.Focused!));

        // --- Monitores e áudio (frente E2)
        MonitorsAndAudio(services, shell, window, real, display, audio);
        GameScreenSettingsAndConfirmation(shell, window);

        // --- Supervisor (frente E1): a UI aberta com --supervisor-pipe manda heartbeats pela thread da UI (MVP-003/006).
        string supervisorPipe = NamedPipeIpc.NewPipeName("tk-smoke");
        int heartbeats = 0;
        var supervisorServer = new IpcServer(new NamedPipeServerTransport(supervisorPipe), new IpcConnectionOptions
        {
            PeerName = SupervisorMessages.SupervisorPeer,
            Handler = new IpcHandlerMap()
                .OnEvent(SupervisorMessages.Heartbeat, _ => Interlocked.Increment(ref heartbeats))
                .OnRequest(SupervisorMessages.Status, _ => new SupervisorStatus(Environment.ProcessId, 0, UiLaunchMode.Normal)),
        });
        supervisorServer.Start();
        var link = services.GetRequiredService<SupervisorLink>();
        Assert.False(link.IsSupervised);
        link.Start([SupervisorArguments.Ui, SupervisorArguments.Pipe, supervisorPipe], app, TimeSpan.FromMilliseconds(200));
        Pump(2);
        Assert.True(link.IsSupervised);
        Assert.True(Volatile.Read(ref heartbeats) > 0, "nenhum heartbeat chegou ao supervisor");
        Assert.Equal(Environment.ProcessId, link.SupervisorProcessId);
        Assert.Equal(UiLaunchMode.Normal, link.Status?.Mode);
        supervisorServer.DisposeAsync().AsTask().GetAwaiter().GetResult();
        // --- Tema Playnite na interface (frente F): "Usar este tema" aplica o Main.xaml do tema à tela inicial (MVP-018/019/021).
        PlayniteThemeOnTheWholeUi(services, shell, window, bindingErrors);

        // --- Diagnóstico (frente G): Configurações › Diagnóstico do sistema gera o relatório do checklist (MVP-034).
        shell.OpenSettings();
        Pump(0.8);
        Assert.True(nav.FocusDefault("settings:diagnostics"));
        Press(window, AppCommand.Accept);
        Pump(0.6);
        var diagnostics = Assert.IsType<DiagnosticsViewModel>(shell.TopOverlay);
        Assert.Equal("Copiar", Nav.GetKey(nav.Focused!));
        Assert.Contains("[Monitores]", diagnostics.Report, StringComparison.Ordinal);
        Assert.Contains("[Xbox mode / FSE]", diagnostics.Report, StringComparison.Ordinal);
        Assert.Contains($"Total:                 {shell.Library.Games.Count}", diagnostics.Report, StringComparison.Ordinal);
        Assert.DoesNotContain(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), diagnostics.Report, StringComparison.OrdinalIgnoreCase);
        Assert.StartsWith("Salvo em logs", diagnostics.Status, StringComparison.Ordinal);
        File.WriteAllText(Path.Combine(Screenshots, "diagnostico-sistema.txt"), diagnostics.Report);
        Snap(window, "50-diagnostico");
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);

        // --- Segurança de temas (frente H): caminhos montados por dados não carregam (MVP-016, questão aberta 8).
        ThemeSecurity(services, bindingErrors);

        // Janelas pequenas: a tela virtual do CI limita o tamanho máximo da janela.
        shell.GoHome();
        window.Width = 840; // 21:9
        window.Height = 360;
        Pump(1.5);
        Assert.Equal(2520, window.StageElement.ActualWidth, 0);
        Snap(window, "15-inicio-21x9");
        shell.OpenLibrary(LibraryFilter.All);
        Pump(1.5);
        Snap(window, "16-biblioteca-21x9");
        window.Width = 576; // 16:10
        shell.GoHome();
        Pump(1.5);
        Assert.Equal(1728, window.StageElement.ActualWidth, 0);
        Snap(window, "17-inicio-16x10");

        // 4K: texto e vetores renderizados em 3840x2160.
        window.Width = 640; // 16:9
        Pump(1.5);
        Snap(window, "18-inicio-4k", scale: 2);

        File.WriteAllLines(Path.Combine(Screenshots, "binding-errors.txt"), bindingErrors.Errors);
        window.Close();
        Assert.DoesNotContain(bindingErrors.Errors, e => !e.Contains("Error: 4 :", StringComparison.Ordinal));
    }

    /// <summary>
    /// MVP-029/031/011/036: toda tela essencial abre com um elemento focado e B sempre leva a algum lugar; erro
    /// inesperado mostra a recuperação; o modo seguro avisa e volta ao normal sem reiniciar.
    /// </summary>
    private static void ResilienceAndFocus(App app, ServiceProvider services, ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;

        // Início é a raiz: B não tira o foco de lugar nenhum.
        shell.GoHome();
        Pump(1);
        AssertFocused(window, "Início");
        Press(window, AppCommand.Back);
        Assert.IsType<HomeViewModel>(shell.CurrentPage);
        AssertFocused(window, "Início depois de B");

        // Biblioteca → Detalhes → B → Biblioteca → B → Início.
        shell.OpenLibrary(LibraryFilter.All);
        Pump(1);
        AssertFocused(window, "Biblioteca");
        Press(window, AppCommand.Accept);
        Pump(0.8);
        Assert.IsType<DetailsViewModel>(shell.CurrentPage);
        AssertFocused(window, "Detalhes");
        Press(window, AppCommand.Back);
        Assert.IsType<LibraryViewModel>(shell.CurrentPage);
        AssertFocused(window, "Biblioteca depois de B");
        Press(window, AppCommand.Back);
        Assert.IsType<HomeViewModel>(shell.CurrentPage);
        AssertFocused(window, "Início depois de sair da biblioteca");

        // Favoritos (pode estar vazia: o cabeçalho ainda tem o que focar).
        shell.OpenLibrary(LibraryFilter.Favorites);
        Pump(0.8);
        AssertFocused(window, "Favoritos");

        // Configurações: as linhas novas (modo seguro e Xbox mode) ficam logo antes de "Tema".
        shell.OpenSettings();
        Pump(0.8);
        AssertFocused(window, "Configurações");
        Assert.NotNull(nav.Navigables().FirstOrDefault(e => Nav.GetKey(e) == "settings:safemode"));
        Assert.NotNull(nav.Navigables().FirstOrDefault(e => Nav.GetKey(e) == "settings:fse"));
        Assert.NotEmpty(shell.FseStatus.Reason);
        Press(window, AppCommand.Back);
        Assert.IsNotType<SettingsViewModel>(shell.CurrentPage);
        AssertFocused(window, "depois de sair das Configurações");

        // Diálogo: foco nas ações; B fecha e devolve o foco à página.
        shell.ConfirmExit();
        Pump(0.5);
        AssertFocused(window, "Diálogo de saída");
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);
        AssertFocused(window, "depois do diálogo");

        // MVP-011: exceção não tratada na UI vira tela de recuperação com caminho para o desktop.
        var guard = services.GetRequiredService<CrashGuard>();
        guard.Install(app, window, shell);
        var seen = new List<string>();
        window.Dispatcher.UnhandledException += (_, e) => seen.Add($"{e.Exception.GetType().Name} handled={e.Handled}");
        window.Dispatcher.BeginInvoke(() => throw new InvalidOperationException("Falha simulada pelo teste de fumaça"));
        Pump(0.8);
        string diagnosis = $"eventos=[{string.Join("; ", seen)}] guarda={guard.LastOutcome} fatal={guard.IsFatal} carregada={window.IsLoaded} visível={window.IsVisible} topo={shell.TopOverlay?.GetType().Name ?? "nenhum"}";
        Assert.False(guard.IsFatal, diagnosis);
        Assert.Equal(1, guard.Recoveries);
        Assert.True(shell.TopOverlay is RecoveryViewModel, diagnosis);
        var error = Assert.IsType<RecoveryViewModel>(shell.TopOverlay);
        Assert.True(error.IsError);
        Assert.Contains(error.Actions, a => a.Label == "Sair para o desktop");
        Assert.False(services.GetRequiredService<CrashGuard>().IsFatal);
        AssertFocused(window, "Recuperação de erro");
        Snap(window, "23-recuperacao-erro");
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);
        AssertFocused(window, "depois da recuperação");

        // MVP-036: modo seguro com aviso; "Voltar ao normal" aplica o tema do disco sem reiniciar.
        string journalFile = Path.Combine(Path.GetTempPath(), $"tako-smoke-{Guid.NewGuid():N}.json");
        var journal = new SessionJournal(journalFile, TimeProvider.System);
        var decision = CrashLoopPolicy.Decide(new SessionMarker(true, 1, null), safeModeRequested: false, DateTimeOffset.Now);
        Assert.True(decision.SafeMode);
        shell.EnterSafeMode(decision, journal, services.GetRequiredService<UpdateService>());
        Pump(0.6);
        var notice = Assert.IsType<RecoveryViewModel>(shell.TopOverlay);
        Assert.Equal("Modo seguro", notice.Title);
        Assert.Equal("Continuar no modo seguro", Nav.GetKey(nav.Focused!));
        Snap(window, "24-modo-seguro");
        Press(window, AppCommand.Accept);
        Assert.Null(shell.TopOverlay);
        Assert.True(shell.IsSafeMode);
        shell.OpenSettings();
        Pump(0.8);
        Assert.True(nav.FocusDefault("settings:safemode"));
        Assert.Equal("settings:safemode", Nav.GetKey(nav.Focused!));
        Pump(0.6);
        Snap(window, "25-configuracoes-resiliencia");
        Press(window, AppCommand.Accept);
        Assert.False(shell.IsSafeMode);
        Assert.Equal(ThemeService.DefaultTheme, services.GetRequiredService<ThemeService>().CurrentTheme);
        File.Delete(journalFile);

        // B durante "Iniciando" cancela de verdade (antes só escondia a tela e o jogo abria mesmo assim).
        // Jogo simulado da biblioteca fictícia (sem executável): o destaque da Início pode ser um jogo real.
        var simulated = shell.ItemFor(shell.Library.Games.First(g => g.LaunchTarget is null && g.CanPlay));
        shell.GoHome();
        Pump(0.8);
        shell.Launch(simulated);
        Pump(0.3);
        Assert.IsType<LaunchOverlayViewModel>(shell.TopOverlay);
        AssertFocused(window, "Iniciando");
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);
        Pump(3);
        Assert.False(shell.IsInGame);
        AssertFocused(window, "depois de cancelar o início");

        // Central de Controle: foco no cartão do jogo; B volta ao jogo.
        shell.Launch(simulated);
        Pump(3.2);
        Assert.True(shell.IsInGame);
        Press(window, AppCommand.Guide);
        Pump(0.6);
        Assert.IsType<ControlCenterViewModel>(shell.TopOverlay);
        AssertFocused(window, "Central de Controle");
        Press(window, AppCommand.Back);
        Assert.IsType<GameSessionViewModel>(shell.TopOverlay);
        shell.EndSession();
        Pump(0.8);
        Assert.Null(shell.TopOverlay);
        AssertFocused(window, "Início depois do jogo");
    }

    /// <summary>
    /// Tela de jogo (MVP-002) com a mesa simulada: desligada por padrão, ligar sugere a TV, as linhas extras aparecem e a
    /// confirmação "Você consegue ver esta tela?" responde "não" com B/Esc. Termina desligada para não afetar o resto.
    /// </summary>
    private static void GameScreenSettingsAndConfirmation(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        shell.OpenSettings();
        Pump(0.8);
        Assert.False(shell.Settings.GameScreenEnabled);
        Assert.True(nav.FocusDefault("settings:gamescreen"));
        Press(window, AppCommand.Accept);
        Pump(0.4);
        Assert.True(shell.Settings.GameScreenEnabled);
        Assert.Equal("SIM\\TV", shell.Settings.GameScreenId);
        Assert.Equal("TV da sala", ((SettingsViewModel)shell.CurrentPage).GameScreenLabel);
        Snap(window, "43-configuracoes-tela-de-jogo");

        var confirm = new ScreenConfirmViewModel("TV da sala", ShellViewModel.ScreenConfirmSeconds);
        shell.PushOverlay(confirm);
        Pump(0.5);
        Assert.Equal("overlay", Nav.GetGroup(nav.Focused!));
        Snap(window, "44-confirmar-tela-de-jogo");
        Press(window, AppCommand.Back);
        Assert.True(confirm.Result.IsCompleted);
        Assert.False(confirm.Result.Result);
        shell.CloseOverlay(confirm);

        shell.Settings.GameScreenEnabled = false;
        shell.Settings.GameScreenId = string.Empty;
        shell.Settings.GameScreenName = string.Empty;
        Pump(0.3);
    }

    /// <summary>
    /// MVP-012 com monitores e áudio simulados: opção nas Configurações, restauração só do que o jogo real mudou e
    /// oferta de restauração depois de uma queda do Tako durante o jogo.
    /// </summary>
    private static void MonitorsAndAudio(ServiceProvider services, ShellViewModel shell, MainWindow window, Game game, SimulatedDisplayConfiguration display, SimulatedAudioEndpoints audio)
    {
        var nav = window.Navigator;

        // Configurações: a linha nova fica antes de "Tema"; A liga e desliga (padrão: ligada).
        shell.OpenSettings();
        Pump(0.8);
        Assert.True(shell.Settings.RestoreEnvironmentAfterGame);
        Assert.True(nav.FocusDefault("settings:environment"));
        Press(window, AppCommand.Accept);
        Assert.False(shell.Settings.RestoreEnvironmentAfterGame);
        Press(window, AppCommand.Accept);
        Assert.True(shell.Settings.RestoreEnvironmentAfterGame);
        Pump(0.4);
        Snap(window, "40-configuracoes-monitores-audio");

        // Jogo real: o "jogo" troca a resolução, a saída padrão e silencia os alto-falantes; ao fechar, tudo volta.
        shell.OpenDetails(shell.ItemFor(game));
        Pump(0.8);
        Press(window, AppCommand.Accept); // Jogar
        Pump(3);
        Assert.Equal(ConsoleMode.Runtime.SessionState.Playing, shell.Session);
        Assert.NotNull(services.GetRequiredService<SessionEnvironmentGuard>().Active);
        display.Current = new DisplayTopology([SimulatedDisplayConfiguration.Default.Monitors[0] with { Width = 1280, Height = 720, RefreshRate = RefreshRate.FromHz(120) }]);
        audio.SetDefaultDevice("sim-headset", AudioRole.Console);
        audio.SetMute("sim-speakers", true);
        for (int i = 0; i < 30 && shell.Session != ConsoleMode.Runtime.SessionState.Browsing; i++)
        {
            Pump(1);
        }

        Assert.Equal(ConsoleMode.Runtime.SessionState.Browsing, shell.Session);
        Assert.True(display.Current.IsEquivalentTo(SimulatedDisplayConfiguration.Default));
        Assert.Equal("sim-speakers", audio.Current.DefaultFor(AudioRole.Console));
        Assert.False(audio.Current.LevelOf("sim-speakers")!.Muted);
        Assert.EndsWith("monitores e áudio restaurados", shell.ToastMessage, StringComparison.Ordinal);
        Pump(0.4);
        Snap(window, "41-retorno-monitores-audio");

        // Opção desligada: nada é registrado nem restaurado.
        var guard = services.GetRequiredService<SessionEnvironmentGuard>();
        Assert.Null(guard.BeginSession(game.Id, enabled: false));

        // Queda durante o jogo: o estado de antes fica em disco e a próxima abertura oferece restaurar.
        guard.BeginSession(game.Id, enabled: true);
        display.Current = new DisplayTopology([SimulatedDisplayConfiguration.Default.Monitors[0] with { Width = 1024, Height = 768 }]);
        int appliedBefore = display.AppliedCount;
        shell.OfferEnvironmentRestore();
        Pump(0.5);
        var offer = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.Equal("Restaurar monitores e áudio como estavam antes do jogo", offer.Actions[0].Label);
        Assert.Equal("overlay", Nav.GetGroup(nav.Focused!));
        Snap(window, "42-oferta-restaurar-monitores-audio");
        Press(window, AppCommand.Accept);
        Assert.Null(shell.TopOverlay);
        Assert.Equal(appliedBefore + 1, display.AppliedCount);
        Assert.True(display.Current.IsEquivalentTo(SimulatedDisplayConfiguration.Default));

        // Sem diferença: nada é oferecido.
        guard.BeginSession(game.Id, enabled: true);
        shell.OfferEnvironmentRestore();
        Assert.Null(shell.TopOverlay);
        guard.Discard();
    }

    /// <summary>
    /// Frente F: o tema Playnite escolhido vira a tela inicial (lista real, seleção, detalhes, Jogar e voltar), Menu leva
    /// às Configurações do Tako, falha volta à UI padrão com a recuperação e o modo seguro não aplica o tema. Os três temas
    /// homologados também são aplicados; erros de binding de temas de terceiros vão para o relatório, não para a
    /// verificação do launcher.
    /// </summary>
    private static void PlayniteThemeOnTheWholeUi(ServiceProvider services, ShellViewModel shell, MainWindow window, BindingErrorListener bindingErrors)
    {
        var nav = window.Navigator;
        var playnite = services.GetRequiredService<PlayniteThemeService>();
        int bindingErrorsBefore = bindingErrors.Errors.Count;
        var report = new List<string>();
        window.Dispatcher.UnhandledException += (_, e) => report.Add("  Exceção: " + string.Join(" | ", e.Exception.ToString().Split('\n').Take(25).Select(l => l.Trim())));

        shell.OpenThemes();
        Pump(0.6);
        var builtIn = ((ThemesViewModel)shell.CurrentPage).Themes.Single(t => t.Kind == ThemeKind.BuiltIn);
        shell.SelectTheme(builtIn);
        var ui = Assert.IsType<PlayniteUiViewModel>(shell.PlayniteUi);
        Assert.True(ui.Report.MainFromTheme);
        Assert.Equal("Em uso", ((ThemesViewModel)shell.CurrentPage).Themes.Single(t => t.Kind == ThemeKind.BuiltIn).Badge);
        Assert.False(shell.IsPlayniteUiVisible); // ainda em Configurações › Temas

        // Tela inicial = Main.xaml do tema, com a biblioteca real e o foco numa capa.
        shell.GoHome();
        Pump(2.5);
        Assert.True(shell.IsPlayniteUiVisible);
        Assert.Equal(shell.Library.Games.Count, ui.Model.Games.Count);
        Assert.Equal(Main.GamesGroup, Nav.GetGroup(nav.Focused!));
        AssertFocused(window, "Tela inicial do tema Playnite");
        Assert.Contains(nav.Navigables(), e => Nav.GetGroup(e) == Main.ActionsGroup && Nav.GetKey(e) == "details");
        Assert.Contains(nav.Navigables(), e => Nav.GetGroup(e) == Main.TopGroup && Nav.GetKey(e) == "menu");
        Assert.Contains(nav.Navigables(), e => Nav.GetGroup(e) == FilterPresetSelector.PresetsGroup);
        Snap(window, "60-tema-playnite-inicio");

        // → seleciona a próxima capa (PART_ListGameItems); A abre os detalhes do tema (PART_ElemGameDetails); B fecha.
        string first = Nav.GetKey(nav.Focused!)!;
        Press(window, AppCommand.Right);
        string second = Nav.GetKey(nav.Focused!)!;
        Assert.NotEqual(first, second);
        Assert.Equal(second, ui.Model.SelectedGame!.Id);
        Press(window, AppCommand.Accept);
        Pump(1.2);
        Assert.True(ui.IsDetailsOpen);
        Assert.Equal(GameDetails.DetailsGroup, Nav.GetGroup(nav.Focused!));
        Assert.Equal("play", Nav.GetKey(nav.Focused!));
        Snap(window, "61-tema-playnite-detalhes");
        Press(window, AppCommand.Back);
        Pump(0.8);
        Assert.False(ui.IsDetailsOpen);
        Assert.Equal(second, Nav.GetKey(nav.Focused!));

        // Filtro rápido (PART_ButtonFilter / FilterPresetSelector).
        ui.Model.ToggleFiltersCommand.Execute(null);
        Pump(0.8);
        Assert.Equal("Instalados", ui.Model.ActiveFilterPreset!.Name);
        Assert.All(ui.Model.Games, g => Assert.True(g.IsInstalled));
        Snap(window, "62-tema-playnite-filtro");
        ui.Model.ApplyFilterPresetCommand.Execute(ui.Model.FilterPresets[0]);
        Pump(0.8);

        // Saída garantida: Menu abre as Configurações do Tako; B volta para o tema.
        Press(window, AppCommand.Menu);
        Pump(0.8);
        Assert.IsType<SettingsViewModel>(shell.CurrentPage);
        Assert.False(shell.IsPlayniteUiVisible);
        AssertFocused(window, "Configurações a partir do tema");
        Press(window, AppCommand.Back);
        Pump(1.2);
        Assert.True(shell.IsPlayniteUiVisible);
        AssertFocused(window, "Tema depois das Configurações");

        // Jogar pelo tema passa pela sessão do Tako (Iniciando); B cancela.
        var simulated = shell.Library.Games.First(g => g.LaunchTarget is null && g.CanPlay);
        ui.Model.SelectedGame = ui.Model.Games.Single(g => g.Id == simulated.Id);
        Assert.Equal("Jogar", ui.Model.SelectedGameActionLabel);
        ui.Model.ActivateSelectedCommand.Execute(null);
        Pump(0.4);
        Assert.IsType<LaunchOverlayViewModel>(shell.TopOverlay);
        Snap(window, "63-tema-playnite-jogar");
        Press(window, AppCommand.Back);
        Assert.Null(shell.TopOverlay);
        Pump(3);
        Assert.False(shell.IsInGame);

        // MVP-021: falha do tema em uso → UI padrão e recuperação; "Usar a interface do Tako" limpa a escolha (sem loop).
        shell.RecoverFromThemeFailure("Falha simulada pelo teste de fumaça");
        Pump(0.6);
        Assert.Null(shell.PlayniteUi);
        var recovery = Assert.IsType<RecoveryViewModel>(shell.TopOverlay);
        Assert.Equal("O tema não pôde ser aplicado", recovery.Title);
        AssertFocused(window, "Recuperação do tema Playnite");
        Snap(window, "64-tema-playnite-recuperacao");
        Press(window, AppCommand.Accept);
        Pump(0.6);
        Assert.Null(shell.TopOverlay);
        Assert.Null(playnite.SelectedId);
        Assert.False(shell.IsPlayniteUiVisible);
        AssertFocused(window, "Início do Tako depois da falha do tema");

        // Modo seguro: o tema escolhido não é aplicado; ao sair, volta.
        playnite.Select(builtIn.Theme);
        shell.IsSafeMode = true;
        shell.ApplyPlayniteUi();
        Assert.Null(shell.PlayniteUi);
        shell.IsSafeMode = false;
        shell.ApplyPlayniteUi();
        Assert.NotNull(shell.PlayniteUi);
        Pump(1);

        // Temas homologados aplicados à interface inteira (mesmos pacotes fixados da frente C).
        foreach (var fixture in ThemeFixtures.Homologated)
        {
            string? package = ThemeFixtures.TryDownload(fixture, out string? problem);
            if (package is null)
            {
                report.Add($"{fixture.Name}: não executado ({problem})");
                continue;
            }

            var installed = playnite.Import(package);
            Assert.True(installed.Succeeded, string.Join('\n', installed.Diagnostics));
            var theme = playnite.Imported().Single(t => string.Equals(t.Directory, installed.Directory, StringComparison.OrdinalIgnoreCase));
            playnite.Select(theme);
            shell.ApplyPlayniteUi();
            Pump(2.5);
            if (shell.PlayniteUi is not { } applied)
            {
                string detail = (shell.TopOverlay as RecoveryViewModel)?.Detail ?? "sem detalhe";
                report.Add($"{fixture.Name}: interface NÃO aplicada ({detail})");
                File.WriteAllLines(Path.Combine(Screenshots, "theme-whole-ui.txt"), report);
                Snap(window, $"65-tema-{fixture.Slug}-interface-falha");
                Assert.Fail($"{fixture.Name}: tema não aplicado à interface: {detail}");
                return;
            }

            Assert.True(shell.IsPlayniteUiVisible);
            Assert.True(fixture.MainLoads == applied.Report.MainFromTheme, $"{fixture.Name}: Main.xaml do tema esperado={fixture.MainLoads}");
            AssertFocused(window, $"Tela inicial do tema {fixture.Name}");
            report.Add($"{fixture.Name}: interface aplicada · Main.xaml {(applied.Report.MainFromTheme ? "do tema" : "herdado do tema padrão")} · {applied.Report.Loaded.Count} de {applied.Report.Total} arquivos do tema · {applied.Model.Games.Count} jogos");
            report.AddRange(applied.Report.Rejected.Select(x => $"    RECUSA  {x.File} — {x.Reason}"));
            Snap(window, $"65-tema-{fixture.Slug}-interface");
            Press(window, AppCommand.Accept);
            Pump(1.2);
            Assert.True(applied.IsDetailsOpen, $"{fixture.Name}: A não abriu os detalhes");
            AssertFocused(window, $"Detalhes do tema {fixture.Name}");
            Snap(window, $"66-tema-{fixture.Slug}-detalhes");
            Press(window, AppCommand.Back);
            Pump(0.6);
            Assert.False(applied.IsDetailsOpen);
        }

        // Volta à interface do Tako e remove os temas importados aqui.
        playnite.Select(null);
        shell.ApplyPlayniteUi();
        Assert.Null(shell.PlayniteUi);
        foreach (var theme in playnite.Imported())
        {
            try
            {
                playnite.Remove(theme);
            }
            catch (IOException ex)
            {
                // Imagens que o WPF ainda mantém abertas; o app mostra o mesmo aviso em Configurações › Temas.
                report.Add($"{theme.Manifest.Manifest.DisplayName}: pasta mantida ({ex.Message})");
            }
        }

        Pump(0.6);
        AssertFocused(window, "Início do Tako depois dos temas");
        var themeBindingErrors = bindingErrors.Errors.Skip(bindingErrorsBefore).ToList();
        bindingErrors.Errors.RemoveRange(bindingErrorsBefore, themeBindingErrors.Count);
        report.Add($"Erros de binding dos temas na interface (dados do Playnite sem equivalente no Tako): {themeBindingErrors.Count}");
        report.AddRange(themeBindingErrors.Select(e => e.Split(';')[0]).Distinct(StringComparer.Ordinal).Take(40).Select(e => "  " + e));
        File.WriteAllLines(Path.Combine(Screenshots, "theme-whole-ui.txt"), report);
    }

    /// <summary>Há um elemento focado, visível e alcançável pela navegação no escopo atual.</summary>
    private static void AssertFocused(MainWindow window, string screen)
    {
        var focused = window.Navigator.Focused;
        Assert.True(focused is not null, $"{screen}: nenhum elemento focado");
        Assert.True(focused.IsVisible, $"{screen}: o foco está num elemento invisível ({Nav.GetKey(focused)})");
        Assert.True(window.Navigator.Navigables().Contains(focused), $"{screen}: o foco está fora do escopo navegável ({Nav.GetKey(focused)})");
    }

    private static void Press(MainWindow window, AppCommand command)
    {
        window.Dispatch(command);
        Pump(0.25);
    }

    private static void Pump(double seconds)
    {
        var frame = new DispatcherFrame();
        var timer = new DispatcherTimer(DispatcherPriority.Background) { Interval = TimeSpan.FromSeconds(seconds) };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            frame.Continue = false;
        };
        timer.Start();
        Dispatcher.PushFrame(frame);
    }

    /// <summary>Renderiza o palco (1080 de altura, largura conforme a proporção) para PNG; <paramref name="scale"/> 2 = 4K.</summary>
    private static void Snap(MainWindow window, string name, double scale = 1)
    {
        var stage = window.StageElement;
        stage.UpdateLayout();
        double width = stage.ActualWidth;
        var visual = new DrawingVisual();
        using (var context = visual.RenderOpen())
        {
            context.DrawRectangle(new VisualBrush(stage) { Stretch = Stretch.None, AlignmentX = AlignmentX.Left, AlignmentY = AlignmentY.Top }, null, new Rect(0, 0, width, 1080));
        }

        var bitmap = new RenderTargetBitmap((int)Math.Round(width * scale), (int)Math.Round(1080 * scale), 96 * scale, 96 * scale, PixelFormats.Pbgra32);
        bitmap.Render(visual);
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        using var file = File.Create(Path.Combine(Screenshots, name + ".png"));
        encoder.Save(file);
    }

    private static void RunOnSta(Action action)
    {
        Exception? failure = null;
        var thread = new Thread(() =>
        {
            try
            {
                action();
            }
            catch (Exception ex) when (ex is not OutOfMemoryException)
            {
                failure = ex;
            }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        thread.Join();
        if (failure is not null)
        {
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(failure).Throw();
        }
    }

    private sealed class BindingErrorListener : TraceListener
    {
        public List<string> Errors { get; } = [];

        public override void Write(string? message)
        {
        }

        public override void WriteLine(string? message)
        {
            if (message is not null)
            {
                Errors.Add(message);
            }
        }
    }
}
