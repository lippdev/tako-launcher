namespace Launcher.Core.Input;

/// <summary>Controles PlayStation lidos direto por HID (sem Steam Input nem DS4Windows).</summary>
public enum SonyController
{
    DualShock4,
    DualSense,
}

/// <summary>
/// Interpreta relatórios de entrada HID do DualSense e do DualShock 4 (USB e Bluetooth) como um
/// <see cref="GamepadSnapshot"/> no layout Xbox: ✕ = A, ○ = B, □ = X, △ = Y, L1/R1 = LB/RB,
/// Options = Start, Create/Share = View e PS = Guia.
/// </summary>
public static class SonyHidReport
{
    public const ushort SonyVendorId = 0x054C;

    /// <summary>Modelo pelo product id USB/Bluetooth, ou nulo se não for um controle suportado.</summary>
    public static SonyController? Identify(ushort vendorId, ushort productId) => vendorId != SonyVendorId ? null : productId switch
    {
        0x05C4 or 0x09CC or 0x0BA0 => SonyController.DualShock4,
        0x0CE6 or 0x0DF2 => SonyController.DualSense,
        _ => null,
    };

    /// <summary>
    /// Layouts conhecidos (byte 0 = report id):
    /// DualSense USB 0x01 (64 bytes) e Bluetooth 0x31: analógicos, gatilhos, contador e botões a partir do 3º byte do bloco.
    /// DualShock 4 USB 0x01, Bluetooth 0x11 e o relatório simples 0x01 do Bluetooth (ambos os modelos): botões logo após os analógicos.
    /// </summary>
    public static bool TryParse(SonyController model, ReadOnlySpan<byte> report, out GamepadSnapshot snapshot)
    {
        snapshot = default;
        if (report.IsEmpty)
        {
            return false;
        }

        // (início dos analógicos, deslocamento dos botões em relação aos analógicos)
        (int sticks, int buttons) = (model, report[0], report.Length) switch
        {
            (SonyController.DualSense, 0x01, >= 64) => (1, 7),
            (SonyController.DualSense, 0x31, >= 12) => (2, 7),
            (_, 0x01, >= 8) => (1, 4),
            (SonyController.DualShock4, 0x11, >= 10) => (3, 4),
            _ => (-1, 0),
        };

        if (sticks < 0 || sticks + buttons + 2 >= report.Length)
        {
            return false;
        }

        byte b0 = report[sticks + buttons];
        byte b1 = report[sticks + buttons + 1];
        byte b2 = report[sticks + buttons + 2];

        var pressed = Hat(b0 & 0x0F);
        if ((b0 & 0x10) != 0)
        {
            pressed |= GamepadButtons.X; // □
        }

        if ((b0 & 0x20) != 0)
        {
            pressed |= GamepadButtons.A; // ✕
        }

        if ((b0 & 0x40) != 0)
        {
            pressed |= GamepadButtons.B; // ○
        }

        if ((b0 & 0x80) != 0)
        {
            pressed |= GamepadButtons.Y; // △
        }

        if ((b1 & 0x01) != 0)
        {
            pressed |= GamepadButtons.LeftShoulder;
        }

        if ((b1 & 0x02) != 0)
        {
            pressed |= GamepadButtons.RightShoulder;
        }

        if ((b1 & 0x10) != 0)
        {
            pressed |= GamepadButtons.View; // Create / Share
        }

        if ((b1 & 0x20) != 0)
        {
            pressed |= GamepadButtons.Start; // Options
        }

        if ((b2 & 0x01) != 0)
        {
            pressed |= GamepadButtons.Guide; // PS
        }

        // HID: 0..255 com centro 128 e Y crescendo para baixo; o snapshot usa Y positivo para cima.
        double x = Axis(report[sticks]);
        double y = -Axis(report[sticks + 1]);
        snapshot = new GamepadSnapshot(pressed, x, y);
        return true;
    }

    private static double Axis(byte value) => Math.Clamp((value - 127.5) / 127.5, -1, 1);

    /// <summary>Direcional como chapéu: 0 = cima, sentido horário, 8 = solto.</summary>
    private static GamepadButtons Hat(int value) => value switch
    {
        0 => GamepadButtons.DPadUp,
        1 => GamepadButtons.DPadUp | GamepadButtons.DPadRight,
        2 => GamepadButtons.DPadRight,
        3 => GamepadButtons.DPadDown | GamepadButtons.DPadRight,
        4 => GamepadButtons.DPadDown,
        5 => GamepadButtons.DPadDown | GamepadButtons.DPadLeft,
        6 => GamepadButtons.DPadLeft,
        7 => GamepadButtons.DPadUp | GamepadButtons.DPadLeft,
        _ => GamepadButtons.None,
    };
}
