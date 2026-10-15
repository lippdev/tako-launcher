using System.IO;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Library;
using Launcher.Core.Session;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Editar jogo adicionado (MVP-024): campos de texto pelo teclado; o controle navega entre campos e botões.
/// O teclado virtual para digitar só com o controle é o UX-001 (pós-MVP).
/// </summary>
public sealed partial class EditGameViewModel : ObservableObject, IOverlayViewModel
{
    private static readonly (GameTrackingStrategy? Value, string Label)[] TrackingOptions =
    [
        (null, "Automático"),
        (GameTrackingStrategy.Process, "Processo iniciado"),
        (GameTrackingStrategy.InstallFolder, "Pasta do jogo"),
        (GameTrackingStrategy.None, "Não acompanhar"),
    ];

    private readonly ManualGameRecord _record;
    private readonly Action<ManualGameRecord> _save;
    private readonly Action _cancel;

    [ObservableProperty]
    private string _name;

    [ObservableProperty]
    private string _arguments;

    [ObservableProperty]
    private string _workingDirectory;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(TrackingLabel))]
    private int _trackingIndex;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasError))]
    private string? _error;

    public EditGameViewModel(ManualGameRecord record, Action<ManualGameRecord> save, Action cancel)
    {
        ArgumentNullException.ThrowIfNull(record);
        _record = record;
        _save = save;
        _cancel = cancel;
        _name = record.Title;
        _arguments = record.Arguments ?? string.Empty;
        _workingDirectory = record.WorkingDirectory ?? string.Empty;
        _trackingIndex = Math.Max(0, Array.FindIndex(TrackingOptions, o => o.Value == record.TrackBy));
    }

    /// <summary>Executável, atalho ou URI (só leitura: trocar o arquivo é adicionar outro jogo).</summary>
    public string Target => _record.Path;

    public string TrackingLabel => $"Fim do jogo: {TrackingOptions[TrackingIndex].Label}";

    public bool HasError => Error is not null;

    [RelayCommand]
    private void CycleTracking() => TrackingIndex = (TrackingIndex + 1) % TrackingOptions.Length;

    [RelayCommand]
    private void Save()
    {
        string name = Name.Trim();
        string folder = WorkingDirectory.Trim();
        if (name.Length == 0)
        {
            Error = "Dê um nome ao jogo.";
            return;
        }

        if (folder.Length > 0 && !Directory.Exists(folder))
        {
            Error = $"Pasta de trabalho não encontrada: {folder}";
            return;
        }

        Error = null;
        _save(_record with
        {
            Title = name,
            Arguments = Arguments.Trim() is { Length: > 0 } args ? args : null,
            WorkingDirectory = folder.Length > 0 ? folder : DefaultFolder(),
            TrackBy = TrackingOptions[TrackingIndex].Value,
        });
    }

    [RelayCommand]
    private void Cancel() => _cancel();

    /// <summary>Campo vazio: a pasta do executável, como ao adicionar (URIs ficam sem pasta).</summary>
    private string? DefaultFolder() => _record.Path.Contains("://", StringComparison.Ordinal) ? null : System.IO.Path.GetDirectoryName(_record.Path);
}
