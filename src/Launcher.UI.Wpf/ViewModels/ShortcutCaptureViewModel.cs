using System.Windows.Input;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Launcher.Core.Input;

namespace Launcher.UI.Wpf.ViewModels;

public enum ShortcutKind
{
    Gamepad,
    Keyboard,
}

/// <summary>
/// Tela de captura do atalho da Central (UX-005). Controle: "Segure os botões que quer usar" — vale a combinação que
/// ficar igual por 1 s; tocar B/○ cancela. Teclado: a primeira tecla com Ctrl ou Alt; Esc cancela. Uma combinação
/// recusada pelas regras mostra o motivo e a captura recomeça. Enquanto ela está aberta, o controle e o teclado só
/// alimentam esta tela (MainWindow).
/// </summary>
public sealed partial class ShortcutCaptureViewModel : ObservableObject, IOverlayViewModel
{
    private readonly ShellViewModel _shell;
    private readonly ChordCapture _capture = new();

    [ObservableProperty]
    private string _current;

    [ObservableProperty]
    private double _progress;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasError))]
    private string? _error;

    public ShortcutCaptureViewModel(ShellViewModel shell, ShortcutKind kind)
    {
        _shell = shell;
        Kind = kind;
        _current = Placeholder;
        CancelCommand = new RelayCommand(Cancel);
        if (IsGamepad && shell.ControllerName is null)
        {
            Error = "Nenhum controle conectado · conecte um para escolher os botões";
        }
    }

    public ShortcutKind Kind { get; }

    public bool IsGamepad => Kind == ShortcutKind.Gamepad;

    public string Title => IsGamepad ? "Atalho da Central no controle" : "Atalho da Central no teclado";

    public string Prompt => IsGamepad ? "Segure os botões que quer usar" : "Aperte as teclas que quer usar";

    public string Help => IsGamepad
        ? $"Inclua {ControlCenterChord.Name(GamepadButtons.View, _shell.PadFamily)} ou {ControlCenterChord.Name(GamepadButtons.Guide, _shell.PadFamily)}, que não navegam no Tako. Durante o jogo, a combinação vale depois de meio segundo segurando. Toque {ControlCenterChord.Name(GamepadButtons.B, _shell.PadFamily)} para cancelar."
        : "Use Ctrl ou Alt junto com uma tecla. O atalho funciona com o jogo na frente. Esc cancela.";

    /// <summary>Atalho de agora, para comparar.</summary>
    public string CurrentShortcut => IsGamepad ? $"Agora: segure {_shell.ChordLabel}" : $"Agora: {_shell.KeyLabel}";

    public bool HasError => Error is not null;

    public ICommand CancelCommand { get; }

    private string Placeholder => IsGamepad ? "Nenhum botão pressionado" : "Nenhuma tecla pressionada";

    /// <summary>Leitura do controle (a cada ~16 ms).</summary>
    public void Feed(GamepadButtons pressed, TimeSpan now)
    {
        if (!IsGamepad)
        {
            return;
        }

        var state = _capture.Update(pressed, now);
        Current = _capture.Holding == GamepadButtons.None ? Placeholder : ControlCenterChord.Format(_capture.Holding, _shell.PadFamily);
        Progress = _capture.Progress;
        switch (state)
        {
            case ChordCaptureState.Cancelled:
                Cancel();
                break;
            case ChordCaptureState.Captured:
                var check = _shell.SaveChord(_capture.Result);
                if (check.IsValid)
                {
                    _shell.CloseOverlay(this);
                }
                else
                {
                    Error = check.Error;
                    _capture.Restart();
                    Progress = 0;
                }

                break;
        }
    }

    /// <summary>Tecla do teclado físico (nome do <c>Key</c> do WPF) com os modificadores pressionados.</summary>
    public void Key(string key, KeyModifiers modifiers)
    {
        if (IsGamepad)
        {
            return;
        }

        if (key == "Escape" && modifiers == KeyModifiers.None)
        {
            Cancel();
            return;
        }

        if (KeyShortcut.IsModifierKey(key))
        {
            Current = modifiers == KeyModifiers.None ? Placeholder : new KeyShortcut(modifiers, "…").Display;
            return;
        }

        var shortcut = new KeyShortcut(modifiers, key);
        Current = shortcut.Display;
        var check = _shell.SaveKey(shortcut);
        if (check.IsValid)
        {
            _shell.CloseOverlay(this);
        }
        else
        {
            Error = check.Error;
        }
    }

    private void Cancel()
    {
        _shell.CloseOverlay(this);
        _shell.Toast($"Atalho mantido · {(IsGamepad ? $"segure {_shell.ChordLabel}" : _shell.KeyLabel)}");
    }
}
