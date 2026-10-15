using ConsoleMode.Runtime.Display;
using ConsoleMode.Runtime.Simulation;
using Launcher.Core.Session.GameScreen;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace Launcher.Core.Tests;

public sealed class GameScreenSessionTests
{
    private const string Desk = "SIM\\DESK";
    private const string Tv = "SIM\\TV";

    private readonly SimulatedDisplayDesk _desk = new();
    private readonly SimulatedScreenCurtain _curtain = new();

    [Fact]
    public void Tv_is_turned_on_made_primary_and_the_desk_is_disconnected_then_everything_comes_back()
    {
        var session = Session();
        var before = _desk.Capture();

        var result = session.Begin(new GameScreenSettings(Tv, HideOthersMode.Disconnect, new DisplayMode(3840, 2160, 120), EnableHdr: true), confirmedSetup: null);

        Assert.Equal(GameScreenOutcome.Applied, result.Outcome);
        Assert.True(result.NeedsConfirmation);
        Assert.Equal(new ScreenBounds(0, 0, 3840, 2160), result.TargetBounds);
        var during = _desk.ListDisplays();
        Assert.True(during.Single(d => d.Id == Tv) is { IsActive: true, IsPrimary: true, Hdr: HdrStatus.On, CurrentMode.RefreshHz: 120 });
        Assert.False(during.Single(d => d.Id == Desk).IsActive);
        Assert.True(session.IsActive);

        Assert.True(session.End());

        Assert.True(_desk.Capture().IsEquivalentTo(before));
        Assert.Contains($"hdr {Tv} off", _desk.Calls);
        Assert.False(session.IsActive);
    }

    [Fact]
    public void Calls_follow_the_console_mode_order()
    {
        Session().Begin(new GameScreenSettings(Tv, HideOthersMode.Disconnect, null, EnableHdr: false), null);

        Assert.Equal([$"enable {Tv}", $"primary {Tv}", $"disable {Desk}", $"primary {Tv}"], _desk.Calls);
    }

    [Fact]
    public void Confirmed_setup_is_not_asked_again()
    {
        var settings = new GameScreenSettings(Tv, HideOthersMode.Disconnect, null, false);

        var result = Session().Begin(settings, settings.SetupKey);

        Assert.False(result.NeedsConfirmation);
    }

    [Fact]
    public void Black_curtain_covers_the_other_screens_without_disconnecting_them()
    {
        var session = Session();

        var result = session.Begin(new GameScreenSettings(Tv, HideOthersMode.BlackCurtain, null, false), null);

        Assert.Equal(GameScreenOutcome.Applied, result.Outcome);
        Assert.True(_desk.ListDisplays().Single(d => d.Id == Desk).IsActive);
        Assert.Equal([new ScreenBounds(-2560, 0, 2560, 1440)], _curtain.Covered);

        session.End();

        Assert.Empty(_curtain.Covered);
    }

    [Fact]
    public void Tv_that_does_not_wake_puts_everything_back_and_never_hides_the_desk()
    {
        _desk.NeverWake.Add(Tv);
        var before = _desk.Capture();
        var session = Session();

        var result = session.Begin(new GameScreenSettings(Tv, HideOthersMode.Disconnect, null, false), null);

        Assert.Equal(GameScreenOutcome.Reverted, result.Outcome);
        Assert.DoesNotContain(_desk.Calls, c => c.StartsWith("disable", StringComparison.Ordinal));
        Assert.Equal(2, _desk.Calls.Count(c => c.StartsWith("enable", StringComparison.Ordinal)));
        Assert.True(_desk.Capture().IsEquivalentTo(before));
        Assert.False(session.IsActive);
    }

    [Fact]
    public void Missing_screen_changes_nothing()
    {
        var result = Session().Begin(new GameScreenSettings("SIM\\GONE", HideOthersMode.Disconnect, null, true), null);

        Assert.Equal(GameScreenOutcome.Skipped, result.Outcome);
        Assert.Empty(_desk.Calls);
    }

    [Fact]
    public void Platform_failure_while_reading_changes_nothing()
    {
        _desk.FailNext = true;
        var session = Session();

        var result = session.Begin(new GameScreenSettings(Tv, HideOthersMode.Disconnect, null, false), null);

        Assert.Equal(GameScreenOutcome.Skipped, result.Outcome);
        Assert.Empty(session.ListDisplays());
        Assert.True(session.End());
    }

    [Fact]
    public void Refused_mode_keeps_the_screen_focused_in_its_current_mode()
    {
        var session = Session();

        var result = session.Begin(new GameScreenSettings(Tv, HideOthersMode.Disconnect, new DisplayMode(1280, 720, 30), false), null);

        Assert.Equal(GameScreenOutcome.Applied, result.Outcome);
        Assert.Equal(new DisplayMode(3840, 2160, 60), _desk.ListDisplays().Single(d => d.Id == Tv).CurrentMode);
    }

    [Fact]
    public void Already_focused_screen_with_nothing_else_on_is_left_alone()
    {
        var session = Session();

        var result = session.Begin(new GameScreenSettings(Desk, HideOthersMode.Disconnect, null, EnableHdr: true), null);

        Assert.Equal(GameScreenOutcome.Applied, result.Outcome);
        Assert.Equal([$"primary {Desk}"], _desk.Calls); // HDR sem suporte: nem tenta
        Assert.True(session.End());
        Assert.DoesNotContain("restore", _desk.Calls); // nada mudou: nenhuma restauração
    }

    private GameScreenSession Session() =>
        new(_desk, _desk, _curtain, NullLogger<GameScreenSession>.Instance, _ => { });
}
