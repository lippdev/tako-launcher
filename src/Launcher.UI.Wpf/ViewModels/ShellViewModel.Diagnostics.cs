using System.IO;
using System.Runtime.InteropServices;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Diagnostics;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Diagnóstico do sistema (frente G, MVP-034): relatório sem dados pessoais para anexar ao checklist de hardware.
/// Fica salvo na pasta de logs e pode ser copiado.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Gera o relatório, salva em <c>logs/diagnostico-*.txt</c> e mostra na tela com "Copiar".</summary>
    [RelayCommand]
    public void ShowSystemDiagnostics()
    {
        var snapshot = SystemDiagnostics.Collect(
            new DiagnosticsContext([.. Library.Games], ControllerName, IsSafeMode, IsSafeMode ? SafeModeDescription : null, FseStatus, ThemeDisplayName()),
            _time);
        string report = SystemDiagnostics.Format(snapshot);
        string? savedTo = SaveDiagnostics(report, snapshot.GeneratedAt);

        DiagnosticsViewModel? view = null;
        view = new DiagnosticsViewModel(report, savedTo is null ? "Não foi possível salvar na pasta de logs" : $"Salvo em logs\\{Path.GetFileName(savedTo)}",
            [
                new OverlayAction("Copiar", ActionStyle.Primary, new RelayCommand(() => Toast(CopyToClipboard(report) ? "Diagnóstico copiado: cole na PR ou issue do checklist" : "Não foi possível copiar; use o arquivo na pasta de logs")), IsDefault: true),
                new OverlayAction("Abrir pasta de logs", ActionStyle.Secondary, new RelayCommand(OpenLogsFolder)),
                new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(view!))),
            ]);
        PushOverlay(view);
    }

    private string ThemeDisplayName() => _themes.CurrentTheme == ThemeService.DefaultTheme ? "Padrão" : _themes.CurrentTheme;

    private string? SaveDiagnostics(string report, DateTimeOffset generatedAt)
    {
        try
        {
            Directory.CreateDirectory(AppPaths.LogsDirectory);
            string file = SystemReport.FileName(generatedAt);
            string path = Path.Combine(AppPaths.LogsDirectory, file);
            File.WriteAllText(path, report.Replace("\n", Environment.NewLine, StringComparison.Ordinal));
            LogDiagnosticsSaved(_logger, file);
            return path;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LogDiagnosticsNotSaved(_logger, ex.Message);
            return null;
        }
    }

    private static bool CopyToClipboard(string text)
    {
        try
        {
            System.Windows.Clipboard.SetText(text.Replace("\n", Environment.NewLine, StringComparison.Ordinal));
            return true;
        }
        catch (ExternalException)
        {
            return false; // área de transferência ocupada por outro programa
        }
    }

    private void OpenLogsFolder()
    {
        try
        {
            Directory.CreateDirectory(AppPaths.LogsDirectory);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(AppPaths.LogsDirectory) { UseShellExecute = true })?.Dispose();
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or IOException or UnauthorizedAccessException)
        {
            Toast($"Não foi possível abrir a pasta de logs: {ex.Message}");
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Diagnóstico do sistema salvo em {File}")]
    private static partial void LogDiagnosticsSaved(ILogger logger, string file);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Falha ao salvar o diagnóstico do sistema: {Reason}")]
    private static partial void LogDiagnosticsNotSaved(ILogger logger, string reason);
}

/// <summary>Relatório do diagnóstico na tela (texto monoespaçado com rolagem) e as ações.</summary>
public sealed class DiagnosticsViewModel(string report, string status, IReadOnlyList<OverlayAction> actions) : IOverlayViewModel
{
    public string Report { get; } = report;

    public string Status { get; } = status;

    public IReadOnlyList<OverlayAction> Actions { get; } = actions;
}
