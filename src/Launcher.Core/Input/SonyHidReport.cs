namespace Launcher.Core.Input;

/// <summary>Controles PlayStation lidos direto por HID (sem Steam Input nem DS4Windows).</summary>
public enum SonyController
{
    DualShock4,
    DualSense,

    /// <summary>DualSense Edge (0x0DF2): mesmo relatório do DualSense, com Fn e botões traseiros a mais.</summary>
    DualSenseEdge,
}

/// <summary>Como o controle está ligado ao PC. Muda o layout do relatório 0x01 do DualSense.</summary>
public enum HidTransport
{
    Usb,
    Bluetooth,
}

/// <summary>Formato do relatório de entrada reconhecido (aparece no teste de controle e no diagnóstico).</summary>
public enum SonyReportLayout
{
    /// <summary>DualSense pelo cabo: 0x01 com 64 bytes, analógicos no byte 1 e botões no 8.</summary>
    DualSenseUsb,

    /// <summary>DualSense pelo Bluetooth, modo completo (depois que alguém leu a calibração): 0x31, botões no byte 9.</summary>
    DualSenseBluetooth,

    /// <summary>
    /// Relatório simples 0x01 (Bluetooth sem o modo completo, e o DualShock 4 pelo cabo): botões no byte 5, gatilhos
    /// nos bytes 8 e 9. O Windows completa com zeros até o tamanho do maior relatório (78 bytes no DualSense).
    /// </summary>
    Simple,

    /// <summary>DualShock 4 pelo Bluetooth, modo completo: 0x11, analógicos no byte 3.</summary>
    DualShock4Bluetooth,
}

/// <summary>Botões que não existem no layout Xbox (só aparecem no teste de controle).</summary>
[Flags]
public enum SonyExtraButtons
{
    None = 0,
    L2 = 1 << 0,
    R2 = 1 << 1,
    L3 = 1 << 2,
    R3 = 1 << 3,
    Touchpad = 1 << 4,
    Mute = 1 << 5,

    /// <summary>Fn esquerdo do DualSense Edge.</summary>
    LeftFunction = 1 << 6,

    /// <summary>Fn direito do DualSense Edge.</summary>
    RightFunction = 1 << 7,

    /// <summary>Botão traseiro esquerdo do DualSense Edge.</summary>
    LeftPaddle = 1 << 8,

    /// <summary>Botão traseiro direito do DualSense Edge.</summary>
    RightPaddle = 1 << 9,
}

/// <summary>Leitura completa de um relatório: o que a navegação usa e o resto, para o teste de controle.</summary>
/// <param name="Hat">Chapéu do direcional cru (0 = cima, sentido horário, 8 = solto).</param>
public readonly record struct SonyHidReading(
    GamepadSnapshot Snapshot,
    SonyReportLayout Layout,
    int Hat,
    double RightX,
    double RightY,
    byte LeftTrigger,
    byte RightTrigger,
    SonyExtraButtons Extra);

/// <summary>
/// Interpreta relatórios de entrada HID do DualSense, DualSense Edge e DualShock 4 (USB e Bluetooth) como um
/// <see cref="GamepadSnapshot"/> no layout Xbox: ✕ = A, ○ = B, □ = X, △ = Y, L1/R1 = LB/RB,
/// Options = Start, Create/Share = View e PS = Guia. Formato conforme a documentação pública dos relatórios
/// (o mesmo que drivers como o hid-playstation do Linux e o SDL usam); nenhum código copiado.
/// </summary>
public static class SonyHidReport
{
    public const ushort SonyVendorId = 0x054C;
    public const ushort DualSenseEdgeProductId = 0x0DF2;

    /// <summary>Modelo pelo product id USB/Bluetooth, ou nulo se não for um controle suportado.</summary>
    public static SonyController? Identify(ushort vendorId, ushort productId) => vendorId != SonyVendorId ? null : productId switch
    {
        0x05C4 or 0x09CC or 0x0BA0 => SonyController.DualShock4,
        0x0CE6 => SonyController.DualSense,
        DualSenseEdgeProductId => SonyController.DualSenseEdge,
        _ => null,
    };

    /// <summary>Nome para avisos e para o diagnóstico.</summary>
    public static string DisplayName(SonyController model) => model switch
    {
        SonyController.DualSenseEdge => "Controle PS5 (DualSense Edge)",
        SonyController.DualSense => "Controle PS5 (DualSense)",
        _ => "Controle PS4 (DualShock 4)",
    };

    public static bool TryParse(SonyController model, HidTransport transport, ReadOnlySpan<byte> report, out GamepadSnapshot snapshot)
    {
        bool ok = TryRead(model, transport, report, out var reading);
        snapshot = reading.Snapshot;
        return ok;
    }

