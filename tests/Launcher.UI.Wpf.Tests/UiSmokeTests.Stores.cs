using System.IO;
using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Tests;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// ADR-0019: Configurações › Lojas. Steam, Epic Games e GOG aparecem com o estado da última leitura; A no controle
    /// desliga a loja (a biblioteca é lida de novo) e A de novo religa.
    /// </summary>
    private static void StoreSettings(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        shell.OpenSettings();
        Pump(0.8);
        var settings = Assert.IsType<SettingsViewModel>(shell.CurrentPage);
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Steam));
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Epic));
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Gog));
        Assert.False(string.IsNullOrWhiteSpace(settings.StoreStatus(GameStore.Epic)));

        Assert.True(nav.FocusDefault("settings:store:epic"));
        Assert.Equal("biblioteca", settings.SelectedCategory.Id); // a linha estava em outra categoria: ela abre sozinha
        Press(window, AppCommand.Accept);
        Assert.False(shell.Settings.IsStoreEnabled(GameStore.Epic));
        Assert.StartsWith("Epic Games desligada", shell.ToastMessage, StringComparison.Ordinal);
        Assert.Equal("settings:store:epic", Nav.GetKey(nav.Focused!));
        Pump(0.4);
        Snap(window, "75-configuracoes-lojas");

        Press(window, AppCommand.Accept);
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.Epic));
        Assert.StartsWith("Epic Games ligada", shell.ToastMessage, StringComparison.Ordinal);

        Assert.True(nav.FocusDefault("settings:store:gog"));
        Assert.Equal("settings:store:gog", Nav.GetKey(nav.Focused!));

        // Ubisoft Connect, EA app, Battle.net e Amazon Games (STORE-005 a 008): mesma linha, mesmo liga/desliga.
        foreach (var (key, store) in new[] { ("ubisoft", GameStore.Ubisoft), ("ea", GameStore.Ea), ("battlenet", GameStore.BattleNet), ("amazon", GameStore.Amazon) })
        {
            Assert.True(nav.FocusDefault($"settings:store:{key}"));
            Assert.True(shell.Settings.IsStoreEnabled(store));
            Assert.False(string.IsNullOrWhiteSpace(settings.StoreStatus(store)));
        }

        Assert.True(nav.FocusDefault("settings:store:battlenet"));
        Press(window, AppCommand.Accept);
        Assert.False(shell.Settings.IsStoreEnabled(GameStore.BattleNet));
        Assert.StartsWith("Battle.net desligad", shell.ToastMessage, StringComparison.Ordinal);
        Pump(0.4);
        Snap(window, "76-configuracoes-mais-lojas");
        Press(window, AppCommand.Accept);
        Assert.True(shell.Settings.IsStoreEnabled(GameStore.BattleNet));

        PlayniteImportFlow(shell, window);
    }

    /// <summary>
    /// STORE-009: Importar do Playnite pede consentimento, lê a biblioteca de exemplo (formato do LiteDB 4), mostra a
    /// prévia e só grava depois de confirmar; os arquivos do Playnite não mudam.
    /// </summary>
    private static void PlayniteImportFlow(ShellViewModel shell, MainWindow window)
    {
        var nav = window.Navigator;
        string folder = Path.Combine(Path.GetTempPath(), $"tako-smoke-playnite-{Guid.NewGuid():N}");
        PlayniteSample.Write(folder);
        try
        {
            var stamp = File.GetLastWriteTimeUtc(Path.Combine(folder, "games.db"));
            shell.PlayniteLibraryFolder = folder;
            Assert.True(nav.FocusDefault("settings:playnite-import"));
            Press(window, AppCommand.Accept);
            var consent = Assert.IsType<DialogViewModel>(shell.TopOverlay);
            Assert.Equal("Importar do Playnite?", consent.Title);
            Assert.Contains("nada muda no Playnite", consent.Text, StringComparison.Ordinal);
            Pump(0.4);
            Snap(window, "77-importar-playnite");

            Press(window, AppCommand.Accept); // "Ler biblioteca"
            Until(() => shell.TopOverlay is DialogViewModel { Title: "Prévia da importação" });
            var preview = (DialogViewModel)shell.TopOverlay!;
            Assert.NotNull(shell.PendingPlayniteImport);
            Assert.Contains("jogos novos", preview.Text, StringComparison.Ordinal);
            Assert.Null(shell.Library.Find(Core.Library.Playnite.PlayniteImport.ImportedId(PlayniteSample.Lanterna))); // nada gravado antes de confirmar
            Pump(0.4);
            Snap(window, "78-importar-playnite-previa");

            Press(window, AppCommand.Accept); // "Importar"
            WaitForNoOverlay(shell);
            Assert.Contains("importado", shell.ToastMessage, StringComparison.Ordinal);
            var lanterna = shell.Library.Find(Core.Library.Playnite.PlayniteImport.ImportedId(PlayniteSample.Lanterna));
            Assert.NotNull(lanterna);
            Assert.True(lanterna.IsFavorite);
            Assert.Contains(shell.Collections.Collections, c => c.Name == "Jogar no sofá" && c.Contains(lanterna.Id));
            Assert.Equal(stamp, File.GetLastWriteTimeUtc(Path.Combine(folder, "games.db")));
        }
        finally
        {
            shell.PlayniteLibraryFolder = null;
            Directory.Delete(folder, recursive: true);
        }
    }
}
