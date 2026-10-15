using Launcher.Core.Input;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class HidDevicePathTests
{
    [Theory]
    [InlineData(@"\\?\hid#vid_054c&pid_0ce6&mi_03#7&2a1b3c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}", 0x054C, 0x0CE6)]
    [InlineData(@"\\?\HID#{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&09CC#9&1F2E&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}", 0x054C, 0x09CC)]
    [InlineData(@"\\?\hid#vid_045e&pid_0b13#8&abc&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}", 0x045E, 0x0B13)]
    public void Ids_come_from_usb_and_bluetooth_paths(string path, int vendor, int product)
    {
        Assert.True(HidDevicePath.TryParseIds(path, out ushort v, out ushort p));
        Assert.Equal((vendor, product), (v, p));
    }

    [Fact]
    public void Path_without_ids_is_ignored() => Assert.False(HidDevicePath.TryParseIds(@"\\?\hid#keyboard#1", out _, out _));

    [Fact]
    public void Sony_controllers_are_listed_once_per_device()
    {
        string[] paths =
        [
            @"\\?\hid#vid_054c&pid_0ce6&mi_03#7&aaa&0&0000#{g}",
            @"\\?\hid#vid_054c&pid_0ce6&mi_03#7&aaa&0&0000#{g}",
            @"\\?\HID#{00001124-0000-1000-8000-00805F9B34FB}_VID&0002054C_PID&05C4#9&bbb&0&0000#{g}",
            @"\\?\hid#vid_045e&pid_0b13#8&ccc&0&0000#{g}",
        ];

        Assert.Equal([SonyController.DualSense, SonyController.DualShock4], HidDevicePath.SonyControllers(paths));
    }
}
