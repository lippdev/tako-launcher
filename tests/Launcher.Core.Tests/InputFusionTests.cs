using Launcher.Core.Input;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>
/// Cadeia do "↑" de ponta a ponta: fontes juntas (<see cref="GamepadFusion"/>), interpretador (borda e repetição) e o
/// filtro do eco do teclado da Steam (<see cref="InputEchoFilter"/>).
/// </summary>
public sealed class InputFusionTests
{
    private static readonly GamepadSnapshot Idle = new(GamepadButtons.None, 0, 0);
    private static readonly GamepadSnapshot Up = new(GamepadButtons.DPadUp, 0, 0);

    private static TimeSpan Ms(double ms) => TimeSpan.FromMilliseconds(ms);

    [Fact]
    public void Short_tap_up_fires_once()
    {
        var pad = new GamepadInterpreter();

        Assert.Empty(pad.Update(Idle, Ms(0)));
        Assert.Equal([AppCommand.Up], pad.Update(Up, Ms(16)));
        Assert.Empty(pad.Update(Idle, Ms(80)));
        Assert.Equal([AppCommand.Up], pad.Update(Up, Ms(120)));
    }

    [Fact]
    public void Holding_up_repeats_like_the_other_directions()
    {
        var pad = new GamepadInterpreter();
        var fired = new List<AppCommand>();
        for (int t = 0; t <= 1000; t += 16)
        {
            fired.AddRange(pad.Update(Up, Ms(t)));
        }

        // 1 na borda + repetições a partir de 380 ms a cada 110 ms (até 1 s).
        Assert.Equal(7, fired.Count(c => c == AppCommand.Up));
        Assert.Contains(AppCommand.Up, pad.Pressed);
    }

    [Fact]
    public void Stick_resting_slightly_down_does_not_cancel_dpad_up()
    {
        var pad = new GamepadInterpreter();

        Assert.Equal([AppCommand.Up], pad.Update(new GamepadSnapshot(GamepadButtons.DPadUp, 0.05, -0.3), Ms(0)));
    }

    [Fact]
    public void Idle_xinput_does_not_hide_the_playstation_controller()
    {
        // Antes, qualquer XInput conectado (ex.: o virtual da Steam ou do DS4Windows, em repouso) ganhava do HID.
        var merged = GamepadFusion.Merge(Idle, Up);

        Assert.Equal(GamepadButtons.DPadUp, merged?.Buttons);
    }

    [Fact]
    public void Release_on_one_source_does_not_cancel_press_on_the_other()
    {
        var pad = new GamepadInterpreter();
        var fired = new List<AppCommand>();

        // O mesmo controle visto por HID e por um XInput virtual que chega um quadro depois e solta um quadro antes.
        fired.AddRange(pad.Update(GamepadFusion.Merge(Up, null)!.Value, Ms(0)));
        fired.AddRange(pad.Update(GamepadFusion.Merge(Up, Up)!.Value, Ms(16)));
        fired.AddRange(pad.Update(GamepadFusion.Merge(Up, Idle)!.Value, Ms(32)));
        fired.AddRange(pad.Update(GamepadFusion.Merge(Up, Up)!.Value, Ms(48)));
        fired.AddRange(pad.Update(GamepadFusion.Merge(Idle, Idle)!.Value, Ms(64)));

        Assert.Equal([AppCommand.Up], fired);
    }

    [Fact]
    public void Merge_keeps_the_more_tilted_stick_per_axis()
    {
        var merged = GamepadFusion.Merge(new GamepadSnapshot(GamepadButtons.A, 0.9, -0.05), new GamepadSnapshot(GamepadButtons.B, 0.1, 0.8))!.Value;

        Assert.Equal(GamepadButtons.A | GamepadButtons.B, merged.Buttons);
        Assert.Equal(0.9, merged.LeftX, 3);
        Assert.Equal(0.8, merged.LeftY, 3);
    }

    [Fact]
    public void Merge_of_nothing_is_no_controller()
    {
        Assert.Null(GamepadFusion.Merge(null, null));
        Assert.Null(GamepadFusion.Merge(Array.Empty<GamepadSnapshot?>()));
    }

    [Fact]
    public void Steam_keyboard_echo_after_the_gamepad_is_dropped()
    {
        var echo = new InputEchoFilter();
        var held = new HashSet<AppCommand> { AppCommand.Up };

        Assert.True(echo.AcceptGamepad(AppCommand.Up, Ms(0)));
        Assert.False(echo.AcceptKeyboard(AppCommand.Up, Ms(10), held));

        // Segurando: a repetição do teclado do Windows (eco da Steam) não atropela a do controle.
        Assert.False(echo.AcceptKeyboard(AppCommand.Up, Ms(600), held));
        Assert.True(echo.AcceptGamepad(AppCommand.Up, Ms(380)));
    }

    [Fact]
    public void Steam_keyboard_echo_before_the_gamepad_wins_once()
    {
        var echo = new InputEchoFilter();

        Assert.True(echo.AcceptKeyboard(AppCommand.Up, Ms(0), new HashSet<AppCommand>()));
        Assert.False(echo.AcceptGamepad(AppCommand.Up, Ms(16)));

        // A repetição do controle ao segurar continua valendo.
        Assert.True(echo.AcceptGamepad(AppCommand.Up, Ms(396)));
    }

    [Fact]
    public void Real_keyboard_and_other_commands_pass()
    {
        var echo = new InputEchoFilter();
        var none = new HashSet<AppCommand>();

        Assert.True(echo.AcceptGamepad(AppCommand.Up, Ms(0)));
        Assert.True(echo.AcceptKeyboard(AppCommand.Down, Ms(10), none));
        Assert.True(echo.AcceptKeyboard(AppCommand.Up, Ms(400), none));
        Assert.True(echo.AcceptKeyboard(AppCommand.Up, Ms(800), none));
    }

    [Fact]
    public void Reset_clears_the_held_set()
    {
        var pad = new GamepadInterpreter();
        pad.Update(Up, Ms(0));
        pad.Reset();

        Assert.Empty(pad.Pressed);
    }
}
