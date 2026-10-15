using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.Display;
using Launcher.Core.Presentation;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Modo Console › Tela de jogo (MVP-002, ADR-0015). Cada linha troca de valor ao ser ativada (A/Enter/clique), o que
/// funciona igual no controle, no teclado e no mouse. As preferências continuam com as mesmas chaves no <c>tako.db</c>.
/// </summary>
public sealed partial class ConsoleModeViewModel
{
    public string GameScreenLabel => Settings.GameScreenName.Length > 0 ? Settings.GameScreenName : "Escolher";

    public string GameScreenHideLabel => Settings.GameScreenHideOthers == HideOthersMode.BlackCurtain ? "Cobrir de preto" : "Desconectar";

    public string GameScreenModeLabel => DisplayMode.TryParse(Settings.GameScreenMode)?.ToString() ?? "Manter a atual";

    public FeatureStatus GameScreenStatus => ConsoleFeatures.GameScreen(Settings.GameScreenEnabled, Settings.GameScreenName, _screenConnected);

    private IReadOnlyList<SettingRowViewModel> GameScreenRows() =>
    [
        new("console:gamescreen:on", SettingKind.Toggle, "Usar a tela de jogo", new RelayCommand(ToggleGameScreen),
            description: () => "Ao abrir um jogo, usa só a tela escolhida (ex.: a TV); tudo volta ao sair do jogo",
            isOn: () => Settings.GameScreenEnabled),
        new("console:gamescreen:display", SettingKind.Choice, "Tela onde você joga", new RelayCommand(CycleGameScreen),
            description: () => "A muda de tela. Na primeira vez com uma tela nova, o Tako pergunta se você está vendo a imagem",
            value: () => GameScreenLabel,
            visible: () => Settings.GameScreenEnabled),
        new("console:gamescreen:hide", SettingKind.Choice, "Outras telas durante o jogo", new RelayCommand(CycleGameScreenHide),
            description: () => Settings.GameScreenHideOthers == HideOthersMode.BlackCurtain
                ? "Continuam ligadas, só com a imagem apagada"
                : "Saem da área de trabalho e voltam ao fim do jogo",
            value: () => GameScreenHideLabel,
            visible: () => Settings.GameScreenEnabled),
        new("console:gamescreen:mode", SettingKind.Choice, "Resolução e taxa", new RelayCommand(CycleGameScreenMode),
            description: () => "Valem na tela de jogo só durante o jogo",
            value: () => GameScreenModeLabel,
            visible: () => Settings.GameScreenEnabled),
        new("console:gamescreen:hdr", SettingKind.Toggle, "Ligar o HDR", new RelayCommand(ToggleGameScreenHdr),
            description: () => "Liga o HDR na tela de jogo (se ela tiver) e desliga ao sair",
            isOn: () => Settings.GameScreenHdr,
            visible: () => Settings.GameScreenEnabled),

        new("console:gamescreen:onlaunch", SettingKind.Toggle, "Ao abrir o Tako, ir para a tela de jogo", new RelayCommand(ToggleConsoleModeOnLaunch),
            description: () => ConsoleModeOnLaunchDescription,
            isOn: () => Settings.ConsoleModeOnLaunch,
            visible: () => Settings.GameScreenEnabled),
        new("console:gamescreen:test", SettingKind.Action, "Testar a imagem agora", new AsyncRelayCommand(TestGameScreenAsync),
            description: () => "Mostra o Tako na tela de jogo por alguns segundos e pergunta se dá para ver",
            value: () => "Testar",
            visible: () => Settings.GameScreenEnabled && Settings.GameScreenId.Length > 0),
    ];

    public void ToggleGameScreen()
    {
        Settings.GameScreenEnabled = !Settings.GameScreenEnabled;
        if (Settings.GameScreenEnabled && Settings.GameScreenId.Length == 0)
        {
            // Primeira vez: sugere uma tela que não é a principal (normalmente a TV).
            var displays = Shell.GameScreen.ListDisplays();
            if ((displays.FirstOrDefault(d => !d.IsPrimary) ?? (displays.Count > 0 ? displays[0] : null)) is { } suggestion)
            {
                ChooseGameScreen(suggestion);
            }
        }

        Probe();
    }

    private void CycleGameScreen()
    {
        var displays = Shell.GameScreen.ListDisplays();
        if (displays.Count == 0)
        {
            Shell.Toast("Nenhuma tela encontrada");
            return;
        }

        int current = displays.ToList().FindIndex(d => string.Equals(d.Id, Settings.GameScreenId, StringComparison.OrdinalIgnoreCase));
        ChooseGameScreen(displays[(current + 1) % displays.Count]);
        if (displays.Count == 1)
        {
            Shell.Toast("Só há uma tela conectada");
        }
    }

    private void CycleGameScreenHide() =>
        Settings.GameScreenHideOthers = Settings.GameScreenHideOthers == HideOthersMode.Disconnect ? HideOthersMode.BlackCurtain : HideOthersMode.Disconnect;

    private void CycleGameScreenMode()
    {
        if (Settings.GameScreenId.Length == 0)
        {
            Shell.Toast("Escolha a tela de jogo primeiro");
            return;
        }

        var modes = Shell.GameScreen.ListModes(Settings.GameScreenId);
        if (modes.Count == 0)
        {
            Settings.GameScreenMode = string.Empty;
            Shell.Toast("Ligue a tela de jogo para ver as resoluções dela");
            return;
        }

        // "Manter a atual" e depois os modos, do maior para o menor.
        List<DisplayMode?> options = [null, .. modes];
        int current = options.FindIndex(m => m?.Key == DisplayMode.TryParse(Settings.GameScreenMode)?.Key);
        Settings.GameScreenMode = options[(current + 1) % options.Count]?.Key ?? string.Empty;
    }

    private void ToggleGameScreenHdr() => Settings.GameScreenHdr = !Settings.GameScreenHdr;

    /// <summary>Modo console (#42): só volta para a mesa ao sair do Tako ou em "Voltar ao PC"; o atalho de emergência fica à vista.</summary>
    public string ConsoleModeOnLaunchDescription =>
        $"Como o Console Mode: a mesa só volta ao sair do Tako ou em \"Voltar ao PC\" · emergência: {Shell.EmergencyKeys}";

    private void ToggleConsoleModeOnLaunch()
    {
        Settings.ConsoleModeOnLaunch = !Settings.ConsoleModeOnLaunch;
        Shell.Toast(Settings.ConsoleModeOnLaunch
            ? $"Na próxima abertura, o Tako vai direto para {GameScreenLabel} · {Shell.EmergencyKeys} volta as telas para a mesa"
            : "O Tako abre na mesa; a tela de jogo vale só durante os jogos");
    }

    private async Task TestGameScreenAsync()
    {
        var (seen, problem) = await Shell.TestGameScreenAsync().ConfigureAwait(true);
        Shell.Toast(seen ? $"{GameScreenLabel} confirmada · os jogos abrem nela sem perguntar" : $"Tela de jogo: {problem}");
        Probe();
    }

    private void ChooseGameScreen(ConnectedDisplay display)
    {
        if (!string.Equals(Settings.GameScreenId, display.Id, StringComparison.OrdinalIgnoreCase))
        {
            Settings.GameScreenMode = string.Empty; // o modo é de cada tela
        }

        Settings.GameScreenId = display.Id;
        Settings.GameScreenName = display.Name;
        Probe();
    }
}
