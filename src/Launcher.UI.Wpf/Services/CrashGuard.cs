using System.Diagnostics;
using System.Windows;
using System.Windows.Markup;
using System.Windows.Threading;
using Launcher.Core.Resilience;
using Launcher.UI.Wpf.ViewModels;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.Services;

/// <summary>
/// Rede de segurança da thread da UI (MVP-011/036). Um erro isolado vira a tela de recuperação, que sempre tem um caminho
/// para o desktop; vários seguidos (<see cref="FaultBudget"/>) fecham a janela de tela cheia e encerram o Tako, devolvendo
/// o desktop em vez de deixar uma tela preta presa. Tudo vai para o log de diagnóstico (MVP-033). Exceções fatais em
/// outras threads são registradas em <see cref="App"/>; o processo cai, e a próxima abertura conta a saída suja para o
/// modo seguro.
/// </summary>
public sealed partial class CrashGuard(ILogger<CrashGuard> logger)
{
    private readonly FaultBudget _budget = new();
    private readonly Stopwatch _clock = Stopwatch.StartNew();
    private Window? _window;
    private ShellViewModel? _shell;

    /// <summary>O Tako está fechando por falha: o fechamento não conta como limpo.</summary>
    public bool IsFatal { get; private set; }

    /// <summary>O que foi feito com o último erro (diagnóstico do teste de fumaça).</summary>
    internal string LastOutcome { get; private set; } = "nenhum";

    /// <summary>Erros tratados com a tela de recuperação nesta sessão.</summary>
    internal int Recoveries { get; private set; }

    /// <summary>
    /// Instala o tratador. Chamar depois de criar a janela: ele roda depois do tratador da janela (erros de tema), que
    /// tem prioridade.
    /// </summary>
    public void Install(Application app, Window window, ShellViewModel shell)
    {
        ArgumentNullException.ThrowIfNull(app);
        _window = window;
        _shell = shell;
        app.Dispatcher.UnhandledException += OnDispatcherUnhandledException;
    }

    private void OnDispatcherUnhandledException(object sender, DispatcherUnhandledExceptionEventArgs e)
    {
        // Erro de XAML de tema é da janela (fecha a prévia e mostra a recuperação do tema). Não dá para confiar em
        // e.Handled: outro assinante do dispatcher pode marcá-lo sem mostrar nada ao usuário.
        if (e.Exception is XamlParseException)
        {
            LastOutcome = "tema";
            return;
        }

        e.Handled = true; // o registro no log de diagnóstico é feito pela janela (MVP-033)
        if (IsFatal)
        {
            return;
        }

        if (_budget.TryRecord(_clock.Elapsed) && _window is { IsLoaded: true } && _shell is not null)
        {
            try
            {
                _shell.ShowErrorRecovery(e.Exception);
                Recoveries++;
                LastOutcome = $"recuperação ({_shell.Overlays.Count} sobreposições)";
                return;
            }
            catch (Exception ex) when (ex is not OutOfMemoryException)
            {
                // A própria tela de recuperação falhou: não há UI confiável, fecha a tela cheia.
                LogRecoveryFailed(logger, ex);
                LastOutcome = "recuperação falhou: " + ex;
            }
        }

        ReturnToDesktop();
    }

    /// <summary>Fecha a janela de tela cheia e encerra o Tako; o Windows volta ao desktop.</summary>
    private void ReturnToDesktop()
    {
        LastOutcome = "desktop";
        IsFatal = true;
        LogReturningToDesktop(logger);
        try
        {
            _window?.Hide();
            _window?.Close();
        }
        catch (Exception ex) when (ex is InvalidOperationException or System.ComponentModel.Win32Exception)
        {
            // A janela já está fechando; o Shutdown abaixo encerra mesmo assim.
        }

        Application.Current?.Shutdown(1);
    }

    [LoggerMessage(Level = LogLevel.Error, Message = "A tela de recuperação falhou")]
    private static partial void LogRecoveryFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Critical, Message = "Erros seguidos na UI: fechando a tela cheia e voltando ao desktop")]
    private static partial void LogReturningToDesktop(ILogger logger);
}
