// Adaptado de lippdev/consolemode: tests/ConsoleMode.Tests/SteamShutdownTests.cs
using ConsoleMode.Runtime.Steam;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

public class SteamClientRulesTests
{
    [Theory]
    [InlineData("steam", "Steam", "Chrome_WidgetWin_1", true, false, true)]
    [InlineData("steamwebhelper", "Steam", "vguiPopupWindow", true, false, true)]
    [InlineData("steam", "Steam", "SDL_app", true, false, true)]
    [InlineData("game", "Steam", "Chrome_WidgetWin_1", true, false, false)]
    [InlineData("steam", "Jogo", "Chrome_WidgetWin_1", true, false, false)]
    [InlineData("steam", "Big Picture", "SDL_app", true, false, false)]
    [InlineData("steam", "Steam", "Chrome_WidgetWin_1", false, false, false)]
    [InlineData("steam", "Steam", "Chrome_WidgetWin_1", true, true, false)]
    public void Only_unowned_visible_steam_client_windows_are_close_candidates(string process, string title, string className, bool visible, bool owned, bool expected) =>
        Assert.Equal(expected, SteamClientRules.IsClientWindowCandidate(process, title, className, visible, owned));

    [Theory]
    [InlineData("steam", true)]
    [InlineData("SteamWebHelper", true)]
    [InlineData("cs2", false)]
    [InlineData("steamapp", false)]
    [InlineData(null, false)]
    public void Only_steams_own_processes_may_be_asked_to_close(string? process, bool expected) =>
        Assert.Equal(expected, SteamClientRules.IsSteamProcess(process));

    [Fact]
    public void Steam_goes_to_the_tray_after_a_steam_game_when_no_other_game_runs() =>
        Assert.Equal(SteamTrayDecision.CloseToTray, SteamClientRules.DecideTray(steamGame: true, settingOn: true, steamRunning: true, runningAppId: 0));

    [Fact]
    public void Steam_is_left_alone_after_games_from_other_stores() =>
        Assert.Equal(SteamTrayDecision.NotSteamGame, SteamClientRules.DecideTray(steamGame: false, settingOn: true, steamRunning: true, runningAppId: 0));

    [Fact]
    public void The_setting_turns_it_off_and_wins_over_everything_else() =>
        Assert.Equal(SteamTrayDecision.SettingOff, SteamClientRules.DecideTray(true, settingOn: false, steamRunning: false, runningAppId: 570));

    [Fact]
    public void A_steam_that_is_not_running_is_not_touched() =>
        Assert.Equal(SteamTrayDecision.NotRunning, SteamClientRules.DecideTray(true, true, steamRunning: false, runningAppId: 0));

    [Theory]
    [InlineData(570, SteamTrayDecision.GameRunning)]
    [InlineData(null, SteamTrayDecision.GameStateUnknown)]
    public void Another_game_or_an_unknown_state_keeps_steam_open(int? appId, SteamTrayDecision expected) =>
        Assert.Equal(expected, SteamClientRules.DecideTray(true, true, true, appId));

    [Theory]
    [InlineData(1245620, 1245620)]
    [InlineData(0, 0)]
    [InlineData(-5, null)]
    [InlineData(570u, 570)]
    [InlineData(4000000000u, null)] // não cabe num int: estado desconhecido
    [InlineData("730", 730)]
    [InlineData("-1", null)]
    [InlineData("abc", null)]
    [InlineData("", null)]
    [InlineData(null, null)]
    public void The_running_game_id_is_read_from_whatever_the_registry_holds(object? value, int? expected) =>
        Assert.Equal(expected, SteamClientRules.ParseRunningAppId(value));
}
