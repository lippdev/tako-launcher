using Launcher.Core.Platform;
using Xunit;

namespace Launcher.Core.Tests;

public class PlatformTests
{
    [Fact]
    public void Handheld_with_24H2_and_xbox_app_is_supported() =>
        Assert.Equal(FseSupport.Supported, FseClassifier.Classify(new FseSignals(26100, true, FseClassifier.HandheldDeviceForm)).Support);

    [Theory]
    [InlineData(19045)] // Windows 10 22H2
    [InlineData(22631)] // Windows 11 23H2
    public void Older_windows_is_unavailable(int build)
    {
        var result = FseClassifier.Classify(new FseSignals(build, true, FseClassifier.HandheldDeviceForm));

        Assert.Equal(FseSupport.Unavailable, result.Support);
        Assert.Contains(build.ToString(System.Globalization.CultureInfo.InvariantCulture), result.Reason, StringComparison.Ordinal);
    }

    [Fact]
    public void Missing_xbox_app_is_unavailable() =>
        Assert.Equal(FseSupport.Unavailable, FseClassifier.Classify(new FseSignals(26200, false, FseClassifier.HandheldDeviceForm)).Support);

    [Theory]
    [InlineData(null)] // sem DeviceForm (desktop comum)
    [InlineData(3)] // notebook
    public void Desktop_or_laptop_is_never_reported_as_supported(int? deviceForm) =>
        Assert.Equal(FseSupport.Indeterminate, FseClassifier.Classify(new FseSignals(26200, true, deviceForm)).Support);

    [Fact]
    public void Unreadable_signals_are_indeterminate()
    {
        Assert.Equal(FseSupport.Indeterminate, FseClassifier.Classify(new FseSignals(null, true, FseClassifier.HandheldDeviceForm)).Support);
        Assert.Equal(FseSupport.Indeterminate, FseClassifier.Classify(new FseSignals(26100, null, FseClassifier.HandheldDeviceForm)).Support);
    }

    [Fact]
    public void Startup_command_quotes_the_executable() =>
        Assert.Equal("\"C:\\Program Files\\Tako\\Tako.exe\"", StartupCommand.For(@"C:\Program Files\Tako\Tako.exe"));

    [Theory]
    [InlineData("\"C:\\Users\\Ana\\AppData\\Local\\Programs\\Tako\\Tako.exe\"", true)]
    [InlineData("\"c:\\users\\ana\\appdata\\local\\programs\\tako\\TAKO.EXE\" --fullscreen", true)]
    [InlineData("C:\\Users\\Ana\\AppData\\Local\\Programs\\Tako\\Tako.exe", true)]
    [InlineData("\"D:\\Portatil\\Tako.exe\"", false)]
    [InlineData("", false)]
    [InlineData(null, false)]
    public void Startup_entry_counts_only_when_it_points_to_this_executable(string? command, bool expected) =>
        Assert.Equal(expected, StartupCommand.PointsTo(command, @"C:\Users\Ana\AppData\Local\Programs\Tako\Tako.exe"));
}
