using Launcher.Core.Input;

namespace Launcher.Core.Emulation;

/// <summary>Passo do fechamento gracioso do emulador (EMU-006).</summary>
public enum EmulatorExitStep
{
    /// <summary>Pedir para as janelas do emulador fecharem (WM_CLOSE), como o X da janela.</summary>
    CloseWindow,

    /// <summary>Mandar a tecla de sair do próprio emulador (Esc no RetroArch, duas vezes).</summary>
    SendExitKey,

    /// <summary>O emulador não fechou: perguntar no Tako se pode encerrar à força (pode perder o progresso não salvo).</summary>
    AskToForceClose,

    /// <summary>Fechou.</summary>
    Done,
}

/// <summary>
/// "Sair do emulador" pelo controle (EMU-006, ADR-0024): segurar View + Menu (Select + Start) enquanto o emulador roda
/// fecha o emulador e volta ao Tako. Primeiro WM_CLOSE; se depois de <see cref="StepWait"/> continuar aberto, a tecla
/// de sair do emulador; se ainda assim continuar, o Tako pergunta antes de encerrar. Decisão pura, testada.
/// </summary>
public sealed class EmulatorExitSequence(EmulatorDefinition emulator)
{
    /// <summary>Combinação padrão: View + Menu (Select + Start no padrão dos emuladores).</summary>
    public const GamepadButtons Chord = GamepadButtons.View | GamepadButtons.Start;

    /// <summary>Espera entre um passo e outro.</summary>
    public static readonly TimeSpan StepWait = TimeSpan.FromSeconds(3);

    public EmulatorDefinition Emulator { get; } = emulator;

    public EmulatorExitStep Current { get; private set; } = EmulatorExitStep.CloseWindow;

    /// <summary>Próximo passo depois da espera; <paramref name="stillRunning"/> = o emulador continua aberto.</summary>
    public EmulatorExitStep Next(bool stillRunning)
    {
        Current = !stillRunning ? EmulatorExitStep.Done : Current switch
        {
            EmulatorExitStep.CloseWindow when !string.IsNullOrWhiteSpace(Emulator.ExitKey) => EmulatorExitStep.SendExitKey,
            EmulatorExitStep.Done => EmulatorExitStep.Done,
            _ => EmulatorExitStep.AskToForceClose,
        };
        return Current;
    }
}
