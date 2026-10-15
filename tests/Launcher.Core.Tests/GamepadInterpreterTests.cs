using Launcher.Core.Input;
using Xunit;

namespace Launcher.Core.Tests;

public class GamepadInterpreterTests
{
    private static GamepadSnapshot Pressed(GamepadButtons buttons) => new(buttons, 0, 0);

    [Fact]
    public void Buttons_fire_once_per_press()
    {
        var pad = new GamepadInterpreter();

        Assert.Equal([AppCommand.Accept], pad.Update(Pressed(GamepadButtons.A), TimeSpan.Zero));
        Assert.Empty(pad.Update(Pressed(GamepadButtons.A), TimeSpan.FromSeconds(2)));
        Assert.Empty(pad.Update(Pressed(GamepadButtons.None), TimeSpan.FromSeconds(3)));
        Assert.Equal([AppCommand.Accept], pad.Update(Pressed(GamepadButtons.A), TimeSpan.FromSeconds(4)));
    }

    [Fact]
    public void Held_direction_repeats_after_delay()
    {
        var pad = new GamepadInterpreter();
        var t = TimeSpan.Zero;

        Assert.Single(pad.Update(Pressed(GamepadButtons.DPadRight), t));
        Assert.Empty(pad.Update(Pressed(GamepadButtons.DPadRight), t + TimeSpan.FromMilliseconds(300)));
        Assert.Single(pad.Update(Pressed(GamepadButtons.DPadRight), t + GamepadInterpreter.RepeatDelay));
        Assert.Single(pad.Update(Pressed(GamepadButtons.DPadRight), t + GamepadInterpreter.RepeatDelay + GamepadInterpreter.RepeatInterval));
    }

    [Theory]
    [InlineData(0.9, 0, AppCommand.Right)]
    [InlineData(-0.9, 0, AppCommand.Left)]
    [InlineData(0, 0.9, AppCommand.Up)]
    [InlineData(0, -0.9, AppCommand.Down)]
    public void Left_stick_beyond_dead_zone_navigates(double x, double y, AppCommand expected) =>
        Assert.Equal([expected], new GamepadInterpreter().Update(new GamepadSnapshot(GamepadButtons.None, x, y), TimeSpan.Zero));

    [Fact]
    public void Small_stick_movement_is_ignored() =>
        Assert.Empty(new GamepadInterpreter().Update(new GamepadSnapshot(GamepadButtons.None, 0.3, -0.3), TimeSpan.Zero));

    [Fact]
    public void View_button_works_as_guide() =>
        Assert.Equal([AppCommand.Guide], new GamepadInterpreter().Update(Pressed(GamepadButtons.View), TimeSpan.Zero));
}
