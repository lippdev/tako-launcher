using ConsoleMode.Runtime.SessionTools;
using Xunit;

namespace ConsoleMode.Runtime.Tests.SessionTools;

public class FpsOverlayTests
{
    private static readonly FrameStats Frames = new(143.6, 6.96, 117.2, "DXGI", 1234);

    private static readonly HardwareSample Sample = new()
    {
        CpuUsage = 41.2,
        CpuClockMhz = 4500,
        GpuUsage = 97,
        GpuClockMhz = 2610,
        GpuTempC = 71,
        VramUsedMb = 6656,
        VramTotalMb = 12288,
        RamUsedMb = 9216,
        RamTotalMb = 16384,
    };

    [Theory]
    [InlineData(null, FpsOverlay.FpsOnly)]
    [InlineData("detailed", FpsOverlay.Detailed)]
    [InlineData("STATS", FpsOverlay.Compact)]
    [InlineData("external", FpsOverlay.FpsOnly)] // o "O do RTSS" de antes
    [InlineData("custom", FpsOverlay.Detailed)] // o Personalizado do RTSS
    [InlineData("off", FpsOverlay.Off)]
    public void Saved_styles_from_the_rtss_days_still_load(string? saved, string expected) =>
        Assert.Equal(expected, FpsOverlay.Normalize(saved));

    [Fact]
    public void Old_rtss_position_becomes_top_left()
    {
        Assert.Equal(FpsOverlay.TopLeft, FpsOverlay.NormalizePosition("rtss"));
        Assert.Equal(FpsOverlay.BottomRight, FpsOverlay.NormalizePosition("bottom-right"));
        Assert.Equal(FpsOverlay.Small, FpsOverlay.NormalizeSize("huge"));
        Assert.Equal(2.0, FpsOverlay.Scale(FpsOverlay.Large));
    }

    [Fact]
    public void Off_draws_nothing() => Assert.Empty(FpsOverlay.Rows(FpsOverlay.Off, Frames, Sample));

    [Fact]
    public void Fps_only_is_one_number()
    {
        Assert.Equal("144 FPS", FpsOverlay.PlainText(FpsOverlay.Rows(FpsOverlay.FpsOnly, Frames)));
        Assert.Equal("-- FPS", FpsOverlay.PlainText(FpsOverlay.Rows(FpsOverlay.FpsOnly, null)));
        Assert.False(FpsOverlay.NeedsHardware(FpsOverlay.FpsOnly));
    }

    [Fact]
    public void Compact_is_one_line_with_the_pc()
    {
        var rows = FpsOverlay.Rows(FpsOverlay.Compact, Frames, Sample);
        Assert.Single(rows);
        Assert.Equal("144 FPS   CPU 41%   GPU 97%   RAM 56%", FpsOverlay.PlainText(rows));
        Assert.True(FpsOverlay.NeedsHardware(FpsOverlay.Compact));
        Assert.Equal("144 FPS   CPU --%   GPU --%   RAM --%", FpsOverlay.PlainText(FpsOverlay.Rows(FpsOverlay.Compact, Frames, null)));
    }

    [Fact]
    public void Detailed_has_frame_time_one_percent_low_api_and_the_pc()
    {
        string text = FpsOverlay.PlainText(FpsOverlay.Rows(FpsOverlay.Detailed, Frames, Sample));
        Assert.Equal(
            "144 FPS   7.0 ms   DXGI\n1% low 117 FPS\nCPU 41%   4.5 GHz\nGPU 97%   2610 MHz   71°C\nVRAM 6.5 / 12 GB\nRAM 9.0 / 16 GB",
            text);
    }

    [Fact]
    public void Parts_the_pc_does_not_report_disappear()
    {
        var bare = new HardwareSample { CpuUsage = 10 };
        string text = FpsOverlay.PlainText(FpsOverlay.Rows(FpsOverlay.Detailed, null, bare));
        Assert.Equal("-- FPS   -- ms\n1% low -- FPS\nCPU 10%", text);
    }

    [Fact]
    public void Runs_carry_their_role_for_the_window()
    {
        var first = FpsOverlay.Rows(FpsOverlay.Detailed, Frames, Sample)[0];
        Assert.Equal(OverlayRunKind.Fps, first[0].Kind);
        Assert.Contains(first, r => r.Kind == OverlayRunKind.Accent && r.Text == "DXGI");
    }
}

public class OverlayPlacementTests
{
    private static readonly PixelRect Uhd = new(0, 0, 3840, 2160);

    [Theory]
    [InlineData(FpsOverlay.TopLeft, 24, 24)]
    [InlineData(FpsOverlay.TopRight, 3840 - 24 - 300, 24)]
    [InlineData(FpsOverlay.BottomLeft, 24, 2160 - 24 - 90)]
    [InlineData(FpsOverlay.BottomRight, 3840 - 24 - 300, 2160 - 24 - 90)]
    public void Corners_keep_a_dpi_scaled_margin(string position, int x, int y) =>
        Assert.Equal((x, y), OverlayPlacement.Place(Uhd, 300, 90, position, 1.5)); // 4K a 150%: margem de 16 · 1,5

    [Fact]
    public void Second_monitor_to_the_left_uses_its_own_coordinates()
    {
        var left = new PixelRect(-2560, 0, 0, 1440);
        Assert.Equal((-16 - 200, 16), OverlayPlacement.Place(left, 200, 50, FpsOverlay.TopRight, 1));
        Assert.Equal((-2560 + 16, 16), OverlayPlacement.Place(left, 200, 50, "rtss", 1));
    }

    [Fact]
    public void A_window_bigger_than_the_screen_stays_on_it() =>
        Assert.Equal((0, 0), OverlayPlacement.Place(new PixelRect(0, 0, 100, 100), 500, 500, FpsOverlay.BottomRight, 1));
}
