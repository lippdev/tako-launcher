using System.Diagnostics;
using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Launcher.Core.Input;
using Launcher.Core.Library;
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
public sealed class UiSmokeTests
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
        using var services = App.BuildServices();
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
        Assert.True(preview.Report.Loaded.Count >= 20, preview.Summary);
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

        File.WriteAllLines(Path.Combine(Screenshots, "binding-errors.txt"), bindingErrors.Errors);
        window.Close();
        Assert.DoesNotContain(bindingErrors.Errors, e => !e.Contains("Error: 4 :", StringComparison.Ordinal));
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

    /// <summary>Renderiza o palco 1920x1080 (independente do tamanho da janela) para PNG.</summary>
    private static void Snap(MainWindow window, string name)
    {
        var stage = window.StageElement;
        stage.UpdateLayout();
        var visual = new DrawingVisual();
        using (var context = visual.RenderOpen())
        {
            context.DrawRectangle(new VisualBrush(stage) { Stretch = Stretch.None, AlignmentX = AlignmentX.Left, AlignmentY = AlignmentY.Top }, null, new Rect(0, 0, 1920, 1080));
        }

        var bitmap = new RenderTargetBitmap(1920, 1080, 96, 96, PixelFormats.Pbgra32);
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
