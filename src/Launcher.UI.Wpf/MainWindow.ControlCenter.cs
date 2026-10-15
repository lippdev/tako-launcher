using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Threading;
using ConsoleMode.Runtime;
using Launcher.Core.Input;
using Launcher.UI.Wpf.Input;
using Launcher.UI.Wpf.Services;
using Launcher.UI.Wpf.ViewModels;
using Microsoft.Extensions.Logging;

namespace Launcher.UI.Wpf;

/// <summary>
/// Atalho da Central de Controle (UX-005) e Central "ao vivo" sobre o jogo. Controle: a combinação configurada,
/// segurada por meio segundo (padrão View + Y), lida com o Tako na frente ou atrás. Teclado: o atalho configurado
/// (padrão Ctrl+Shift+T), registrado como atalho global do Windows enquanto um jogo real está aberto.
/// </summary>
public partial class MainWindow
{
    private const int HotkeyId = 0x7A4C;
    private const int WmHotkey = 0x0312;
    private const uint ModNoRepeat = 0x4000;

    private ButtonChordDetector _chord = null!;
    private LiveOverlayPresenter _live = null!;
    private KeyShortcut? _registeredHotkey;
    private HwndSource? _hwndSource;

    /// <summary>Botões pressionados com a tela de captura do controle aberta (filtro do eco de teclado da Steam).</summary>
    private GamepadButtons _captureButtons;

    private void InitializeControlCenterShortcuts()
    {
        _chord = new ButtonChordDetector(_shell.Settings.ControlCenterChord);
        _live = new LiveOverlayPresenter(this, Stage, [Backdrop, ContentArea, Sidebar, PlayniteHost]);
        _shell.Settings.PropertyChanged += OnShortcutSettingChanged;
        SourceInitialized += (_, _) =>
        {
            _hwndSource = HwndSource.FromHwnd(new WindowInteropHelper(this).Handle);
            _hwndSource?.AddHook(OnWindowMessage);
        };
        Closed += (_, _) => UnregisterGlobalHotkey();
    }

