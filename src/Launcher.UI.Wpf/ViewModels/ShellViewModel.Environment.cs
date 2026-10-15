using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Session.Restore;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Monitores e áudio da sessão (frente E2, MVP-012): registra antes do jogo real, restaura só o que mudou no estado
/// Restoring e, se o Tako caiu durante um jogo, oferece a restauração na próxima abertura. A lógica fica no
/// <see cref="SessionEnvironmentGuard"/> (Core); aqui só a ligação com a sessão e os avisos.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Antes de abrir o jogo real: grava o estado atual (se a opção estiver ligada).</summary>
    private void CaptureEnvironmentBeforeGame(string gameId) =>
        _environment.BeginSession(gameId, Settings.RestoreEnvironmentAfterGame);

    /// <summary>Estado Restoring: devolve monitores e áudio; o texto volta para o aviso de "bem-vindo".</summary>
    private string RestoreEnvironmentAfterGame()
    {
        var report = _environment.EndSession();
        return report switch
        {
            { NothingToDo: true } => string.Empty,
            { Succeeded: true } => " · monitores e áudio restaurados",
            _ => " · não deu para restaurar tudo (veja os logs)",
        };
    }

    /// <summary>
    /// Na abertura: se a sessão anterior terminou no meio de um jogo e monitores ou áudio continuam diferentes,
    /// pergunta se deve restaurar. Nada muda sem a confirmação. Devolve se a pergunta foi feita.
    /// </summary>
    public bool OfferEnvironmentRestore()
    {
        if (_environment.PendingFromPreviousRun() is not { } pending)
        {
            return false;
        }

        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(
            "Restaurar monitores e áudio?",
            $"O Tako fechou durante um jogo ({pending.CapturedAt.ToLocalTime():dd/MM HH:mm}) e monitores ou áudio continuam diferentes de antes.",
            [
                new OverlayAction("Restaurar monitores e áudio como estavam antes do jogo", ActionStyle.Primary, new AsyncRelayCommand(async () =>
                {
                    CloseOverlay(dialog!);
                    // Fora da thread da UI: o Windows pode levar alguns segundos para trocar o modo das telas.
                    var report = await RunOffUiAsync("Restaurando monitores e áudio…", "Voltando ao que era antes do jogo", () => _environment.RestoreFrom(pending)).ConfigureAwait(true);
                    Toast(report.Succeeded ? "Monitores e áudio restaurados" : "Não deu para restaurar tudo · detalhes nos logs");
                }), IsDefault: true),
                new OverlayAction("Manter como está", ActionStyle.Secondary, new RelayCommand(() =>
                {
                    CloseOverlay(dialog!);
                    _environment.Discard();
                })),
            ]);
        PushOverlay(dialog);
        return true;
    }
}
