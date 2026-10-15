using ConsoleMode.Runtime.Audio;
using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Simulation;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

public class EnvironmentContractTests
{
    [Theory]
    [InlineData(60000u, 1001u, 59940u, 1000u, true)]
    [InlineData(60u, 1u, 60000u, 1000u, true)]
    [InlineData(60u, 1u, 120u, 1u, false)]
    [InlineData(144u, 1u, 0u, 0u, false)]
    public void Refresh_rates_compare_by_value(uint n1, uint d1, uint n2, uint d2, bool same) =>
        Assert.Equal(same, new RefreshRate(n1, d1).IsSameRate(new RefreshRate(n2, d2)));

    [Fact]
    public void Topology_equivalence_ignores_order_names_and_platform_data()
    {
        var a = new MonitorState("A", "Monitor A", true, 0, 0, 1920, 1080, RefreshRate.FromHz(60), DisplayRotation.None);
        var b = new MonitorState("B", "Monitor B", false, 1920, 0, 1920, 1080, RefreshRate.FromHz(60), DisplayRotation.None);

        Assert.True(new DisplayTopology([a, b], "x").IsEquivalentTo(new DisplayTopology([b with { Name = "Outro" }, a])));
        Assert.False(new DisplayTopology([a, b]).IsEquivalentTo(new DisplayTopology([a])));
        Assert.False(new DisplayTopology([a]).IsEquivalentTo(new DisplayTopology([a with { Rotation = DisplayRotation.Rotate180 }])));
    }

    [Fact]
    public void Audio_equivalence_compares_defaults_and_levels_of_devices_present_on_both_sides()
    {
        var state = SimulatedAudioEndpoints.Default;

        Assert.True(state.IsEquivalentTo(state with { Levels = [state.Levels[0]] }));
        Assert.False(state.IsEquivalentTo(state with { Levels = [state.Levels[0] with { Muted = true }] }));
        Assert.False(state.IsEquivalentTo(state with { Defaults = [new AudioDefaultDevice(AudioRole.Console, "sim-headset")] }));
    }

    [Fact]
    public void Simulated_services_change_only_memory()
    {
        var display = new SimulatedDisplayConfiguration();
        var audio = new SimulatedAudioEndpoints();
        var target = new DisplayTopology([SimulatedDisplayConfiguration.Default.Monitors[0] with { Width = 1280, Height = 720 }], "opaco");

        display.Apply(target, ["SIM\\MONITOR1"]);
        audio.SetDefaultDevice("sim-headset", AudioRole.Multimedia);
        audio.SetVolume("sim-headset", 0.25f);

        Assert.Equal(1280, display.Capture().Monitors[0].Width);
        Assert.Null(display.Capture().PlatformData);
        Assert.Equal("sim-headset", audio.Capture().DefaultFor(AudioRole.Multimedia));
        Assert.Equal(0.25f, audio.Capture().LevelOf("sim-headset")!.Volume);
        Assert.Throws<RuntimeEnvironmentException>(() => audio.SetDefaultDevice("desconectado", AudioRole.Console));
    }
}
