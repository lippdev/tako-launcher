using Launcher.Core.Emulation;
using Launcher.Core.Input;

namespace Launcher.UI.Wpf;

/// <summary>
/// "Sair do emulador" pelo controle (EMU-006, ADR-0024): com um emulador aberto pelo Tako e o Tako atrás, segurar
/// View + Menu (Select + Start) por meio segundo fecha o emulador com calma e volta ao Tako. Lido no mesmo laço do
/// atalho da Central de Controle.
/// </summary>
public partial class MainWindow
{
    private readonly ButtonChordDetector _emulatorExitChord = new(EmulatorExitSequence.Chord);

    /// <summary>Verdadeiro se a leitura foi consumida (a combinação acabou de disparar).</summary>
    private bool HandleEmulatorExitChord(GamepadButtons buttons)
    {
        if (IsActive || !_shell.CanExitEmulatorFromController)
        {
            _emulatorExitChord.Update(GamepadButtons.None, _clock.Elapsed); // solta: a próxima pressão começa do zero
            return false;
        }

        if (!_emulatorExitChord.Update(buttons, _clock.Elapsed))
        {
            return false;
        }

        _gamepad.Reset(); // os botões da combinação não viram comando ao voltar para o Tako
        _shell.ExitEmulator();
        return true;
    }
}
