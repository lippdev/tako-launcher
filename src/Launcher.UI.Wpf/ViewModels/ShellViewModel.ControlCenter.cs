using System.ComponentModel;
using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using ConsoleMode.Runtime;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Central de Controle sobre o jogo real (MVP-026/030). Com o jogo em janela ou em tela cheia sem bordas, o atalho abre
/// a Central "ao vivo", como a sobreposição da Steam: a janela do Tako fica transparente e sempre no topo, cobrindo o
/// monitor do jogo, e o jogo continua visível e rodando atrás (<see cref="IsLiveOverlay"/>; a janela é tratada pela
/// MainWindow). Em tela cheia exclusiva uma janela por cima minimiza o jogo: aí o Tako vem para a frente com a Central
/// sobre a arte do jogo. Fechar devolve o foco ao jogo. Daqui saem também as ações que deixam o Tako para trás
/// (alternar janela, abrir o ControlFS) e o "Encerrar jogo" de um jogo real, que pede para ele fechar como o X da janela.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Fechar a Central sem devolver o foco ao jogo (outra janela ou a Início vão para a frente).</summary>
    private bool _keepGameBehind;

    /// <summary>A Central está aberta sobre o jogo ao vivo (janela transparente por cima do jogo).</summary>
    [ObservableProperty]
    private bool _isLiveOverlay;

    /// <summary>O atalho, com o Tako em segundo plano, pode abrir a Central: há um jogo real aberto e nada por cima.</summary>
    public bool CanOpenControlCenterFromBackground =>
        TopOverlay is null && _backgroundSession is null && IsTrackingRealGame && Session == SessionState.Playing;

    /// <summary>
    /// Atalho com o jogo na frente. <paramref name="live"/>: o jogo não está em tela cheia exclusiva, então a Central abre
    /// por cima dele sem a arte de fundo. Falso se não há o que abrir.
    /// </summary>
    public bool OpenControlCenterFromBackground(bool live)
    {
        if (!CanOpenControlCenterFromBackground || RunningGame is null)
        {
            return false;
        }

        OpenControlCenterOverRealGame(live);
        return true;
    }

    /// <summary>Central de Controle sobre um jogo real: ao vivo, ou com a arte do jogo no fundo.</summary>
    private async void OpenControlCenterOverRealGame(bool live = false)
    {
        if (RunningGame is not { } game)
        {
            return;
        }

        var session = new GameSessionViewModel(game, SessionOptions) { IsRealGame = true };
        if (live)
        {
            IsLiveOverlay = true; // a MainWindow já está transparente por cima do jogo
        }
        else
        {
            PushOverlay(session);
        }

        PushOverlay(new ControlCenterViewModel(this, session, SessionTools));
        session.Frame = await game.LoadHeroAsync().ConfigureAwait(true); // ao vivo, só para o cartão de captura
    }

    /// <summary>Depois de fechar a Central: o fundo do jogo real sai junto e, se for o caso, o jogo volta à frente.</summary>
    private void OnControlCenterClosed()
    {
        if (IsLiveOverlay)
        {
            IsLiveOverlay = false; // a janela some de cima do jogo antes de ele voltar ao primeiro plano
            if (!_keepGameBehind)
            {
                TryResumeRealGame();
            }

            return;
        }

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

    /// <summary>Abre o ControlFS por cima do jogo; o atalho da Central a traz de volta.</summary>
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
