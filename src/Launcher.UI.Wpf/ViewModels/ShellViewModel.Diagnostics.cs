using System.IO;
using System.Runtime.InteropServices;
using CommunityToolkit.Mvvm.Input;
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using Launcher.Core.Diagnostics;
using Launcher.Core.Input;
using Launcher.UI.Wpf.Services;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Diagnóstico do sistema (frente G, MVP-034): relatório sem dados pessoais para anexar ao checklist de hardware.
/// Fica salvo na pasta de logs e pode ser copiado.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>
    /// Controles lidos agora (HID e XInput) com estado e relatórios crus; a janela liga à leitura real. Vazio sem janela
    /// (testes de view model).
    /// </summary>
    public Func<IReadOnlyList<ControllerProbe>> ReadControllers { get; set; } = () => [];

    /// <summary>Gera o relatório, salva em <c>logs/diagnostico-*.txt</c> e mostra na tela com "Copiar".</summary>
    [RelayCommand]
    public void ShowSystemDiagnostics()
    {
        var snapshot = SystemDiagnostics.Collect(
            new DiagnosticsContext([.. Library.Games], ControllerName, IsSafeMode, IsSafeMode ? SafeModeDescription : null, FseStatus, ThemeDisplayName(), ControllerTestLines()),
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

    /// <summary>
    /// Teste de controle (Configurações): cada controle detectado com fonte, modelo, USB/Bluetooth e product id, os
    /// botões, o direcional e os analógicos ao vivo, e os últimos relatórios HID crus. "Copiar" leva tudo para a issue.
    /// </summary>
    [RelayCommand]
    public void ShowControllerTest()
    {
        ControllerTestViewModel? view = null;
        view = new ControllerTestViewModel(
            ControllerTestLines,
            [
                new OverlayAction("Copiar", ActionStyle.Primary, new RelayCommand(() => Toast(CopyToClipboard(view!.Text) ? "Teste de controle copiado: cole na issue" : "Não foi possível copiar")), IsDefault: true),
                new OverlayAction("Fechar", ActionStyle.Secondary, new RelayCommand(() => CloseOverlay(view!))),
            ]);
        PushOverlay(view);
    }

    private IReadOnlyList<string> ControllerTestLines() => ControllerTest.Lines(ReadControllers(), SystemDiagnostics.IsSteamRunning());

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

/// <summary>
/// Teste de controle ao vivo: relê os controles 10 vezes por segundo. Os botões continuam navegando (só entre "Copiar"
/// e "Fechar"); ○/B (ou Esc) precisa ser apertado duas vezes para sair, para dar para testá-lo também.
/// </summary>
public sealed partial class ControllerTestViewModel : ObservableObject, IOverlayViewModel, IDisposable
{
    public const string BackHint = "Aperte ○/B (ou Esc) de novo para sair";
    private const long BackConfirmMilliseconds = 3000;

    private readonly Func<IReadOnlyList<string>> _read;
    private readonly DispatcherTimer _timer = new(DispatcherPriority.Background) { Interval = TimeSpan.FromMilliseconds(100) };
    private long? _backPressedAt;

    [ObservableProperty]
    private string _text = string.Empty;

    [ObservableProperty]
    private string _hint = "Aperte e segure cada botão: a linha do controle mostra o que o Tako recebe.";

    public ControllerTestViewModel(Func<IReadOnlyList<string>> read, IReadOnlyList<OverlayAction> actions)
    {
        _read = read;
        Actions = actions;
        Refresh();
        _timer.Tick += (_, _) => Refresh();
        _timer.Start();
    }

    public IReadOnlyList<OverlayAction> Actions { get; }

    public bool HandleBack()
    {
        long now = Environment.TickCount64;
        if (_backPressedAt is long before && now - before <= BackConfirmMilliseconds)
        {
            return false; // segundo toque: a Shell fecha
        }

        _backPressedAt = now;
        Hint = BackHint;
        return true;
    }

    public void Refresh() => Text = string.Join('\n', _read());

    public void Dispose() => _timer.Stop();
}
