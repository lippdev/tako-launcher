using Launcher.Core.Input;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// Relatórios de exemplo montados byte a byte conforme o formato público do DualSense/Edge e do DualShock 4 (o mesmo
/// descrito pelos drivers hid-playstation e SDL); nenhum relatório real de usuário.
/// </summary>
public class SonyHidReportTests
{
    /// <summary>Relatório com analógicos no centro e direcional solto.</summary>
    private static byte[] Report(int length, byte id, int sticks, int buttons, byte b0 = 0x08, byte b1 = 0, byte b2 = 0)
    {
        var report = new byte[length];
        report[0] = id;
        for (int i = 0; i < 4; i++)
        {
            report[sticks + i] = 0x80;
        }

        report[sticks + buttons] = b0;
        report[sticks + buttons + 1] = b1;
        report[sticks + buttons + 2] = b2;
        return report;
    }

    /// <summary>
    /// DualSense/Edge no Bluetooth antes do modo completo: relatório simples 0x01 (10 bytes) que o Windows entrega
    /// completado com zeros até 78 bytes (o tamanho do 0x31). Gatilhos soltos (0) nos bytes 8 e 9.
    /// </summary>
    private static byte[] BluetoothSimplePadded(byte b0 = 0x08, byte b1 = 0, byte b2 = 0)
    {
        var report = Report(78, 0x01, 1, 4, b0, b1, b2);
        report[8] = 0x00; // L2
        report[9] = 0x00; // R2
        return report;
    }

    [Theory]
    [InlineData(0x054C, 0x0CE6, SonyController.DualSense)]
    [InlineData(0x054C, 0x0DF2, SonyController.DualSenseEdge)]
    [InlineData(0x054C, 0x05C4, SonyController.DualShock4)]
    [InlineData(0x054C, 0x09CC, SonyController.DualShock4)]
    public void Identifies_supported_controllers(int vendor, int product, SonyController expected) =>
        Assert.Equal(expected, SonyHidReport.Identify((ushort)vendor, (ushort)product));

    [Fact]
    public void Ignores_other_devices()
    {
        Assert.Null(SonyHidReport.Identify(0x045E, 0x0B13));
        Assert.Null(SonyHidReport.Identify(0x054C, 0x0001));
    }

    [Fact]
    public void Edge_has_its_own_name() =>
        Assert.Equal("Controle PS5 (DualSense Edge)", SonyHidReport.DisplayName(SonyController.DualSenseEdge));

    [Theory]
    [InlineData(SonyController.DualSense, HidTransport.Usb, 64, 0x01, 1, 7)] // DualSense USB
    [InlineData(SonyController.DualSense, HidTransport.Bluetooth, 78, 0x31, 2, 7)] // DualSense Bluetooth
    [InlineData(SonyController.DualSense, HidTransport.Bluetooth, 10, 0x01, 1, 4)] // DualSense Bluetooth (relatório simples)
    [InlineData(SonyController.DualSense, HidTransport.Bluetooth, 78, 0x01, 1, 4)] // simples completado com zeros pelo Windows
    [InlineData(SonyController.DualSenseEdge, HidTransport.Usb, 64, 0x01, 1, 7)] // Edge USB
    [InlineData(SonyController.DualSenseEdge, HidTransport.Bluetooth, 78, 0x31, 2, 7)] // Edge Bluetooth
    [InlineData(SonyController.DualSenseEdge, HidTransport.Bluetooth, 78, 0x01, 1, 4)] // Edge Bluetooth (simples)
    [InlineData(SonyController.DualShock4, HidTransport.Usb, 64, 0x01, 1, 4)] // DualShock 4 USB
    [InlineData(SonyController.DualShock4, HidTransport.Bluetooth, 78, 0x11, 3, 4)] // DualShock 4 Bluetooth
    public void Maps_face_buttons_and_ps_in_every_layout(SonyController model, HidTransport transport, int length, byte id, int sticks, int buttons)
    {
        // ✕ + △, L1, Options, PS
        var report = Report(length, id, sticks, buttons, b0: 0x08 | 0x20 | 0x80, b1: 0x01 | 0x20, b2: 0x01);

        Assert.True(SonyHidReport.TryParse(model, transport, report, out var snapshot));
        Assert.Equal(GamepadButtons.A | GamepadButtons.Y | GamepadButtons.LeftShoulder | GamepadButtons.Start | GamepadButtons.Guide, snapshot.Buttons);
        Assert.Equal(0, snapshot.LeftX, 2);
        Assert.Equal(0, snapshot.LeftY, 2);
    }

    [Theory]
    [InlineData(0, GamepadButtons.DPadUp)]
    [InlineData(1, GamepadButtons.DPadUp | GamepadButtons.DPadRight)]
    [InlineData(2, GamepadButtons.DPadRight)]
    [InlineData(5, GamepadButtons.DPadDown | GamepadButtons.DPadLeft)]
    [InlineData(7, GamepadButtons.DPadUp | GamepadButtons.DPadLeft)]
    [InlineData(8, GamepadButtons.None)]
    [InlineData(15, GamepadButtons.None)]
    public void Reads_dpad_hat(byte hat, GamepadButtons expected)
    {
        Assert.True(SonyHidReport.TryParse(SonyController.DualSenseEdge, HidTransport.Usb, Report(64, 0x01, 1, 7, b0: hat), out var snapshot));
        Assert.Equal(expected, snapshot.Buttons);
    }

