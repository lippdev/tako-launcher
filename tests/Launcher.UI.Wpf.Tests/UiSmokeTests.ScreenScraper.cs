using System.IO;
using Launcher.Core.Input;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// ADR-0022: Configurações › Dados de ROMs (ScreenScraper) e ROM no "Adicionar jogo". A sobreposição tem o passo a
    /// passo; "Testar" sem conta avisa o que falta; conta errada dá mensagem clara; conta certa mostra nível e cota; Salvar
    /// liga. A ROM pede o sistema, preenche a plataforma e não abre. "Testar" é simulado e não há provedores registrados:
    /// o CI nunca fala com o ScreenScraper.
    /// </summary>
    private static void ScreenScraperDataSettings(ShellViewModel shell, MainWindow window, MetadataService metadata)
    {
        var nav = window.Navigator;
        var tested = new List<ScreenScraperSettings>();
        metadata.ScreenScraperTester = (settings, _) =>
        {
            tested.Add(settings);
            return settings.Password == "errada"
                ? Task.FromException<ScreenScraperAccount>(new ScreenScraperException(ScreenScraperFailure.BadUser, "login"))
                : Task.FromResult(new ScreenScraperAccount(settings.User, 1, 1, 20, 20000));
        };

        // Versão sem as credenciais do software: aviso claro já ao abrir, "Testar" não vai à rede.
        var unavailable = new ScreenScraperSettingsViewModel(ScreenScraperSettings.Off, hasSoftwareAccess: false, _ => throw new InvalidOperationException(), _ => { }, () => { }, () => null, () => { });
        Assert.Equal(ScreenScraperErrors.NoSoftwareAccess, unavailable.Status);
        Assert.True(unavailable.StatusIsError);

        shell.OpenSettings();
        Pump(0.8);
        Assert.Equal("Desligado", shell.ScreenScraperSummary);
        Assert.True(nav.FocusDefault("settings:screenscraper"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        var editor = Assert.IsType<ScreenScraperSettingsViewModel>(shell.TopOverlay);
        Assert.Equal("ss:enabled", Nav.GetKey(nav.Focused!));
        Assert.True(editor.HasSoftwareAccess);
        Assert.Equal(3, editor.Steps.Count);
        Assert.Contains("screenscraper.fr", editor.Steps[0], StringComparison.Ordinal);

        // Sem conta: avisa o que falta, sem ir à rede.
        editor.TestCommand.Execute(null);
        Assert.True(editor.StatusIsError);
        Assert.Contains("usuário", editor.Status, StringComparison.Ordinal);
        Assert.Empty(tested);

        Press(window, AppCommand.Accept);
        Assert.True(editor.Enabled);
        editor.User = " jogador ";
        editor.Password = "errada";
        editor.TestCommand.Execute(null);
        Pump(0.4);
        Assert.True(editor.StatusIsError);
        Assert.Contains("Usuário ou senha", editor.Status, StringComparison.Ordinal);
        Assert.Equal("jogador", tested[^1].User);

        editor.Password = "certa";
        editor.TestCommand.Execute(null);
        Pump(0.4);
        Assert.False(editor.StatusIsError);
        Assert.StartsWith("Tudo certo: conta jogador, nível 1", editor.Status, StringComparison.Ordinal);
        Assert.Contains("19.980 de 20.000 consultas restam hoje", editor.Status, StringComparison.Ordinal);
        Snap(window, "78-configuracoes-screenscraper");

        editor.SaveCommand.Execute(null);
        Pump(0.4);
        Assert.Null(shell.TopOverlay);
        Assert.True(metadata.ScreenScraperSettings.IsActive);
        Assert.Equal("Ligado · sua conta", shell.ScreenScraperSummary);
        Assert.StartsWith("ScreenScraper ligado", shell.ToastMessage, StringComparison.Ordinal);

        // Reabrir: a senha salva não volta para a tela; campo vazio mantém a salva.
        Assert.True(nav.FocusDefault("settings:screenscraper"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        editor = Assert.IsType<ScreenScraperSettingsViewModel>(shell.TopOverlay);
        Assert.Equal(string.Empty, editor.Password);
        Assert.Contains("já salva", editor.PasswordLabel, StringComparison.Ordinal);
        Assert.Equal("certa", editor.Build().Password);
        editor.ToggleEnabledCommand.Execute(null);
        editor.SaveCommand.Execute(null);
        Pump(0.4);
        Assert.Equal("Desligado", shell.ScreenScraperSummary);

        // ROM no "Adicionar jogo": pergunta o sistema (o provável primeiro), preenche a plataforma e não abre.
        string rom = Path.Combine(Path.GetTempPath(), $"tako-smoke-{Guid.NewGuid():N}", "Lanterna Distante (USA).sfc");
        Directory.CreateDirectory(Path.GetDirectoryName(rom)!);
        File.WriteAllText(rom, "rom falsa");
        shell.AddRom(rom);
        Pump(0.5);
        var picker = Assert.IsType<DialogViewModel>(shell.TopOverlay);
        Assert.Equal(["SNES", "Outro sistema", "Cancelar"], picker.Actions.Select(a => a.Label));
        Snap(window, "79-rom-escolher-sistema");
        picker.Actions[0].Command.Execute(null);
        Pump(0.6);
        var details = Assert.IsType<DetailsViewModel>(shell.CurrentPage);
        Assert.Equal("Lanterna Distante", details.Game.Title);
        Assert.Equal("snes", details.Game.Game.Platform);
        Assert.Equal("Abrir no emulador (em breve)", details.PrimaryLabel);
        Assert.Contains(details.Stats, s => s.Label == "Sistema" && s.Value == "SNES");
        Snap(window, "80-detalhes-rom");
        details.PrimaryCommand.Execute(null);
        Assert.Equal(ShellViewModel.RomLaunchNotice, shell.ToastMessage);
        Assert.Null(shell.RunningGame);
        shell.Library.RemoveManual(details.Game.Id);
        Pump(0.3);
        Directory.Delete(Path.GetDirectoryName(rom)!, recursive: true);
    }
}
