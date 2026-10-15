using CommunityToolkit.Mvvm.Input;
using ConsoleMode.Runtime.Display;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Configurações › Tela de jogo (MVP-002). Cada linha troca de valor ao ser ativada (A/Enter/clique), o que funciona
/// igual no controle, no teclado e no mouse.
/// </summary>
public sealed partial class SettingsViewModel
{
    public string GameScreenLabel => Settings.GameScreenName.Length > 0 ? Settings.GameScreenName : "Escolher";

    public string GameScreenHideLabel => Settings.GameScreenHideOthers == HideOthersMode.BlackCurtain ? "Cobrir de preto" : "Desconectar";

    public string GameScreenModeLabel => DisplayMode.TryParse(Settings.GameScreenMode)?.ToString() ?? "Manter a atual";

    [RelayCommand]
    private void ToggleGameScreen()
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
    }

    [RelayCommand]
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

    [RelayCommand]
    private void CycleGameScreenHide()
    {
        Settings.GameScreenHideOthers = Settings.GameScreenHideOthers == HideOthersMode.Disconnect ? HideOthersMode.BlackCurtain : HideOthersMode.Disconnect;
        OnPropertyChanged(nameof(GameScreenHideLabel));
    }

    [RelayCommand]
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
            OnPropertyChanged(nameof(GameScreenModeLabel));
            Shell.Toast("Ligue a tela de jogo para ver as resoluções dela");
            return;
        }

        // "Manter a atual" e depois os modos, do maior para o menor.
        List<DisplayMode?> options = [null, .. modes];
        int current = options.FindIndex(m => m?.Key == DisplayMode.TryParse(Settings.GameScreenMode)?.Key);
        Settings.GameScreenMode = options[(current + 1) % options.Count]?.Key ?? string.Empty;
        OnPropertyChanged(nameof(GameScreenModeLabel));
    }

    [RelayCommand]
    private void ToggleGameScreenHdr() => Settings.GameScreenHdr = !Settings.GameScreenHdr;

    private void ChooseGameScreen(ConnectedDisplay display)
    {
        if (!string.Equals(Settings.GameScreenId, display.Id, StringComparison.OrdinalIgnoreCase))
        {
            Settings.GameScreenMode = string.Empty; // o modo é de cada tela
        }

        Settings.GameScreenId = display.Id;
        Settings.GameScreenName = display.Name;
        OnPropertyChanged(nameof(GameScreenLabel));
        OnPropertyChanged(nameof(GameScreenModeLabel));
    }
}
