using System.IO;
using Launcher.Core.Emulation;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.ScreenScraper;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// Épico EMU (ADR-0024): Configurações › Emulação e o assistente; pasta de ROMs com subpastas por sistema (lida de
    /// verdade, com multi-disco); emulador personalizado (o PING do Windows faz o papel do emulador: abre, fica aberto e
    /// não lê a ROM); editor de argumentos; abrir a ROM pela sessão de sempre; "sair do emulador" chegando à confirmação
    /// de encerrar; e a volta ao Tako. Nenhum emulador, BIOS ou ROM real.
    /// </summary>
    private static void EmulationJourney(ShellViewModel shell, MainWindow window, EmulationService emulation)
    {
        var nav = window.Navigator;
        string root = Path.Combine(Path.GetTempPath(), $"tako-smoke-emu-{Guid.NewGuid():N}");
        string snes = Path.Combine(root, "roms", "snes", "Lanterna Distante (USA).sfc");
        Directory.CreateDirectory(Path.GetDirectoryName(snes)!);
        Directory.CreateDirectory(Path.Combine(root, "roms", "psx"));
        File.WriteAllText(snes, "rom falsa");
        File.WriteAllText(Path.Combine(root, "roms", "psx", "Saga Estelar (USA) (Disc 1).chd"), "disco falso");
        File.WriteAllText(Path.Combine(root, "roms", "psx", "Saga Estelar (USA) (Disc 2).chd"), "disco falso");

        try
        {
            // Configurações › Emulação: categoria própria, com o assistente na primeira linha.
            shell.OpenSettings();
            Pump(0.6);
            var settings = Assert.IsType<SettingsViewModel>(shell.CurrentPage);
            Assert.True(nav.FocusDefault("settings:emu-setup"));
            Assert.Equal("emulacao", settings.SelectedCategory.Id);
            Pump(0.3);
            Snap(window, "90-configuracoes-emulacao");

            Press(window, AppCommand.Accept);
            Pump(0.5);
            var wizard = Assert.IsType<SetupWizardViewModel>(shell.TopOverlay);
            Assert.Equal("Configurar emulação", wizard.Heading);
            Assert.Equal(["Emuladores", "ROMs", "Sistemas", "Pronto"], wizard.Steps.Select(s => s.Label));
            Assert.Equal("overlay", Nav.GetGroup(nav.Focused!));
            Snap(window, "91-assistente-emulacao");
            Press(window, AppCommand.Accept); // Continuar → pastas de ROMs
            Pump(0.3);
            Assert.Equal("Onde estão os seus jogos?", wizard.Title);
            Press(window, AppCommand.Back); // volta um passo
            Press(window, AppCommand.Back); // fecha
            Assert.Null(shell.TopOverlay);

            // Pasta raiz com subpastas por sistema: o Tako pergunta e lê os jogos de verdade (2 discos viram 1 jogo).
            shell.AddRomFolder(Path.Combine(root, "roms"));
            Pump(0.4);
            var how = Assert.IsType<DialogViewModel>(shell.TopOverlay);
            Assert.Equal("Usar as subpastas", how.Actions[0].Label);
            Assert.Contains("SNES", how.Text, StringComparison.Ordinal);
            how.Actions[0].Command.Execute(null);
            Pump(0.4);
            Assert.Single(emulation.Config.RomFolders);
            Assert.StartsWith("Pasta adicionada", shell.ToastMessage, StringComparison.Ordinal);
            var scan = emulation.Source.Scan();
            Assert.Equal(["Lanterna Distante", "Saga Estelar"], scan.Games.Select(g => g.Title).Order(StringComparer.Ordinal));
            Assert.Equal("2 discos", scan.Games.Single(g => g.Platform == "ps1").SizeLabel);

            shell.OpenRomFolders();
            Pump(0.5);
            var folders = Assert.IsType<SettingsDetailViewModel>(shell.CurrentPage);
            Assert.Equal("Emulação", folders.Overline);
            Assert.Equal("settings", folders.RouteKey);
            Assert.Equal("emu:folder:0", Nav.GetKey(nav.Focused!));
            Snap(window, "92-pastas-de-roms");
            Press(window, AppCommand.Back);
            Assert.IsType<SettingsViewModel>(shell.CurrentPage);

            // Emulador personalizado: o PING abre "jogos de SNES" (não lê a ROM e fica aberto até ser fechado).
            string ping = Path.Combine(Environment.SystemDirectory, "PING.EXE");
            var custom = shell.AddCustomEmulator(ping, ["snes"]);
            Pump(0.5);
            Assert.IsType<SettingsDetailViewModel>(shell.CurrentPage);
            Snap(window, "93-emulador-personalizado");
            emulation.Update(c => c with { CustomEmulators = [.. c.CustomEmulators.Select(e => e.Id == custom.Id ? e with { Arguments = "-n 60 127.0.0.1" } : e)] });
            Assert.True(emulation.Inventory.Find(custom.Id)!.Serves("snes"));

            // Editor de argumentos pelo controle: marcadores com A, marcador desconhecido não salva.
            shell.OpenEmulator(custom.Id);
            Pump(0.5);
            Assert.True(nav.FocusDefault($"emu:{custom.Id}:args"));
            Press(window, AppCommand.Accept);
            Pump(0.4);
            var editor = Assert.IsType<ArgumentsEditorViewModel>(shell.TopOverlay);
            Assert.Equal("-n 60 127.0.0.1", editor.Text);
            Assert.StartsWith("Exemplo: PING.EXE -n 60", editor.Preview, StringComparison.OrdinalIgnoreCase);
            editor.Placeholders[0].Command.Execute(null);
            Assert.EndsWith("{rom}", editor.Text, StringComparison.Ordinal);
            editor.Text = "{rompath}";
            editor.SaveCommand.Execute(null);
            Assert.True(editor.HasError);
            Snap(window, "94-argumentos-do-emulador");
            editor.CancelCommand.Execute(null);
            Assert.Null(shell.TopOverlay);

            // Emulador de cada sistema: o personalizado aparece para o SNES.
            shell.OpenEmulationSystems();
            Pump(0.5);
            Assert.True(nav.FocusDefault("emu:system:snes"));
            Press(window, AppCommand.Accept);
            Pump(0.3);
            var choose = Assert.IsType<DialogViewModel>(shell.TopOverlay);
            Assert.Contains(choose.Actions, a => a.Label.StartsWith("PING", StringComparison.OrdinalIgnoreCase));
            Snap(window, "95-emulador-do-sistema");
            Press(window, AppCommand.Back);

            // A ROM abre pela sessão de sempre (tela "Iniciando", acompanhamento do processo) e mostra o emulador.
            var system = ScreenScraperSystems.ForKey("snes")!;
            var game = shell.AddRomWithSystem(snes, system);
            Pump(0.8);
            var details = Assert.IsType<DetailsViewModel>(shell.CurrentPage);
            Assert.Equal("Jogar", details.PrimaryLabel);
            Assert.Contains(details.Stats, s => s.Label == "Emulador" && s.Value.StartsWith("PING", StringComparison.OrdinalIgnoreCase));
            Snap(window, "96-detalhes-rom-emulador");
            Press(window, AppCommand.Accept); // Jogar
            Assert.IsType<LaunchOverlayViewModel>(shell.TopOverlay);
            Pump(3.5);
            Assert.Equal(ConsoleMode.Runtime.SessionState.Playing, shell.Session);
            Assert.Equal(game.Id, shell.RunningGame!.Id);
            Assert.True(shell.CanExitEmulatorFromController);

            // Sair do emulador: o PING não tem janela nem tecla de sair, então o Tako pergunta antes de encerrar.
            shell.ExitEmulator();
            for (int i = 0; i < 20 && shell.TopOverlay is not DialogViewModel; i++)
            {
                Pump(0.5);
            }

            var force = Assert.IsType<DialogViewModel>(shell.TopOverlay);
            Assert.EndsWith("não fechou", force.Title, StringComparison.Ordinal);
            Assert.Equal(["Encerrar o emulador", "Voltar ao jogo"], force.Actions.Select(a => a.Label));
            Snap(window, "97-sair-do-emulador");
            force.Actions[0].Command.Execute(null);
            for (int i = 0; i < 30 && shell.Session != ConsoleMode.Runtime.SessionState.Browsing; i++)
            {
                Pump(0.5);
            }

            Assert.Equal(ConsoleMode.Runtime.SessionState.Browsing, shell.Session);
            Assert.Null(shell.RunningGame);
            Assert.False(shell.CanExitEmulatorFromController);
            Assert.StartsWith("Bem-vindo de volta", shell.ToastMessage, StringComparison.Ordinal);

            // Opções do jogo: emulador, argumentos e (no RetroArch) o .cfg deste jogo.
            shell.ShowGameOptions(shell.ItemFor(shell.Library.Find(game.Id)!));
            Pump(0.3);
            var options = Assert.IsType<DialogViewModel>(shell.TopOverlay);
            Assert.Contains(options.Actions, a => a.Label == "Emulador deste jogo");
            Assert.Contains(options.Actions, a => a.Label == "Argumentos deste jogo");
            shell.CloseOverlay(options);
            shell.Library.RemoveManual(game.Id);
        }
        finally
        {
            emulation.Update(_ => EmulationConfig.Default);
            Pump(0.3);
            Directory.Delete(root, recursive: true);
        }
    }
}
