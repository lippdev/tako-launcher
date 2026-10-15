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

    [Fact]
    public void Direction_held_during_disconnect_does_not_keep_moving()
    {
        var pad = new GamepadInterpreter();
        var t = TimeSpan.Zero;
        Assert.Single(pad.Update(Pressed(GamepadButtons.DPadRight), t));
        Assert.Single(pad.Update(Pressed(GamepadButtons.DPadRight), t + GamepadInterpreter.RepeatDelay));

        // Controle caiu no meio da repetição; volta com o direcional ainda pressionado.
        pad.Reset();
        var later = t + TimeSpan.FromSeconds(5);
        Assert.Empty(pad.Update(Pressed(GamepadButtons.DPadRight), later));
        Assert.Empty(pad.Update(Pressed(GamepadButtons.DPadRight), later + GamepadInterpreter.RepeatDelay));
        Assert.Empty(pad.Update(Pressed(GamepadButtons.DPadRight), later + TimeSpan.FromSeconds(10)));

        // Soltou e apertou de novo: volta a funcionar sem reiniciar nada.
        Assert.Empty(pad.Update(Pressed(GamepadButtons.None), later + TimeSpan.FromSeconds(11)));
        Assert.Equal([AppCommand.Right], pad.Update(Pressed(GamepadButtons.DPadRight), later + TimeSpan.FromSeconds(12)));
    }

    [Fact]
    public void Button_held_while_launcher_was_in_background_does_not_fire()
    {
        var pad = new GamepadInterpreter();
        pad.Reset();

        Assert.Empty(pad.Update(Pressed(GamepadButtons.A), TimeSpan.Zero));
        // Um botão novo funciona na hora; só o que já estava pressionado fica ignorado.
        Assert.Equal([AppCommand.Back], pad.Update(Pressed(GamepadButtons.A | GamepadButtons.B), TimeSpan.FromMilliseconds(16)));
        Assert.Empty(pad.Update(Pressed(GamepadButtons.None), TimeSpan.FromMilliseconds(32)));
        Assert.Equal([AppCommand.Accept], pad.Update(Pressed(GamepadButtons.A), TimeSpan.FromMilliseconds(48)));
    }

    [Fact]
    public void Reconnected_controller_with_nothing_pressed_works_immediately()
    {
        var pad = new GamepadInterpreter();
        Assert.Single(pad.Update(Pressed(GamepadButtons.DPadDown), TimeSpan.Zero));

        pad.Reset();

        Assert.Empty(pad.Update(Pressed(GamepadButtons.None), TimeSpan.FromSeconds(1)));
        Assert.Equal([AppCommand.Down], pad.Update(Pressed(GamepadButtons.DPadDown), TimeSpan.FromSeconds(1.016)));
    }
}