    private void OnShortcutSettingChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(SettingsService.ControlCenterChord):
                _chord.Chord = _shell.Settings.ControlCenterChord;
                break;
            case nameof(SettingsService.ControlCenterKey) when _registeredHotkey is not null:
                UnregisterGlobalHotkey();
                UpdateGlobalHotkey();
                break;
        }
    }

    /// <summary>
    /// Leitura do controle para o atalho, com o Tako na frente ou atrás. Verdadeiro se a leitura foi consumida (tela de
    /// captura aberta ou combinação acabou de disparar).
    /// </summary>
    private bool HandleControlCenterChord(GamepadButtons buttons)
    {
        var now = _clock.Elapsed;
        if (_shell.TopOverlay is ShortcutCaptureViewModel { IsGamepad: true } capture)
        {
            // A tela de captura recebe tudo; nada vira comando e a combinação nova só vale depois de soltar.
            _gamepad.Reset();
            _captureButtons = buttons;
            if (IsActive)
            {
                capture.Feed(buttons, now);
            }

            return true;
        }

        _captureButtons = GamepadButtons.None;

        if (!_chord.Update(buttons, now))
        {
            return false;
        }

        _gamepad.Reset(); // os botões da combinação não viram comando ao soltar
        if (IsActive)
        {
            Handle(AppCommand.Guide, InputKind.Gamepad);
        }
        else
        {
            SummonFromBackground(InputKind.Gamepad);
        }

        return true;
    }

    /// <summary>Atalho de teclado com o Tako em primeiro plano (sem o atalho global registrado).</summary>
    private bool IsControlCenterKey(Key key)
    {
        var shortcut = _shell.Settings.ControlCenterKey;
        return key.ToString() == shortcut.Key && (KeyModifiers)Keyboard.Modifiers == shortcut.Modifiers;
    }

    /// <summary>
    /// Com um jogo real na frente, o atalho abre a Central. Fora da tela cheia exclusiva, ao vivo: a janela fica
    /// transparente por cima do jogo antes de pedir o foco (para não mostrar a biblioteca por um instante).
    /// </summary>
    private void SummonFromBackground(InputKind source)
    {
        if (!_shell.CanOpenControlCenterFromBackground)
        {
            return;
        }

        bool live = !LiveOverlayPresenter.IsExclusiveFullscreen();
        _shell.InputKind = source;
        OpenControlCenterOverGame(live ? LiveOverlayPresenter.ForegroundWindow() : 0, live);
    }

    /// <summary>Exposto ao teste de fumaça: abre a Central ao vivo (ou com a arte) como o atalho com o jogo na frente.</summary>
    internal void OpenControlCenterOverGame(nint gameWindow, bool live)
    {
        if (WindowState == WindowState.Minimized)
        {
            WindowState = WindowState.Maximized;
        }

        if (live)
        {
            _live.Enter(gameWindow);
        }

        if (!_shell.OpenControlCenterFromBackground(live))
        {
            _live.Leave();
            _live.Restore();
            return;
        }

        LogControlCenterOpened(_logger, live);
        // Foco do teclado na Central. O jogo pode continuar lendo o controle em segundo plano (ver DESIGN.md).
        _shell.SessionTools.Windows.Activate(new WindowInteropHelper(this).Handle);
    }

    /// <summary>A Central ao vivo fechou: some de cima do jogo já; a tela normal do Tako volta depois do jogo.</summary>
    private void OnLiveOverlayChanged()
    {
        if (_shell.IsLiveOverlay || !_live.IsActive)
        {
            return;
        }

        _live.Leave();
        Dispatcher.BeginInvoke(_live.Restore, DispatcherPriority.Background);
    }

    internal bool IsLivePresentation => _live.IsActive;

    // ---------------------------------------------------------------- Atalho global do teclado

    /// <summary>Durante um jogo real o atalho de teclado é global (funciona com o jogo na frente); fora dele, não.</summary>
    private void UpdateGlobalHotkey()
    {
        bool wanted = _shell.Session != SessionState.Browsing && _shell.IsTrackingRealGame;
        if (!wanted)
        {
            UnregisterGlobalHotkey();
            return;
        }

        if (_registeredHotkey is not null || _hwndSource is null)
        {
            return;
        }

        var shortcut = _shell.Settings.ControlCenterKey;
        if (!Enum.TryParse(shortcut.Key, out Key key))
        {
            return;
        }

        uint modifiers = (uint)(shortcut.Modifiers & (KeyModifiers.Alt | KeyModifiers.Ctrl | KeyModifiers.Shift)) | ModNoRepeat;
        if (RegisterHotKey(_hwndSource.Handle, HotkeyId, modifiers, (uint)KeyInterop.VirtualKeyFromKey(key)))
        {
            _registeredHotkey = shortcut;
        }
        else
        {
            LogHotkeyUnavailable(_logger, shortcut.Display);
            _shell.Toast($"{shortcut.Display} já é usado por outro programa · troque o atalho nas Configurações");
        }
    }

    private void UnregisterGlobalHotkey()
    {
        if (_registeredHotkey is not null && _hwndSource is not null)
        {
            UnregisterHotKey(_hwndSource.Handle, HotkeyId);
        }

        _registeredHotkey = null;
    }

    private nint OnWindowMessage(nint hwnd, int msg, nint wParam, nint lParam, ref bool handled)
    {
        if (msg == WmHotkey && wParam == HotkeyId)
        {
            handled = true;
            if (IsActive)
            {
                Handle(AppCommand.Guide, InputKind.Keyboard);
            }
            else
            {
                SummonFromBackground(InputKind.Keyboard);
            }
        }

        return 0;
    }

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool RegisterHotKey(nint hwnd, int id, uint modifiers, uint virtualKey);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool UnregisterHotKey(nint hwnd, int id);

    [LoggerMessage(Level = LogLevel.Information, Message = "Central de Controle aberta com o jogo na frente (ao vivo: {Live})")]
    private static partial void LogControlCenterOpened(ILogger logger, bool live);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Atalho global {Shortcut} indisponível: outro programa já o registrou")]
    private static partial void LogHotkeyUnavailable(ILogger logger, string shortcut);
}
