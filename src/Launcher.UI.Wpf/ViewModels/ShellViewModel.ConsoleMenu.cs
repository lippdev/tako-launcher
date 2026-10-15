using Launcher.Core.Session;
using Launcher.Core.Session.GameScreen;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Menu Modo Console (ADR-0020): a página dos cartões, o detalhe de cada recurso e o teste da tela de jogo usado pelo
/// assistente "Configurar para jogar na TV".
/// </summary>
public sealed partial class ShellViewModel
{
    public void OpenConsoleMode() => Navigate(new ConsoleModeViewModel(this), replace: CurrentPage is ConsoleModeViewModel);

    /// <summary>Detalhe de um recurso: B volta aos cartões, com o foco no cartão de origem.</summary>
    public void OpenConsoleFeature(SettingsDetailViewModel detail) => Navigate(detail, replace: CurrentPage is SettingsDetailViewModel);

    /// <summary>
    /// Mostra o Tako na tela de jogo por alguns segundos, pergunta "Você consegue ver esta tela?" e volta as telas
    /// como estavam. Confirmada, a combinação tela + modo não é perguntada de novo no primeiro jogo.
    /// </summary>
    /// <returns>Se a imagem foi confirmada e, se não, o motivo em linguagem de usuário.</returns>
    public async Task<(bool Seen, string? Problem)> TestGameScreenAsync()
    {
        EnsureUiContext(); // as continuações voltam para a thread da UI (sobreposições, janela)
        if (Settings.GameScreen is not { } settings)
        {
            return (false, "Escolha a tela de jogo primeiro");
        }

        if (RunningGame is not null || GameScreen.IsActive)
        {
            return (false, "A tela de jogo já está em uso; feche o jogo antes de testar");
        }

        var result = await Task.Run(() => GameScreen.Begin(settings, confirmedSetup: null)).ConfigureAwait(true);
        if (result.Outcome != GameScreenOutcome.Applied)
        {
            return (false, Capitalize(result.Reason ?? "não foi possível usar a tela"));
        }

        bool seen = false;
        try
        {
            if (result.TargetBounds is { } bounds)
            {
                MoveLauncherTo(bounds);
            }

            seen = await ConfirmGameScreenAsync(result.Target?.Name ?? Settings.GameScreenName).ConfigureAwait(true);
            if (seen)
            {
                Settings.GameScreenConfirmedSetup = GameScreenConfirmations.Remember(Settings.GameScreenConfirmedSetup, settings.SetupKey);
            }
        }
        finally
        {
            // Sempre devolve as telas, fora da thread da UI e com "Voltando para a mesa…", como no fim do jogo.
            await RunOffUiAsync(ReturningToDesk, ReturningDetail, EndGameScreen).ConfigureAwait(true);
            if (GameScreen.ListDisplays().FirstOrDefault(d => d.IsPrimary && d.IsActive)?.Bounds is { } home)
            {
                MoveLauncherTo(home); // o Tako volta para a tela principal junto com as outras telas
            }
        }

        return (seen, seen ? null : "Você não confirmou a imagem; as telas voltaram ao normal");
    }

    private static string Capitalize(string text) => text.Length == 0 ? text : char.ToUpperInvariant(text[0]) + text[1..];
}
