using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Dados dos jogos (IGDB), ADR-0021. Bloco isolado (uma linha nas Configurações e uma sobreposição) para
/// a reorganização das Configurações movê-lo para "Biblioteca e lojas" sem mexer na lógica.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>"Desligada", "Ligada · sua chave da Twitch" ou "Ligada · falta a chave".</summary>
    public string IgdbSummary => _metadata.IgdbSettings switch
    {
        { IsActive: true } => "Ligada · sua chave da Twitch",
        { Enabled: true } => "Ligada · falta a chave",
        _ => "Desligada",
    };

    /// <summary>Aviso discreto nos detalhes: jogo sem dados que poderia ganhá-los com a IGDB ligada.</summary>
    public string? MetadataHintFor(Game game) => _metadata.CouldUseIgdb(game)
        ? "Sem informações deste jogo? Ligue a IGDB em Configurações › Biblioteca e lojas › Dados dos jogos para buscar em mais fontes."
        : null;

    [RelayCommand]
    private void OpenIgdbSettings()
    {
        IgdbSettingsViewModel? editor = null;
        editor = new IgdbSettingsViewModel(
            _metadata.IgdbSettings,
            settings => _metadata.TestIgdbAsync(settings, CancellationToken.None),
            saved =>
            {
                CloseOverlay(editor!);
                _metadata.SaveIgdb(saved);
                OnPropertyChanged(nameof(IgdbSummary));
                CurrentPage.Refresh();
                if (saved.IsActive)
                {
                    // Só os jogos que ainda não passaram pela IGDB são consultados, aos poucos.
                    _ = _metadata.RefreshAsync(() => Library.Games);
                    Toast("IGDB ligada · buscando dados dos jogos em segundo plano");
                }
                else
                {
                    Toast("IGDB desligada · os dados vêm só da Steam");
                }
            },
            () => OpenUri(IgdbSettingsViewModel.ConsoleUrl),
            ReadClipboard,
            () => CloseOverlay(editor!));
        PushOverlay(editor);
    }

    private static string? ReadClipboard()
    {
        try
        {
            return System.Windows.Clipboard.ContainsText() ? System.Windows.Clipboard.GetText() : null;
        }
        catch (System.Runtime.InteropServices.COMException)
        {
            return null; // outro programa segurando a área de transferência
        }
    }
}
