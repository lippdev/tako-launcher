using ConsoleMode.Runtime.SessionTools;
using Xunit;

namespace ConsoleMode.Runtime.Tests.SessionTools;

public class FpsLimitsTests
{
    [Fact]
    public void No_limit_comes_first_and_has_a_plain_label()
    {
        Assert.Equal(0, FpsLimits.Options[0]);
        Assert.Equal("Sem limite", FpsLimits.Label(0));
        Assert.Equal("60 FPS", FpsLimits.Label(60));
    }

    [Theory]
    [InlineData(-5, 0)]
    [InlineData(0, 0)]
    [InlineData(60, 60)]
    [InlineData(59, 60)]
    [InlineData(100, 90)]
    [InlineData(500, 165)]
    public void Saved_values_snap_to_an_offered_limit(int saved, int expected) =>
        Assert.Equal(expected, FpsLimits.Normalize(saved));

    [Fact]
    public void Left_and_right_stop_at_the_ends()
    {
        Assert.Equal(0, FpsLimits.Step(0, -1));
        Assert.Equal(30, FpsLimits.Step(0, 1));
        Assert.Equal(45, FpsLimits.Step(60, -1));
        Assert.Equal(165, FpsLimits.Step(165, 1));
    }

    [Fact]
    public void A_cycles_back_to_no_limit_after_the_last()
    {
        Assert.Equal(30, FpsLimits.Cycle(0));
        Assert.Equal(0, FpsLimits.Cycle(165));
    }
}

public class RtssLocatorTests
{
    [Fact]
    public void The_registry_folder_comes_first_then_both_program_files_without_repeats()
    {
        string x86 = Path.Combine(Path.GetTempPath(), "Program Files (x86)");
        string x64 = Path.Combine(Path.GetTempPath(), "Program Files");
        string registry = Path.Combine(x86, "RivaTuner Statistics Server");
        var folders = RtssLocator.Folders($"\"{registry}{Path.DirectorySeparatorChar}\"", x86, x64);
        Assert.Equal([registry, Path.Combine(x64, "RivaTuner Statistics Server")], folders);
        Assert.Empty(RtssLocator.Folders(" ", null, null));
    }

    [Fact]
    public void Resolve_returns_the_folder_that_has_rtss()
    {
        string[] folders = [Path.Combine(Path.GetTempPath(), "a"), Path.Combine(Path.GetTempPath(), "b")];
        string exe = Path.Combine(folders[1], RtssLocator.ExeName);
        Assert.Equal(folders[1], RtssLocator.Resolve(folders, p => p == exe));
        Assert.Null(RtssLocator.Resolve(folders, _ => false));
    }
}
