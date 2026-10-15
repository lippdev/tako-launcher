using System.Collections.ObjectModel;
using System.Windows;
using System.Windows.Input;
using Launcher.PlayniteCompat.P10;
using Launcher.PlayniteCompat.P10.Data;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>Prévia de um tema Fullscreen do Playnite 10 renderizado com a biblioteca do launcher (H2).</summary>
public sealed class PlaynitePreviewViewModel : IOverlayViewModel
{
    public PlaynitePreviewViewModel(P10ThemeReport report, IEnumerable<GameItemViewModel> games, ICommand close)
    {
        Report = report;
        Close = close;
        foreach (var game in games)
        {
            Games.Add(PlayniteUiViewModel.ToEntry(game));
        }

        string inherited = report.Inherited.Count > 0 ? $" ({report.Inherited.Count} herdados do tema padrão)" : string.Empty;
        Summary = $"{report.Name} {report.Version} · API de tema {report.ThemeApiVersion} · {report.Loaded.Count} de {report.Total} arquivos carregados{inherited}";
        RejectedLines = report.Rejected.Select(r => $"{r.File} — {r.Reason}").ToList();
    }

    public P10ThemeReport Report { get; }

    public ResourceDictionary Resources => Report.Resources;

    public ObservableCollection<P10GameEntry> Games { get; } = [];

    public string Summary { get; }

    public IReadOnlyList<string> RejectedLines { get; }

    public ICommand Close { get; }
}