    /// <summary>
    /// Bug da alpha.9 ("DualSense Edge não vai para cima"): no Bluetooth sem a Steam, o controle manda o relatório
    /// simples, que o Windows completa com zeros até 78 bytes. Lido como o layout USB, o "direcional" vinha do byte do
    /// L2 solto (0 = cima): ↑ ficava sempre pressionado, nunca disparava, e os botões não respondiam.
    /// </summary>
    [Fact]
    public void Bluetooth_simple_report_padded_by_windows_is_not_read_as_usb()
    {
        Assert.True(SonyHidReport.TryParse(SonyController.DualSenseEdge, HidTransport.Bluetooth, BluetoothSimplePadded(), out var released));
        Assert.Equal(GamepadButtons.None, released.Buttons);

        Assert.True(SonyHidReport.TryParse(SonyController.DualSenseEdge, HidTransport.Bluetooth, BluetoothSimplePadded(b0: 0x00), out var up));
        Assert.Equal(GamepadButtons.DPadUp, up.Buttons);

        Assert.True(SonyHidReport.TryParse(SonyController.DualSenseEdge, HidTransport.Bluetooth, BluetoothSimplePadded(b0: 0x08 | 0x20), out var cross));
        Assert.Equal(GamepadButtons.A, cross.Buttons);
    }

    [Fact]
    public void Usb_report_with_released_dpad_and_released_triggers_has_no_direction()
    {
        var report = Report(64, 0x01, 1, 7);
        report[5] = 0; // L2 solto
        report[6] = 0; // R2 solto
        report[7] = 0; // contador

        Assert.True(SonyHidReport.TryParse(SonyController.DualSenseEdge, HidTransport.Usb, report, out var snapshot));
        Assert.Equal(GamepadButtons.None, snapshot.Buttons);
    }

    [Fact]
    public void Edge_function_and_back_buttons_do_not_shift_the_dpad()
    {
        // Fn esquerdo + Fn direito + traseiros, com ↑ no direcional.
        var usb = Report(64, 0x01, 1, 7, b0: 0x00, b2: 0x10 | 0x20 | 0x40 | 0x80);
        var bt = Report(78, 0x31, 2, 7, b0: 0x00, b2: 0x10 | 0x20 | 0x40 | 0x80);

        foreach (var (report, transport) in new[] { (usb, HidTransport.Usb), (bt, HidTransport.Bluetooth) })
        {
            Assert.True(SonyHidReport.TryRead(SonyController.DualSenseEdge, transport, report, out var reading));
            Assert.Equal(GamepadButtons.DPadUp, reading.Snapshot.Buttons);
            Assert.Equal(0, reading.Hat);
            Assert.Equal(
                SonyExtraButtons.LeftFunction | SonyExtraButtons.RightFunction | SonyExtraButtons.LeftPaddle | SonyExtraButtons.RightPaddle,
                reading.Extra);
        }
    }

    [Fact]
    public void Simple_report_counter_bits_are_not_read_as_buttons()
    {
        // No simples, os 6 bits altos do 3º byte de botões são o contador de relatórios.
        Assert.True(SonyHidReport.TryRead(SonyController.DualSense, HidTransport.Bluetooth, BluetoothSimplePadded(b2: 0xFC), out var reading));
        Assert.Equal(SonyExtraButtons.None, reading.Extra);
        Assert.Equal(GamepadButtons.None, reading.Snapshot.Buttons);
        Assert.Equal(SonyReportLayout.Simple, reading.Layout);
    }

    [Fact]
    public void Reads_triggers_and_right_stick_for_the_test_screen()
    {
        var report = Report(64, 0x01, 1, 7, b1: 0x04);
        report[3] = 0x00; // analógico direito à esquerda
        report[5] = 200; // L2

        Assert.True(SonyHidReport.TryRead(SonyController.DualSense, HidTransport.Usb, report, out var reading));
        Assert.Equal(SonyReportLayout.DualSenseUsb, reading.Layout);
        Assert.Equal(-1, reading.RightX, 2);
        Assert.Equal(200, reading.LeftTrigger);
        Assert.Equal(SonyExtraButtons.L2, reading.Extra);
    }

    [Fact]
    public void Stick_up_and_right_are_positive()
    {
        var report = Report(64, 0x01, 1, 7);
        report[1] = 0xFF; // direita
        report[2] = 0x00; // cima

        Assert.True(SonyHidReport.TryParse(SonyController.DualSense, HidTransport.Usb, report, out var snapshot));
        Assert.Equal(1, snapshot.LeftX, 2);
        Assert.Equal(1, snapshot.LeftY, 2);
    }

    [Fact]
    public void Rejects_unknown_or_short_reports()
    {
        Assert.False(SonyHidReport.TryParse(SonyController.DualSense, HidTransport.Usb, [0x05, 0, 0], out _));
        Assert.False(SonyHidReport.TryParse(SonyController.DualShock4, HidTransport.Usb, [0x01, 0x80], out _));
        Assert.False(SonyHidReport.TryParse(SonyController.DualSense, HidTransport.Usb, [], out _));
        Assert.False(SonyHidReport.TryParse(SonyController.DualSenseEdge, HidTransport.Bluetooth, [0x31, 0, 0x80, 0x80], out _));
    }
}
