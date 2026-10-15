using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Simulation;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

public sealed class GameScreenPlannerTests
{
    private static readonly ConnectedDisplay Desk = new("D", "Mesa", true, true, new ScreenBounds(0, 0, 2560, 1440), new DisplayMode(2560, 1440, 144), HdrStatus.Unsupported);
    private static readonly ConnectedDisplay Side = new("S", "Lateral", true, false, new ScreenBounds(2560, 0, 1920, 1080), new DisplayMode(1920, 1080, 60), HdrStatus.Off);
    private static readonly ConnectedDisplay TvOff = new("T", "TV", false, false, null, null, HdrStatus.Unknown);

    [Theory]
    [InlineData("3840x2160@120", 3840, 2160, 120)]
    [InlineData("1920X1080@60", 1920, 1080, 60)]
    [InlineData(" 2560×1440@144 ", 2560, 1440, 144)]
    public void Mode_key_round_trips(string key, int width, int height, int hz)
    {
        var mode = DisplayMode.TryParse(key);

        Assert.Equal(new DisplayMode(width, height, hz), mode);
        Assert.Equal(mode, DisplayMode.TryParse(mode!.Key));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("current")]
    [InlineData("1920x1080")]
    [InlineData("0x1080@60")]
    [InlineData("-1x1080@60")]
    [InlineData("axb@c")]
    public void Invalid_mode_keys_are_refused(string? key) => Assert.Null(DisplayMode.TryParse(key));

    [Fact]
    public void Modes_are_listed_largest_first_without_repeats()
    {
        var modes = DisplayMode.Normalize(
        [
            new(1920, 1080, 60), new(3840, 2160, 60), new(3840, 2160, 120), new(1920, 1080, 60), new(0, 0, 60),
        ]);

        Assert.Equal([new(3840, 2160, 120), new(3840, 2160, 60), new(1920, 1080, 60)], modes);
    }

    [Fact]
    public void Setup_key_changes_with_screen_or_mode()
    {
        var current = new GameScreenSettings("T", HideOthersMode.Disconnect, null, EnableHdr: false);
        var mode = current with { Mode = new DisplayMode(3840, 2160, 120) };

        Assert.Equal("T|current", current.SetupKey);
        Assert.Equal("T|3840x2160@120", mode.SetupKey);
        Assert.False(current.NeedsConfirmation("t|CURRENT"));
        Assert.True(mode.NeedsConfirmation("T|current"));
        Assert.True(current.NeedsConfirmation(null));
        // HDR e o jeito de esconder as outras telas não mudam o que aparece na tela de jogo.
        Assert.False((current with { EnableHdr = true, HideOthers = HideOthersMode.BlackCurtain }).NeedsConfirmation(current.SetupKey));
    }

    [Fact]
    public void Inactive_tv_is_turned_on_and_every_other_active_screen_is_hidden()
    {
        var plan = GameScreenPlanner.Plan(new GameScreenSettings("T", HideOthersMode.Disconnect, null, false), [Desk, Side, TvOff]);

        Assert.True(plan.CanApply);
        Assert.True(plan.EnableTarget);
        Assert.Equal(["D", "S"], plan.HideIds);
        Assert.Null(plan.Mode);
        Assert.False(plan.TurnOnHdr);
    }

    [Fact]
    public void Screen_that_is_not_connected_is_a_problem_and_changes_nothing()
    {
        var plan = GameScreenPlanner.Plan(new GameScreenSettings("gone", HideOthersMode.Disconnect, null, true), [Desk, Side]);

        Assert.False(plan.CanApply);
        Assert.NotNull(plan.Problem);
        Assert.Empty(plan.HideIds);
    }

    [Fact]
    public void Mode_is_applied_only_when_different()
    {
        var same = GameScreenPlanner.Plan(new GameScreenSettings("S", HideOthersMode.Disconnect, new DisplayMode(1920, 1080, 60), false), [Desk, Side]);
        var other = GameScreenPlanner.Plan(new GameScreenSettings("S", HideOthersMode.Disconnect, new DisplayMode(1280, 720, 60), false), [Desk, Side]);
        var asleep = GameScreenPlanner.Plan(new GameScreenSettings("T", HideOthersMode.Disconnect, new DisplayMode(3840, 2160, 120), false), [Desk, TvOff]);

        Assert.Null(same.Mode);
        Assert.Equal(new DisplayMode(1280, 720, 60), other.Mode);
        Assert.Equal(new DisplayMode(3840, 2160, 120), asleep.Mode);
        Assert.Equal(["D"], same.HideIds);
    }

    [Fact]
    public void Hdr_is_turned_on_only_where_it_can_be()
    {
        var off = GameScreenPlanner.Plan(new GameScreenSettings("S", HideOthersMode.Disconnect, null, true), [Desk, Side]);
        var unsupported = GameScreenPlanner.Plan(new GameScreenSettings("D", HideOthersMode.Disconnect, null, true), [Desk, Side]);
        var alreadyOn = GameScreenPlanner.Plan(new GameScreenSettings("S", HideOthersMode.Disconnect, null, true), [Desk, Side with { Hdr = HdrStatus.On }]);
        var asleep = GameScreenPlanner.Plan(new GameScreenSettings("T", HideOthersMode.Disconnect, null, true), [Desk, TvOff]);

        Assert.True(off.TurnOnHdr);
        Assert.False(unsupported.TurnOnHdr);
        Assert.Single(unsupported.Notes);
        Assert.False(alreadyOn.TurnOnHdr);
        Assert.True(asleep.TurnOnHdr); // decide depois de ligar a TV
    }

    [Fact]
    public void Simulated_desk_turns_screens_on_and_off_and_moves_the_primary()
    {
        var desk = new SimulatedDisplayDesk();

        desk.Enable(["SIM\\TV"]);
        desk.SetPrimary("SIM\\TV");
        desk.Disable(["SIM\\DESK"]);

        var displays = desk.ListDisplays();
        Assert.True(displays.Single(d => d.Id == "SIM\\TV") is { IsActive: true, IsPrimary: true });
        Assert.False(displays.Single(d => d.Id == "SIM\\DESK").IsActive);
        Assert.Throws<RuntimeEnvironmentException>(() => desk.Disable(["SIM\\TV"]));
        Assert.Single(desk.Capture().Monitors);
    }
}
