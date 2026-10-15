using System.Windows;
using System.Windows.Interop;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Windows;
using Launcher.Core.Session;
using Launcher.Core.Session.GameScreen;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// "Tela de jogo" (MVP-002, como o Console Mode): antes de abrir o jogo real, foca a tela escolhida em Configurações e,
/// na primeira vez com uma tela ou modo novo, pergunta se dá para ver a imagem. A lógica fica no
/// <see cref="GameScreenSession"/> (Core); aqui só a ligação com a sessão e os avisos.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Tempo para responder "Você consegue ver esta tela?" antes de as telas voltarem sozinhas.</summary>
    public const int ScreenConfirmSeconds = 15;

    public GameScreenSession GameScreen { get; }

    /// <summary>
    /// Foca a tela de jogo (se ligada para este jogo, UX-007). Falso: o usuário não confirmou e as telas já voltaram;
    /// não abra o jogo.
    /// </summary>
    private async Task<bool> FocusGameScreenAsync(EffectiveSessionSettings effective)
    {
        if (effective.GameScreenProblem is { } problem)
        {
            Toast($"Tela de jogo: {problem} · o jogo abre na tela atual");
        }

        if (effective.GameScreen is not { } settings)
        {
            return true;
        }

        // Cada resolução já confirmada vale (um jogo pode pedir outra); o formato esperado é a combinação ou vazio.
        string confirmed = GameScreenConfirmations.Matching(Settings.GameScreenConfirmedSetup, settings.SetupKey);
        var result = await Task.Run(() => GameScreen.Begin(settings, confirmed)).ConfigureAwait(true);
        switch (result.Outcome)
        {
            case GameScreenOutcome.Skipped:
                Toast($"Tela de jogo: {result.Reason} · o jogo abre na tela atual");
                return true;
            case GameScreenOutcome.Reverted:
                Toast($"Tela de jogo: {result.Reason} · as telas voltaram como estavam");
                return true;
        }

        if (result.TargetBounds is { } bounds)
        {
            MoveLauncherTo(bounds);
        }

        if (!result.NeedsConfirmation)
        {
            return true;
        }

        if (await ConfirmGameScreenAsync(result.Target?.Name ?? Settings.GameScreenName).ConfigureAwait(true))
        {
            Settings.GameScreenConfirmedSetup = GameScreenConfirmations.Remember(Settings.GameScreenConfirmedSetup, settings.SetupKey);
            return true;
        }

        RestoreGameScreen();
        Toast("Tela de jogo não confirmada · suas telas voltaram ao normal");
        return false;
    }

    private async Task<bool> ConfirmGameScreenAsync(string screenName)
    {
        var confirm = new ScreenConfirmViewModel(screenName, ScreenConfirmSeconds);
        PushOverlay(confirm);
        bool seen = await confirm.Result.ConfigureAwait(true);
        if (Overlays.Contains(confirm))
        {
            CloseOverlay(confirm);
        }

        return seen;
    }

    /// <summary>Desfaz a tela de jogo, se estiver em foco. Devolve o complemento do aviso de "bem-vindo".</summary>
    private string RestoreGameScreen()
    {
        if (!GameScreen.IsActive)
        {
            return string.Empty;
        }

        return GameScreen.End() ? string.Empty : " · as telas não voltaram exatamente como antes (veja os logs)";
    }

    /// <summary>O Tako vai para a tela de jogo (com a cortina preta, ele poderia ficar coberto na tela antiga).</summary>
    private static void MoveLauncherTo(ScreenBounds bounds)
    {
        if (Application.Current?.MainWindow is not { } window)
        {
            return;
        }

        var state = window.WindowState;
        window.WindowState = WindowState.Normal;
        WindowPlacement.MoveTo(new WindowInteropHelper(window).Handle, bounds);
        window.WindowState = state == WindowState.Minimized ? WindowState.Maximized : state;
    }
}
