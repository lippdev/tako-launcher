using System.ComponentModel;
using System.Diagnostics;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Central de Controle sobre o jogo real (MVP-026/030): o Guia traz o Tako para a frente com a Central aberta sobre a
/// arte do jogo; fechá-la devolve o jogo. Daqui saem também as ações que deixam o Tako para trás (alternar janela,
/// abrir o ControlFS) e o "Encerrar jogo" de um jogo real, que pede para ele fechar como o X da janela.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Fechar a Central sem devolver o foco ao jogo (outra janela ou a Início vão para a frente).</summary>
    private bool _keepGameBehind;

    /// <summary>O Guia, com o Tako em segundo plano, pode abrir a Central: há um jogo real aberto e nada por cima.</summary>
    public bool CanOpenControlCenterFromBackground =>
        TopOverlay is null && _backgroundSession is null && IsTrackingRealGame && Session == SessionState.Playing;

    /// <summary>Central de Controle sobre um jogo real: a arte do jogo fica no fundo enquanto ela está aberta.</summary>
    private async void OpenControlCenterOverRealGame()
    {
        if (RunningGame is not { } game)
        {
            return;
        }

        var session = new GameSessionViewModel(game, SessionOptions) { IsRealGame = true };
        PushOverlay(session);
        PushOverlay(new ControlCenterViewModel(this, session, SessionTools));
        session.Frame = await game.LoadHeroAsync().ConfigureAwait(true);
    }

    /// <summary>Depois de fechar a Central: o fundo do jogo real sai junto e, se for o caso, o jogo volta à frente.</summary>
    private void OnControlCenterClosed()
    {
        if (TopOverlay is not GameSessionViewModel { IsRealGame: true } real)
        {
            return;
        }

        Overlays.Remove(real);
        if (!_keepGameBehind)
        {
            TryResumeRealGame();
        }
    }

    /// <summary>A numa janela aberta: fecha a Central e traz aquela janela para a frente; o jogo continua aberto.</summary>
    public void SwitchToWindow(ControlCenterViewModel center, OpenWindow window)
    {
        ArgumentNullException.ThrowIfNull(window);
        CloseControlCenterKeepingGameBehind(center);
        if (!SessionTools.Windows.Activate(window.Handle))
        {
            Toast($"{(string.IsNullOrEmpty(window.ProcessName) ? "A janela" : window.ProcessName)} não está mais aberta");
        }
    }

    /// <summary>Abre o ControlFS por cima do jogo; o Guia traz a Central de volta.</summary>
    public void OpenControlFs(ControlCenterViewModel center)
    {
        CloseControlCenterKeepingGameBehind(center);
        if (!SessionTools.ControlFs.Open())
        {
            Toast("O ControlFS não abriu · veja se ele ainda está instalado");
        }
    }

    /// <summary>"Encerrar jogo": o simulado termina na hora; o real recebe o pedido de fechar, como o X da janela.</summary>
    public void EndSessionFromControlCenter(ControlCenterViewModel center)
    {
        ArgumentNullException.ThrowIfNull(center);
        if (!center.IsRealGame)
        {
            EndSession();
            return;
        }

        var game = RunningGame;
        int asked = 0;
        foreach (int id in _monitor?.Tracker?.GameProcessIds ?? [])
        {
            try
            {
                using var process = Process.GetProcessById(id);
                if (process.MainWindowHandle != 0 && SessionTools.Windows.RequestClose(process.MainWindowHandle))
                {
                    asked++;
                }
            }
            catch (Exception ex) when (ex is ArgumentException or InvalidOperationException or Win32Exception)
            {
                // O processo já fechou: nada a pedir.
            }
        }

        LogRealGameCloseRequested(_logger, game?.Id ?? string.Empty, asked);

        // O jogo volta à frente: se ele perguntar se quer salvar, a resposta é dada nele. O fim da sessão vem do
        // acompanhamento do processo, como quando o jogo fecha sozinho.
        CloseOverlay(center);
        if (game is not null)
        {
            Toast(asked > 0 ? $"Pedimos para {game.Title} fechar · se ele perguntar, responda no jogo" : $"{game.Title} não tem janela para fechar · saia pelo menu do jogo");
        }
    }

    /// <summary>O jogo real fechou com a Central aberta por cima: ela e o fundo do jogo saem de cena.</summary>
    private void CloseRealGameOverlays()
    {
        foreach (var center in Overlays.OfType<ControlCenterViewModel>().Where(c => c.IsRealGame).ToList())
        {
            CloseControlCenterKeepingGameBehind(center);
        }

        foreach (var real in Overlays.OfType<GameSessionViewModel>().Where(s => s.IsRealGame).ToList())
        {
            Overlays.Remove(real);
        }
    }

    private void CloseControlCenterKeepingGameBehind(ControlCenterViewModel center)
    {
        _keepGameBehind = true;
        try
        {
            CloseOverlay(center);
        }
        finally
        {
            _keepGameBehind = false;
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Encerrar jogo {GameId} pela Central de Controle: pedido de fechar enviado a {Windows} janela(s)")]
    private static partial void LogRealGameCloseRequested(ILogger logger, string gameId, int windows);
}
