// Adaptado de lippdev/consolemode: tests/ConsoleMode.Tests/DisplayIdentityTests.cs
using System.Text;
using ConsoleMode.Runtime.Display;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

public sealed class DisplayIdentityTests
{
    [Fact]
    public void Device_path_gives_the_hardware_id_and_the_device_instance()
    {
        Assert.True(DisplayIdentity.TryParseDevicePath(
            @"\\?\DISPLAY#GSM5B7F#5&2d8e1c7&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}", out string hw, out string instance));
        Assert.Equal("GSM5B7F", hw);
        Assert.Equal(@"DISPLAY\GSM5B7F\5&2d8e1c7&0&UID4352", instance);

        Assert.False(DisplayIdentity.TryParseDevicePath(string.Empty, out _, out _));
        Assert.False(DisplayIdentity.TryParseDevicePath(@"\\?\DISPLAY#GSM5B7F", out _, out _));
    }

    [Fact]
    public void Monitor_id_has_the_console_mode_format()
    {
        Assert.Equal(
            @"MONITOR\GSM5B7F\{4d36e96e-e325-11ce-bfc1-08002be10318}\0004",
            DisplayIdentity.MonitorId("GSM5B7F", @"{4d36e96e-e325-11ce-bfc1-08002be10318}\0004"));
        Assert.Equal(string.Empty, DisplayIdentity.MonitorId("GSM5B7F", null));
    }

    [Fact]
    public void Edid_gives_name_serial_and_native_resolution()
    {
        var edid = new byte[128];

        // Temporização detalhada 3840 × 2160 (o modo preferido).
        edid[54] = 0x08;
        edid[55] = 0xE8;
        edid[56] = 0x00;
        edid[58] = 0xF0;
        edid[59] = 0x70;
        edid[61] = 0x80;
        WriteText(edid, 72, 0xFC, "LG TV SSCR2");
        WriteText(edid, 90, 0xFF, "0x01010101");

        var info = DisplayIdentity.ParseEdid(edid);

        Assert.Equal("LG TV SSCR2", info.Name);
        Assert.Equal("0x01010101", info.Serial);
        Assert.Equal(3840, info.PreferredWidth);
        Assert.Equal(2160, info.PreferredHeight);
        Assert.Equal(EdidInfo.Empty, DisplayIdentity.ParseEdid(null));
    }

    [Fact]
    public void Active_monitors_keep_their_source_and_inactive_ones_get_a_free_one()
    {
        IReadOnlyList<int> all = [0, 1, 2];
        var assigned = DisplayIdentity.AssignSources<string, int>(
        [
            ("desk", 0, null, all),
            ("tv", null, 0, all),      // quer a 0 (a última dele), mas a mesa está com ela
            ("side", null, 2, all),    // recebe a última dele
            ("projector", null, null, all),
        ]);

        Assert.Equal(0, assigned["desk"]);
        Assert.Equal(2, assigned["side"]);
        Assert.Equal(1, assigned["tv"]);
        Assert.False(assigned.ContainsKey("projector")); // acabaram as fontes
    }

    private static void WriteText(byte[] edid, int offset, byte tag, string text)
    {
        edid[offset + 3] = tag;
        byte[] bytes = Encoding.ASCII.GetBytes(text + "\n");
        for (int i = 0; i < 13; i++)
        {
            edid[offset + 5 + i] = i < bytes.Length ? bytes[i] : (byte)0x20;
        }
    }
}
