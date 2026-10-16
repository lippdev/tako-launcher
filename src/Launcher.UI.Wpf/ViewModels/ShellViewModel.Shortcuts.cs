using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Input;
using Launcher.UI.Wpf.Input;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// Atalhos da Central de Controle (UX-005): combinação segurada no controle (padrão View + Y; Create/Share + △ no
/// PlayStation) e atalho de teclado (padrão Ctrl+Shift+Tab), escolhidos em Configurações com telas de captura. As regras
/// ficam em <see cref="ShortcutRules"/>: nenhuma combinação serve a duas ações do Tako.
/// </summary>
public sealed partial class ShellViewModel
{
    /// <summary>Família do controle conectado, para os nomes dos botões (Xbox sem controle).</summary>
    public GamepadFamily PadFamily => ControllerName switch
    {
        string name when name.Contains("PS5", StringComparison.Ordinal) => GamepadFamily.PlayStation5,
        string name when name.Contains("PS4", StringComparison.Ordinal) => GamepadFamily.PlayStation4,
        _ => GamepadFamily.Xbox,
    };

    /// <summary>"View + Y" (ou "Create + △" com um DualSense).</summary>
    public string ChordLabel => ControlCenterChord.Format(Settings.ControlCenterChord, PadFamily);

    /// <summary>"Ctrl+Shift+Tab".</summary>
    public string KeyLabel => Settings.ControlCenterKey.Display;

    /// <summary>Como abrir a Central no dispositivo em uso: "segure View + Y" ou "Ctrl+Shift+Tab".</summary>
    public string GuideName => InputKind == InputKind.Gamepad ? $"segure {ChordLabel}" : KeyLabel;

    /// <summary>Texto das linhas de atalho nas Configurações.</summary>
    public string ShortcutDefaultsLabel =>
        $"{ControlCenterChord.Format(ControlCenterChord.Default, PadFamily)} no controle e {KeyShortcut.Default.Display} no teclado";

    /// <summary>Glifos da dica "Fechar" da Central: os botões da combinação ou o atalho de teclado.</summary>
    private List<string> GuideGlyphs(bool keyboard) =>
        keyboard ? [KeyLabel] : [.. ControlCenterChord.Names(Settings.ControlCenterChord, PadFamily)];

    /// <summary>Configurações › atalho do controle: tela "Segure os botões que quer usar".</summary>
    [RelayCommand]
    private void CaptureChord() => PushOverlay(new ShortcutCaptureViewModel(this, ShortcutKind.Gamepad));

    /// <summary>Configurações › atalho do teclado: captura de tecla com modificadores.</summary>
    [RelayCommand]
    private void CaptureKey() => PushOverlay(new ShortcutCaptureViewModel(this, ShortcutKind.Keyboard));

    [RelayCommand]
    private void ResetShortcuts()
    {
        Settings.ControlCenterChord = ControlCenterChord.Default;
        Settings.ControlCenterKey = KeyShortcut.Default;
        OnShortcutsChanged();
        Toast($"Atalhos padrão: {ShortcutDefaultsLabel}");
    }

    /// <summary>Valida e guarda a combinação do controle; com aviso, guarda e explica.</summary>
    internal ShortcutCheck SaveChord(GamepadButtons chord)
    {
        var check = ShortcutRules.CheckChord(chord, PadFamily);
        if (check.IsValid)
        {
            Settings.ControlCenterChord = chord & ControlCenterChord.Known;
            OnShortcutsChanged();
            Toast(check.Warning is null ? $"Central de Controle: segure {ChordLabel}" : $"Salvo · {check.Warning}");
        }

        return check;
    }

    internal ShortcutCheck SaveKey(KeyShortcut key)
    {
        // Outros atalhos globais do Tako: o de emergência do modo console volta as telas para a mesa.
        var takoShortcuts = new Dictionary<string, string>(StringComparer.Ordinal) { [EmergencyKeys] = "voltar as telas para a mesa" };
        var check = ShortcutRules.CheckKeyboard(key, takoShortcuts);
        if (check.IsValid)
        {
            Settings.ControlCenterKey = key;
            OnShortcutsChanged();
            Toast(check.Warning is null ? $"Central de Controle: {KeyLabel}" : $"Salvo · {check.Warning}");
        }

        return check;
    }

    private void OnShortcutsChanged()
    {
        OnPropertyChanged(nameof(ChordLabel));
        OnPropertyChanged(nameof(KeyLabel));
        OnPropertyChanged(nameof(GuideName));
        OnPropertyChanged(nameof(ShortcutDefaultsLabel));
        UpdateHints();
    }

    partial void OnControllerNameChanged(string? value) => OnShortcutsChanged();
}
