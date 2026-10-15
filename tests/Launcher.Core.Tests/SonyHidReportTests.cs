using Launcher.Core.Input;
using Xunit;

namespace Launcher.Core.Tests;

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

    [Theory]
    [InlineData(0x054C, 0x0CE6, SonyController.DualSense)]
    [InlineData(0x054C, 0x0DF2, SonyController.DualSense)]
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

    [Theory]
    [InlineData(SonyController.DualSense, 64, 0x01, 1, 7)] // DualSense USB
    [InlineData(SonyController.DualSense, 78, 0x31, 2, 7)] // DualSense Bluetooth
    [InlineData(SonyController.DualSense, 10, 0x01, 1, 4)] // DualSense Bluetooth (relatório simples)
    [InlineData(SonyController.DualShock4, 64, 0x01, 1, 4)] // DualShock 4 USB
    [InlineData(SonyController.DualShock4, 78, 0x11, 3, 4)] // DualShock 4 Bluetooth
    public void Maps_face_buttons_and_ps_in_every_layout(SonyController model, int length, byte id, int sticks, int buttons)
    {
        // ✕ + △, L1, Options, PS
        var report = Report(length, id, sticks, buttons, b0: 0x08 | 0x20 | 0x80, b1: 0x01 | 0x20, b2: 0x01);

        Assert.True(SonyHidReport.TryParse(model, report, out var snapshot));
        Assert.Equal(GamepadButtons.A | GamepadButtons.Y | GamepadButtons.LeftShoulder | GamepadButtons.Start | GamepadButtons.Guide, snapshot.Buttons);
        Assert.Equal(0, snapshot.LeftX, 2);
        Assert.Equal(0, snapshot.LeftY, 2);
    }

    [Theory]
    [InlineData(0, GamepadButtons.DPadUp)]
    [InlineData(2, GamepadButtons.DPadRight)]
    [InlineData(5, GamepadButtons.DPadDown | GamepadButtons.DPadLeft)]
    [InlineData(8, GamepadButtons.None)]
    public void Reads_dpad_hat(byte hat, GamepadButtons expected)
    {
        Assert.True(SonyHidReport.TryParse(SonyController.DualSense, Report(64, 0x01, 1, 7, b0: hat), out var snapshot));
        Assert.Equal(expected, snapshot.Buttons);
    }

    [Fact]
    public void Stick_up_and_right_are_positive()
    {
        var report = Report(64, 0x01, 1, 7);
        report[1] = 0xFF; // direita
        report[2] = 0x00; // cima

        Assert.True(SonyHidReport.TryParse(SonyController.DualSense, report, out var snapshot));
        Assert.Equal(1, snapshot.LeftX, 2);
        Assert.Equal(1, snapshot.LeftY, 2);
    }

    [Fact]
    public void Rejects_unknown_or_short_reports()
    {
        Assert.False(SonyHidReport.TryParse(SonyController.DualSense, [0x05, 0, 0], out _));
        Assert.False(SonyHidReport.TryParse(SonyController.DualShock4, [0x01, 0x80], out _));
        Assert.False(SonyHidReport.TryParse(SonyController.DualSense, [], out _));
    }
}