    /// <summary>
    /// Layouts (byte 0 = report id):
    /// DualSense/Edge USB 0x01 (64 bytes) e Bluetooth 0x31 (78 bytes): analógicos, gatilhos, contador e então 4 bytes de
    /// botões. Relatório simples 0x01 (DualSense/Edge no Bluetooth antes do modo completo, DualShock 4 USB e Bluetooth):
    /// botões logo após os analógicos. DualShock 4 Bluetooth completo 0x11: igual ao simples, dois bytes adiante.
    /// No Bluetooth o Windows entrega o relatório simples completado com zeros (78 bytes no DualSense): o tamanho sozinho
    /// não separa o simples do USB, por isso o transporte decide.
    /// </summary>
    public static bool TryRead(SonyController model, HidTransport transport, ReadOnlySpan<byte> report, out SonyHidReading reading)
    {
        reading = default;
        if (report.IsEmpty)
        {
            return false;
        }

        bool dualSense = model is SonyController.DualSense or SonyController.DualSenseEdge;
        SonyReportLayout? layout = (dualSense, report[0], transport, report.Length) switch
        {
            (true, 0x01, HidTransport.Usb, >= 64) => SonyReportLayout.DualSenseUsb,
            (true, 0x31, _, >= 12) => SonyReportLayout.DualSenseBluetooth,
            (_, 0x01, _, >= 8) => SonyReportLayout.Simple,
            (false, 0x11, _, >= 10) => SonyReportLayout.DualShock4Bluetooth,
            _ => null,
        };

        if (layout is not { } found)
        {
            return false;
        }

        // (início dos analógicos, deslocamento dos botões e dos gatilhos em relação aos analógicos)
        (int sticks, int buttons, int triggers) = found switch
        {
            SonyReportLayout.DualSenseUsb => (1, 7, 4),
            SonyReportLayout.DualSenseBluetooth => (2, 7, 4),
            SonyReportLayout.DualShock4Bluetooth => (3, 4, 7),
            _ => (1, 4, 7),
        };

        byte b0 = report[sticks + buttons];
        byte b1 = report[sticks + buttons + 1];
        byte b2 = report[sticks + buttons + 2];
        bool full = found is SonyReportLayout.DualSenseUsb or SonyReportLayout.DualSenseBluetooth;

        int hat = b0 & 0x0F;
        var pressed = Hat(hat);
        pressed |= Bit(b0, 0x10, GamepadButtons.X); // □
        pressed |= Bit(b0, 0x20, GamepadButtons.A); // ✕
        pressed |= Bit(b0, 0x40, GamepadButtons.B); // ○
        pressed |= Bit(b0, 0x80, GamepadButtons.Y); // △
        pressed |= Bit(b1, 0x01, GamepadButtons.LeftShoulder);
        pressed |= Bit(b1, 0x02, GamepadButtons.RightShoulder);
        pressed |= Bit(b1, 0x10, GamepadButtons.View); // Create / Share
        pressed |= Bit(b1, 0x20, GamepadButtons.Start); // Options
        pressed |= Bit(b2, 0x01, GamepadButtons.Guide); // PS

        var extra = Bit(b1, 0x04, SonyExtraButtons.L2) | Bit(b1, 0x08, SonyExtraButtons.R2)
            | Bit(b1, 0x40, SonyExtraButtons.L3) | Bit(b1, 0x80, SonyExtraButtons.R3)
            | Bit(b2, 0x02, SonyExtraButtons.Touchpad);
        if (full)
        {
            // No simples, os bits altos deste byte são o contador de relatórios, não botões.
            extra |= Bit(b2, 0x04, SonyExtraButtons.Mute)
                | Bit(b2, 0x10, SonyExtraButtons.LeftFunction) | Bit(b2, 0x20, SonyExtraButtons.RightFunction)
                | Bit(b2, 0x40, SonyExtraButtons.LeftPaddle) | Bit(b2, 0x80, SonyExtraButtons.RightPaddle);
        }

        // HID: 0..255 com centro 128 e Y crescendo para baixo; o snapshot usa Y positivo para cima.
        var snapshot = new GamepadSnapshot(pressed, Axis(report[sticks]), -Axis(report[sticks + 1]));
        byte l2 = sticks + triggers < report.Length ? report[sticks + triggers] : (byte)0;
        byte r2 = sticks + triggers + 1 < report.Length ? report[sticks + triggers + 1] : (byte)0;
        reading = new SonyHidReading(snapshot, found, hat, Axis(report[sticks + 2]), -Axis(report[sticks + 3]), l2, r2, extra);
        return true;
    }

    private static GamepadButtons Bit(byte value, int mask, GamepadButtons button) => (value & mask) != 0 ? button : GamepadButtons.None;

    private static SonyExtraButtons Bit(byte value, int mask, SonyExtraButtons button) => (value & mask) != 0 ? button : SonyExtraButtons.None;

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
