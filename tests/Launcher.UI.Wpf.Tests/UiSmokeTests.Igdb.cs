using Launcher.Core.Input;
using Launcher.Core.Library;
using Launcher.Core.Library.Metadata.Igdb;
using Launcher.UI.Wpf.Navigation;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

public sealed partial class UiSmokeTests
{
    /// <summary>
    /// ADR-0021: Configurações › Dados dos jogos (IGDB). Desligada por padrão; jogo sem dados mostra o aviso discreto nos
    /// detalhes; a sobreposição tem o passo a passo, "Testar" sem chave avisa o que falta, chave inválida dá mensagem
    /// clara, e Salvar liga. "Testar" é simulado e não há provedores registrados: o CI nunca fala com a Twitch.
    /// </summary>
    private static void IgdbDataSettings(ShellViewModel shell, MainWindow window, MetadataService metadata)
    {
        var nav = window.Navigator;
        var tested = new List<IgdbSettings>();
        metadata.IgdbTester = (settings, _) =>
        {
            tested.Add(settings);
            return settings.ClientSecret.StartsWith("errado", StringComparison.Ordinal)
                ? Task.FromException(new IgdbException(IgdbFailure.InvalidSecret, "403"))
                : Task.CompletedTask;
        };

        // Jogo sem dados e IGDB desligada: aviso discreto nos detalhes.
        var bare = shell.Library.AddManual(ManualGameRecord.Create("Jogo Sem Dados", @"C:\Jogos\SemDados\jogo.exe", DateTimeOffset.Now));
        Pump(0.4);
        shell.OpenDetails(shell.ItemFor(bare));
        Pump(0.6);
        var details = Assert.IsType<DetailsViewModel>(shell.CurrentPage);
        Assert.Contains("Dados dos jogos", details.MetadataHint, StringComparison.Ordinal);
        Snap(window, "76-detalhes-aviso-igdb");

        shell.OpenSettings();
        Pump(0.8);
        Assert.Equal("Desligada", shell.IgdbSummary);
        Assert.True(nav.FocusDefault("settings:igdb"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        var editor = Assert.IsType<IgdbSettingsViewModel>(shell.TopOverlay);
        Assert.Equal("igdb:enabled", Nav.GetKey(nav.Focused!));
        Assert.Equal(4, editor.Steps.Count);
        Assert.Contains("http://localhost", editor.Steps[1], StringComparison.Ordinal);

        // Sem chave: avisa o que falta, sem ir à rede.
        editor.TestCommand.Execute(null);
        Assert.True(editor.StatusIsError);
        Assert.Contains("Client ID", editor.Status, StringComparison.Ordinal);
        Assert.Empty(tested);

        Press(window, AppCommand.Accept);
        Assert.True(editor.Enabled);
        editor.ClientId = " abcdefghijklmnopqrstuvwxyz0123\n";
        editor.ClientSecret = "errado0123456789errado01234567";
        editor.TestCommand.Execute(null);
        Pump(0.4);
        Assert.True(editor.StatusIsError);
        Assert.Contains("Novo segredo", editor.Status, StringComparison.Ordinal);
        Assert.Equal("abcdefghijklmnopqrstuvwxyz0123", tested[^1].ClientId); // espaços colados saem

        editor.ClientSecret = "certo01234567890certo012345678";
        editor.TestCommand.Execute(null);
        Pump(0.4);
        Assert.False(editor.StatusIsError);
        Assert.StartsWith("Tudo certo", editor.Status, StringComparison.Ordinal);
        Snap(window, "77-configuracoes-igdb");

        editor.SaveCommand.Execute(null);
        Pump(0.4);
        Assert.Null(shell.TopOverlay);
        Assert.True(metadata.IgdbSettings.IsActive);
        Assert.Equal("Ligada · sua chave da Twitch", shell.IgdbSummary);
        Assert.StartsWith("IGDB ligada", shell.ToastMessage, StringComparison.Ordinal);
        Assert.Null(shell.MetadataHintFor(bare));

        // Reabrir: o segredo salvo não volta para a tela; campo vazio mantém o salvo. Desligar volta ao padrão.
        Assert.True(nav.FocusDefault("settings:igdb"));
        Press(window, AppCommand.Accept);
        Pump(0.5);
        editor = Assert.IsType<IgdbSettingsViewModel>(shell.TopOverlay);
        Assert.Equal(string.Empty, editor.ClientSecret);
        Assert.Contains("já salvo", editor.SecretLabel, StringComparison.Ordinal);
        Assert.Equal("certo01234567890certo012345678", editor.Build().ClientSecret);
        editor.ToggleEnabledCommand.Execute(null);
        editor.SaveCommand.Execute(null);
        Pump(0.4);
        Assert.Equal("Desligada", shell.IgdbSummary);
        Assert.False(metadata.IgdbSettings.Enabled);
        shell.Library.RemoveManual(bare.Id);
        Pump(0.3);
    }
}
