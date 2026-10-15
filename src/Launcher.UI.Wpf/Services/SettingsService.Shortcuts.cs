using CommunityToolkit.Mvvm.ComponentModel;
using Launcher.Core.Input;
using Chord = Launcher.Core.Input.ControlCenterChord;

namespace Launcher.UI.Wpf.Services;

/// <summary>Atalhos da Central de Controle (UX-005): combinação do controle e atalho de teclado.</summary>
public sealed partial class SettingsService
{
    /// <summary>Botões segurados por meio segundo; padrão View + Y.</summary>
    [ObservableProperty]
    private GamepadButtons _controlCenterChord = Chord.Default;

    /// <summary>Atalho de teclado, também global durante um jogo; padrão Ctrl+Shift+T.</summary>
    [ObservableProperty]
    private KeyShortcut _controlCenterKey = KeyShortcut.Default;

    partial void OnControlCenterChordChanged(GamepadButtons value) => Save(nameof(ControlCenterChord), Chord.Serialize(value));

    partial void OnControlCenterKeyChanged(KeyShortcut value) => Save(nameof(ControlCenterKey), value.Serialize());

    private void LoadShortcuts(IReadOnlyDictionary<string, string> saved)
    {
        ControlCenterChord = Chord.Parse(saved.GetValueOrDefault(nameof(ControlCenterChord)));
        ControlCenterKey = KeyShortcut.Parse(saved.GetValueOrDefault(nameof(ControlCenterKey)));
    }
}
